use bevy::prelude::Vec3;
use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use log::info;

use crate::animations::{smoothstep, Fall, JumpSample, JumpSegment, Spring, Squat, TwoFootedJump};
use crate::rigs::mix::blend_clips;
use crate::rigs::segment_debug::segment_debug_enabled;
use crate::{Animation, Effects};

impl JumpSample {
	/// Shared squat / spring / airborne / land routing. `airborne` is Fall or Tuck.
	pub(crate) fn apply_pose<A: Animation<HumanoidV0Rig>>(
		&self,
		rig: &mut HumanoidV0Rig,
		airborne: &A,
		airborne_progress: f32,
		land_from_progress: f32,
	) {
		match self.segment {
			JumpSegment::Squat => {
				self.windup.apply_for(rig, self.pose_progress);
			}
			JumpSegment::Spring => {
				blend_clips(
					rig,
					&Squat::for_loop(1.0, 1.0),
					0.0,
					&Spring::default(),
					self.local,
					smoothstep(self.blend_progress),
				);
			}
			JumpSegment::Fall => {
				if self.blend_progress < 1.0 {
					blend_clips(
						rig,
						&Spring::default(),
						1.0,
						airborne,
						airborne_progress,
						smoothstep(self.blend_progress),
					);
				} else {
					airborne.apply_for(rig, airborne_progress);
				}
			}
			JumpSegment::Land => {
				if self.blend_progress < 1.0 {
					blend_clips(
						rig,
						airborne,
						land_from_progress,
						&self.landing,
						self.pose_progress,
						smoothstep(self.blend_progress),
					);
				} else {
					self.landing.apply_for(rig, self.pose_progress);
				}
			}
		}
	}
}

impl TwoFootedJump {
	fn apply_sampled(&self, rig: &mut HumanoidV0Rig, elapsed: f32, sample: &JumpSample) {
		if segment_debug_enabled() {
			self.log_sampled_debug(rig, elapsed, sample);
		}
		sample.apply_pose(rig, &Fall::default(), sample.local, 1.0);
	}

	fn effects_from_sample(
		&self,
		lengths: character_rigs::humanoid::LegSegmentLengths,
		sample: &JumpSample,
	) -> Effects {
		let y = sample.vertical_offset(self, lengths);
		if y.abs() > f32::EPSILON {
			ArmatureOffset::from_translation(Vec3::new(0.0, y, 0.0))
		} else {
			ArmatureOffset::IDENTITY
		}
	}

	fn log_sampled_debug(&self, rig: &HumanoidV0Rig, elapsed: f32, sample: &JumpSample) {
		match sample.segment {
			JumpSegment::Fall if sample.local > 0.9 => {
				let fall = Fall::default();
				info!(
					"jump fall end: elapsed={:.3} cycle_t={:.3} air_end={:.3} fall_local={:.4} fall_femur=0 fall_shoulder_flex={:.4}",
					elapsed,
					sample.cycle_t,
					sample.timings.air_end(),
					sample.local,
					fall.shoulder_flex(character_rigs::Side::Left, sample.local),
				);
			}
			JumpSegment::Land if sample.local < sample.timings.land_descent_duration + 0.05 => {
				info!(
					"jump land start: elapsed={:.3} cycle_t={:.3} land_local={:.4} land_depth={:.4} land_femur={:.4} transition={:.4} land_desc_d={:.4} y={:.4}",
					elapsed,
					sample.cycle_t,
					sample.local,
					sample.landing.depth(sample.pose_progress),
					sample.landing.femur_swing(sample.pose_progress),
					sample.blend_progress,
					sample.timings.land_descent_duration,
					sample.vertical_offset(self, rig.segment_lengths),
				);
			}
			_ => {}
		}
	}
}

impl Animation<HumanoidV0Rig> for TwoFootedJump {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, elapsed: f32) {
		let sample = self.sample(rig.segment_lengths, elapsed);
		self.apply_sampled(rig, elapsed, &sample);
	}

	fn effects_for(&self, rig: &HumanoidV0Rig, elapsed: f32) -> Effects {
		let sample = self.sample(rig.segment_lengths, elapsed);
		self.effects_from_sample(rig.segment_lengths, &sample)
	}

	fn apply(&self, rig: &mut HumanoidV0Rig, elapsed: f32) -> Effects {
		let sample = self.sample(rig.segment_lengths, elapsed);
		self.apply_sampled(rig, elapsed, &sample);
		self.effects_from_sample(rig.segment_lengths, &sample)
	}
}

impl TwoFootedJump {
	pub fn log_landing_debug(&self, rig: &HumanoidV0Rig, elapsed: f32, label: &str) {
		let lengths = rig.segment_lengths;
		let sample = self.sample(lengths, elapsed);
		let y = sample.vertical_offset(self, lengths);

		info!(
			"{label}: elapsed={:.3} cycle_t={:.3} segment={:?} local={:.4} land_depth={:.4} y={:.4} timings[squat=({:.3},{:.3}) spring={:.3} air={:.3} land=({:.4},{:.3})] speeds[pre={:.3} landing={:.3}]",
			elapsed,
			sample.cycle_t,
			sample.segment,
			sample.local,
			sample.landing_depth(),
			y,
			sample.timings.squat_descent_duration,
			sample.timings.squat_ascent_duration,
			sample.timings.spring_duration,
			sample.timings.air_duration,
			sample.timings.land_descent_duration,
			sample.timings.land_ascent_duration,
			self.pre_squat_speed,
			self.landing_squat_speed,
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::DEFAULT_SPRING_DURATION;

	fn default_jump() -> TwoFootedJump {
		TwoFootedJump::default()
	}

	#[test]
	fn spring_end_legs_straight() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig.segment_lengths;
		let elapsed = jump.timings(lengths).squat_end() + DEFAULT_SPRING_DURATION * 0.99;
		jump.apply(&mut rig, elapsed);

		assert!(rig.posed_angle("femur.L") < 0.05, "femur should be straight");
		assert!(rig.posed_angle("shin.L") < 0.05, "shin should be straight");
		Ok(())
	}

	#[test]
	fn land_starts_compression_after_touchdown() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig.segment_lengths;
		let timings = jump.timings(lengths);
		jump.apply(&mut rig, timings.air_end() + timings.land_descent_duration * 0.25);

		assert!(rig.posed_angle("femur.L") > 0.01, "land should start folding");
		Ok(())
	}

	#[test]
	fn land_peak_below_full_squat() -> anyhow::Result<()> {
		let mut rig_squat = HumanoidV0Rig::imported();
		Squat::for_loop(1.0, 1.0).apply(&mut rig_squat, 0.5);
		let squat_femur = rig_squat.posed_angle("femur.L");

		let mut rig_land = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig_land.segment_lengths;
		let timings = jump.timings(lengths);
		jump.apply(&mut rig_land, timings.air_end() + timings.land_descent_duration * 0.99);
		let land_femur = rig_land.posed_angle("femur.L");

		assert!(land_femur < squat_femur);
		Ok(())
	}

	#[test]
	fn windup_still_drops_the_armature() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig.segment_lengths;
		let mid_windup = jump.timings(lengths).squat_descent_duration * 0.99;
		let effects = jump.apply(&mut rig, mid_windup);
		if effects.0.translation.y >= 0.0 {
			return Err(anyhow::anyhow!(
				"windup drop should be negative Y, got {}",
				effects.0.translation.y
			));
		}
		Ok(())
	}

	#[test]
	fn land_transition_blends_arms_from_fall() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig.segment_lengths;
		let timings = jump.timings(lengths);
		let blend = timings.land_pose_blend_duration();
		jump.apply(&mut rig, timings.air_end() + blend * 0.5);

		let mut fall_rig = HumanoidV0Rig::imported();
		Fall::default().apply(&mut fall_rig, 1.0);
		assert!(rig.posed_angle("shoulder.L") > 0.05, "blended shoulder leaves rest");
		assert!(
			rig.posed_angle("shoulder.L") < fall_rig.posed_angle("shoulder.L"),
			"blend is short of the full fall spread"
		);
		Ok(())
	}

	fn assert_pose_near(a: &HumanoidV0Rig, b: &HumanoidV0Rig, label: &str) {
		for name in a.animation_bone_names() {
			assert!(a.rotation(name).dot(b.rotation(name)).abs() > 1.0 - 1e-5, "{label}: {name}");
		}
	}

	/// Main’s split apply_for / effects_for before the shared sample context.
	fn legacy_apply_for(jump: &TwoFootedJump, rig: &mut HumanoidV0Rig, elapsed: f32) {
		use crate::animations::FALL_BLEND_FRACTION;
		let lengths = rig.segment_lengths;
		let (segment, local) = {
			let cycle = jump.timings(lengths).cycle_duration();
			let cycle_t = if cycle <= f32::EPSILON { 0.0 } else { elapsed % cycle };
			TwoFootedJump::segment_at_time(cycle_t, &jump.timings(lengths))
		};
		let timings = jump.timings(lengths);

		match segment {
			JumpSegment::Squat => {
				let squat = jump.prejump_squat(lengths);
				let progress = local / timings.squat_duration().max(f32::EPSILON);
				squat.apply_for(rig, progress);
			}
			JumpSegment::Spring => {
				blend_clips(
					rig,
					&Squat::for_loop(1.0, 1.0),
					0.0,
					&Spring::default(),
					local,
					smoothstep(local),
				);
			}
			JumpSegment::Fall => {
				let fall = Fall::default();
				let blend_end = FALL_BLEND_FRACTION;
				if local < blend_end {
					let transition_progress = (local / blend_end).clamp(0.0, 1.0);
					blend_clips(
						rig,
						&Spring::default(),
						1.0,
						&fall,
						local,
						smoothstep(transition_progress),
					);
				} else {
					fall.apply_for(rig, local);
				}
			}
			JumpSegment::Land => {
				let land = jump.landing_squat(lengths);
				let land_duration = timings.land_duration().max(f32::EPSILON);
				let land_progress = local / land_duration;
				let blend_window = timings.land_pose_blend_duration();
				let transition_progress = if blend_window > f32::EPSILON {
					(local / blend_window).clamp(0.0, 1.0)
				} else {
					1.0
				};
				if transition_progress < 1.0 {
					blend_clips(
						rig,
						&Fall::default(),
						1.0,
						&land,
						land_progress,
						smoothstep(transition_progress),
					);
				} else {
					land.apply_for(rig, land_progress);
				}
			}
		}
	}

	fn legacy_effects_for(jump: &TwoFootedJump, rig: &HumanoidV0Rig, elapsed: f32) -> Effects {
		let y = jump.vertical_offset(rig.segment_lengths, elapsed);
		if y.abs() > f32::EPSILON {
			ArmatureOffset::from_translation(Vec3::new(0.0, y, 0.0))
		} else {
			ArmatureOffset::IDENTITY
		}
	}

	fn sample_elapsed_grid(
		jump: &TwoFootedJump,
		lengths: character_rigs::humanoid::LegSegmentLengths,
	) -> Vec<f32> {
		let timings = jump.timings(lengths);
		let mut times = vec![
			0.0,
			timings.squat_descent_duration * 0.5,
			timings.squat_end() - 1e-4,
			timings.squat_end(),
			timings.squat_end() + 1e-4,
			timings.squat_end() + timings.spring_duration * 0.5,
			timings.spring_end() - 1e-4,
			timings.spring_end(),
			timings.spring_end() + 1e-4,
			timings.spring_end()
				+ timings.air_duration * crate::animations::FALL_BLEND_FRACTION * 0.5,
			timings.spring_end() + timings.air_duration * crate::animations::FALL_BLEND_FRACTION,
			timings.spring_end() + timings.air_duration * 0.5,
			timings.air_end() - 1e-4,
			timings.air_end(),
			timings.air_end() + 1e-4,
			timings.air_end() + timings.land_pose_blend_duration() * 0.5,
			timings.air_end() + timings.land_descent_duration * 0.25,
			timings.air_end() + timings.land_descent_duration,
			timings.cycle_duration() - 1e-3,
			timings.cycle_duration(),
			timings.cycle_duration() + 0.05,
		];
		times.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
		times
	}

	#[test]
	fn sample_matches_public_segment_and_offset() -> anyhow::Result<()> {
		let jump = default_jump();
		let lengths = HumanoidV0Rig::imported().segment_lengths;
		for elapsed in sample_elapsed_grid(&jump, lengths) {
			let sample = jump.sample(lengths, elapsed);
			let (segment, local) = jump.segment(lengths, elapsed);
			assert_eq!(sample.segment, segment, "elapsed={elapsed}");
			assert!((sample.local - local).abs() < 1e-6, "local elapsed={elapsed}");
			assert!((sample.cycle_t - jump.time_in_cycle(lengths, elapsed)).abs() < 1e-6);
			assert!(
				(sample.vertical_offset(&jump, lengths) - jump.vertical_offset(lengths, elapsed))
					.abs() < 1e-6
			);
		}
		Ok(())
	}

	#[test]
	fn jump_pose_and_effects_match_legacy_split_path() -> anyhow::Result<()> {
		let jump = default_jump();
		let lengths = HumanoidV0Rig::imported().segment_lengths;
		for elapsed in sample_elapsed_grid(&jump, lengths) {
			let mut legacy = HumanoidV0Rig::imported();
			legacy_apply_for(&jump, &mut legacy, elapsed);
			let legacy_effects = legacy_effects_for(&jump, &legacy, elapsed);

			let mut split = HumanoidV0Rig::imported();
			jump.apply_for(&mut split, elapsed);
			let split_effects = jump.effects_for(&split, elapsed);

			let mut combined = HumanoidV0Rig::imported();
			let combined_effects = jump.apply(&mut combined, elapsed);

			assert_pose_near(&legacy, &split, &format!("split elapsed={elapsed}"));
			assert_pose_near(&legacy, &combined, &format!("combined elapsed={elapsed}"));
			assert_eq!(legacy_effects, split_effects, "split effects elapsed={elapsed}");
			assert_eq!(legacy_effects, combined_effects, "combined effects elapsed={elapsed}");
		}
		Ok(())
	}

	fn proportioned_rig(femur: f32, shin: f32) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::imported();
		rig.segment_lengths = character_rigs::humanoid::LegSegmentLengths { femur, shin };
		rig
	}

	#[test]
	fn jump_matches_legacy_on_varied_proportions() -> anyhow::Result<()> {
		let jump = default_jump();
		for (femur, shin) in [(0.35, 0.4), (0.5, 0.5), (0.72, 0.31)] {
			let probe = proportioned_rig(femur, shin);
			let lengths = probe.segment_lengths;
			let elapsed =
				jump.timings(lengths).spring_end() + jump.timings(lengths).air_duration * 0.4;
			let mut legacy = proportioned_rig(femur, shin);
			legacy_apply_for(&jump, &mut legacy, elapsed);
			let mut optimized = proportioned_rig(femur, shin);
			jump.apply_for(&mut optimized, elapsed);
			assert_pose_near(&legacy, &optimized, &format!("femur={femur} shin={shin}"));
			assert_eq!(
				legacy_effects_for(&jump, &legacy, elapsed),
				jump.effects_for(&optimized, elapsed)
			);
		}
		Ok(())
	}

	fn bench_jump_paths(mode: &str) -> Vec<u128> {
		use crate::animations::TwoFootedTuckedFlip;
		use std::hint::black_box;
		use std::time::Instant;

		const FRAMES: u32 = 400;
		const CHARACTERS: u32 = 32;
		const RUNS: u32 = 7;
		let jump = TwoFootedJump::default();
		let flip = TwoFootedTuckedFlip::default();
		let mut rigs: Vec<HumanoidV0Rig> = (0..CHARACTERS)
			.map(|index| {
				proportioned_rig(
					if index % 3 == 1 {
						0.38
					} else if index % 3 == 2 {
						0.68
					} else {
						0.5
					},
					if index % 3 == 1 {
						0.44
					} else if index % 3 == 2 {
						0.33
					} else {
						0.5
					},
				)
			})
			.collect();
		let cycle = jump.cycle_duration(HumanoidV0Rig::imported().segment_lengths);

		let mut run_ns: Vec<u128> = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			for frame in 0..FRAMES {
				let frame = black_box(frame);
				for (index, rig) in rigs.iter_mut().enumerate() {
					let elapsed = black_box(
						(frame as f32 * 0.016 + index as f32 * 0.11) % cycle.max(f32::EPSILON),
					);
					let use_flip = index % 2 == 1;
					match mode {
						"legacy-pose" if use_flip => {
							// Flip legacy lives in the flip module; pose-only uses the production path
							// when measuring combined/split production entry points below.
							flip.apply_for(rig, elapsed);
						}
						"legacy-pose" => legacy_apply_for(&jump, rig, elapsed),
						"pose" if use_flip => flip.apply_for(rig, elapsed),
						"pose" => jump.apply_for(rig, elapsed),
						"effects" if use_flip => {
							black_box(flip.effects_for(rig, elapsed));
						}
						"effects" => {
							black_box(jump.effects_for(rig, elapsed));
						}
						"combined-legacy" if use_flip => {
							flip.apply_for(rig, elapsed);
							black_box(flip.effects_for(rig, elapsed));
						}
						"combined-legacy" => {
							legacy_apply_for(&jump, rig, elapsed);
							black_box(legacy_effects_for(&jump, rig, elapsed));
						}
						"combined" if use_flip => {
							black_box(flip.apply(rig, elapsed));
						}
						_ => {
							black_box(jump.apply(rig, elapsed));
						}
					}
					black_box(&rig.pose);
				}
			}
			let samples = FRAMES as u64 * CHARACTERS as u64;
			run_ns.push(start.elapsed().as_nanos() / samples as u128);
		}
		run_ns.sort_unstable();
		run_ns
	}

	fn report_jump_bench(label: &str, run_ns: &[u128]) {
		let min = *run_ns.first().expect("run");
		let median = run_ns[run_ns.len() / 2];
		let mean = run_ns.iter().sum::<u128>() / run_ns.len() as u128;
		let spread = run_ns.last().expect("run") - min;
		eprintln!(
			"jump_flip_cycle_microbench {label}: runs={runs:?} min={min} median={median} mean={mean} spread={spread} ns/sample",
			runs = run_ns,
		);
	}

	/// `cargo test -p character-animations jump_flip_cycle_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore]
	fn jump_flip_cycle_microbench() {
		eprintln!(
			"32 characters × 400 frames × 7 runs; jump+flip complete cycles; varied femur/shin; pose/effects/combined"
		);
		report_jump_bench("pose-only legacy", &bench_jump_paths("legacy-pose"));
		report_jump_bench("pose-only sample context", &bench_jump_paths("pose"));
		report_jump_bench("effects-only", &bench_jump_paths("effects"));
		report_jump_bench("combined legacy split", &bench_jump_paths("combined-legacy"));
		report_jump_bench("combined shared sample", &bench_jump_paths("combined"));
	}
}
