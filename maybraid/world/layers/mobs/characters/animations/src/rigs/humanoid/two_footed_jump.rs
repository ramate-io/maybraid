use bevy::prelude::{Transform, Vec3};
use character_rigs::{humanoid::HumanoidRig, Side};
use log::info;

use crate::animations::{
	Fall, JumpSegment, Spring, Squat, Transition, TransitionCurve, TwoFootedJump,
};
use crate::rigs::transition::capture_animation_pose;
use crate::{Animation, Effects};

fn segment_debug_enabled() -> bool {
	std::env::var("CROZON_ANIMATION_DEBUG").is_ok()
}

impl<R: HumanoidRig> Animation<R> for TwoFootedJump<R> {
	fn apply_for(&self, rig: &mut R, elapsed: f32) {
		let lengths = rig.segment_lengths();
		let (derived, sample) = self.sample(lengths, elapsed);
		let timings = derived.timings;
		let local = sample.local_progress;

		match sample.segment {
			JumpSegment::Squat => {
				let progress = local / timings.squat_duration().max(f32::EPSILON);
				derived.prejump_squat::<R>().apply_for(rig, progress);
			}
			JumpSegment::Spring => {
				let from_pose = capture_animation_pose(&Squat::<R>::for_loop(1.0, 1.0), rig, 0.0);
				let _ = Transition::from_pose(Spring::<R>::default(), from_pose)
					.with_curve(TransitionCurve::SmoothStep)
					.apply(rig, local, local);
			}
			JumpSegment::Fall => {
				let fall = Fall::<R>::default();
				if segment_debug_enabled() && local > 0.9 {
					info!(
						"jump fall end: elapsed={:.3} cycle_t={:.3} air_end={:.3} fall_local={:.4} fall_femur=0 fall_shoulder_flex={:.4}",
						elapsed,
						self.time_in_cycle(lengths, elapsed),
						timings.air_end(),
						local,
						fall.shoulder_flex(Side::Left, local),
					);
				}
				if sample.transition_weight < 1.0 {
					let from_pose = capture_animation_pose(&Spring::<R>::default(), rig, 1.0);
					let _ = Transition::from_pose(fall, from_pose)
						.with_curve(TransitionCurve::SmoothStep)
						.apply(rig, local, sample.transition_weight);
				} else {
					fall.apply_for(rig, local);
				}
			}
			JumpSegment::Land => {
				let land = derived.landing_squat::<R>();
				let land_duration = timings.land_duration().max(f32::EPSILON);
				let land_progress = local / land_duration;
				if segment_debug_enabled() && local < timings.land_descent_duration + 0.05 {
					info!(
						"jump land start: elapsed={:.3} cycle_t={:.3} land_local={:.4} land_depth={:.4} land_femur={:.4} transition={:.4} land_desc_d={:.4} y={:.4}",
						elapsed,
						self.time_in_cycle(lengths, elapsed),
						local,
						land.depth(land_progress),
						land.femur_swing(land_progress),
						sample.transition_weight,
						timings.land_descent_duration,
						sample.vertical_offset,
					);
				}
				if sample.transition_weight < 1.0 {
					let from_pose = capture_animation_pose(&Fall::<R>::default(), rig, 1.0);
					let _ = Transition::from_pose(land, from_pose)
						.with_curve(TransitionCurve::SmoothStep)
						.apply(rig, land_progress, sample.transition_weight);
				} else {
					land.apply_for(rig, land_progress);
				}
			}
		}
	}

	fn effects_for(&self, rig: &R, elapsed: f32) -> Effects {
		let lengths = rig.segment_lengths();
		let derived = self.rig_derived(lengths);
		let sample = self.cached_sample(lengths, elapsed, &derived);
		Effects {
			r#move: (sample.vertical_offset.abs() > f32::EPSILON)
				.then(|| Transform::from_translation(Vec3::new(0.0, sample.vertical_offset, 0.0))),
		}
	}
}

impl<R: HumanoidRig> TwoFootedJump<R> {
	pub fn log_landing_debug(&self, rig: &R, elapsed: f32, label: &str) {
		let lengths = rig.segment_lengths();
		let derived = self.rig_derived(lengths);
		let sample = self.cached_sample(lengths, elapsed, &derived);
		let timings = derived.timings;
		let land_progress = sample.local_progress / timings.land_duration().max(f32::EPSILON);

		info!(
			"{label}: elapsed={:.3} cycle_t={:.3} segment={:?} local={:.4} land_depth={:.4} y={:.4} timings[squat=({:.3},{:.3}) spring={:.3} air={:.3} land=({:.4},{:.3})] speeds[pre={:.3} landing={:.3}]",
			elapsed,
			self.time_in_cycle(lengths, elapsed),
			sample.segment,
			sample.local_progress,
			derived.landing_squat::<R>().depth(land_progress),
			sample.vertical_offset,
			timings.squat_descent_duration,
			timings.squat_ascent_duration,
			timings.spring_duration,
			timings.air_duration,
			timings.land_descent_duration,
			timings.land_ascent_duration,
			self.pre_squat_speed,
			self.landing_squat_speed,
		);
	}
}

#[cfg(test)]
mod tests {
	use std::hint::black_box;
	use std::time::Instant;

	use character_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;
	use crate::animations::{Squat, DEFAULT_SPRING_DURATION};

	fn default_jump() -> TwoFootedJump<HumanoidV0Rig> {
		TwoFootedJump::default()
	}

	#[test]
	fn spring_end_legs_straight() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig.segment_lengths();
		let elapsed = jump.timings(lengths).squat_end() + DEFAULT_SPRING_DURATION * 0.99;
		jump.apply(&mut rig, elapsed);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur");
		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("shin");
		assert!(femur.swing.abs() < 0.05);
		assert!(shin.flex.abs() < 0.05);
		Ok(())
	}

	#[test]
	fn land_starts_compression_after_touchdown() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		crate::rigs::mix::seed_bind_pose(&mut rig);
		let jump = default_jump();
		let lengths = rig.segment_lengths();
		let timings = jump.timings(lengths);
		jump.apply(&mut rig, timings.air_end() + timings.land_descent_duration * 0.25);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur");
		assert!(femur.swing.abs() > 0.01);
		Ok(())
	}

	#[test]
	fn land_peak_below_full_squat() -> anyhow::Result<()> {
		let mut rig_squat = HumanoidV0Rig::imported();
		Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0).apply(&mut rig_squat, 0.5);
		let squat_femur = rig_squat
			.pose()
			.get(&rig_squat.leg(Side::Left).femur.name)
			.expect("femur")
			.swing;

		let mut rig_land = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig_land.segment_lengths();
		let timings = jump.timings(lengths);
		jump.apply(&mut rig_land, timings.air_end() + timings.land_descent_duration * 0.99);
		let land_femur =
			rig_land.pose().get(&rig_land.leg(Side::Left).femur.name).expect("femur").swing;

		assert!(land_femur.abs() < squat_femur.abs());
		Ok(())
	}

	#[test]
	fn windup_still_drops_the_armature() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig.segment_lengths();
		let mid_windup = jump.timings(lengths).squat_descent_duration * 0.99;
		let effects = jump.apply(&mut rig, mid_windup);
		let Some(tf) = effects.r#move else {
			return Err(anyhow::anyhow!("jump windup must keep Effects.move"));
		};
		if tf.translation.y >= 0.0 {
			return Err(anyhow::anyhow!(
				"windup drop should be negative Y, got {}",
				tf.translation.y
			));
		}
		Ok(())
	}

	#[test]
	fn land_transition_blends_arms_from_fall() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		crate::rigs::mix::seed_bind_pose(&mut rig);
		let jump = default_jump();
		let lengths = rig.segment_lengths();
		let timings = jump.timings(lengths);
		let blend = timings.land_pose_blend_duration();
		jump.apply(&mut rig, timings.air_end() + blend * 0.5);

		let shoulder = rig.pose().get(&rig.arm(Side::Left).shoulder.name).expect("shoulder");
		let fall_shoulder = Fall::<HumanoidV0Rig>::default().shoulder_flex(Side::Left, 1.0);
		assert!(shoulder.flex.abs() > 0.05);
		assert!(shoulder.flex.abs() < fall_shoulder.abs());
		Ok(())
	}

	#[test]
	fn split_apply_and_effects_share_cached_sample() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = default_jump();
		let elapsed = 1.37;
		jump.apply_for(&mut rig, elapsed);
		let effects = jump.effects_for(&rig, elapsed);
		let lengths = rig.segment_lengths();
		let derived = jump.rig_derived(lengths);
		let sample = jump.cached_sample(lengths, elapsed, &derived);
		let y = sample.vertical_offset;
		assert_eq!(effects.r#move.map(|t| t.translation.y), (y.abs() > f32::EPSILON).then_some(y),);
		Ok(())
	}

	#[test]
	#[ignore = "microbench: cargo test -p character-animations two_footed_jump_apply_split --release -- --ignored --nocapture"]
	fn two_footed_jump_apply_split_microbench() {
		const CHARACTERS: usize = 64;
		const FRAMES: usize = 600;
		const RUNS: usize = 5;

		let jump = default_jump();
		let mut rigs: Vec<HumanoidV0Rig> =
			(0..CHARACTERS).map(|_| HumanoidV0Rig::imported()).collect();
		let elapsed_samples: Vec<f32> =
			(0..FRAMES).map(|i| i as f32 * 0.016 + (i % 17) as f32 * 0.003).collect();

		let mut durations = Vec::with_capacity(RUNS);
		for _ in 0..RUNS {
			let start = Instant::now();
			for elapsed in &elapsed_samples {
				for rig in &mut rigs {
					black_box(jump.apply_for(rig, black_box(*elapsed)));
					black_box(jump.effects_for(rig, black_box(*elapsed)));
				}
			}
			durations.push(start.elapsed());
		}
		durations.sort();
		let min = durations[0];
		let median = durations[durations.len() / 2];
		eprintln!(
			"two_footed_jump apply_for+effects_for: characters={CHARACTERS} frames={FRAMES} runs={RUNS} min={min:?} median={median:?}"
		);
	}
}
