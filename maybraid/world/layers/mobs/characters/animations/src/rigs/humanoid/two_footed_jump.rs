use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use log::info;

use crate::animations::{Fall, JumpSample, JumpSegment, TwoFootedJump};
use crate::rigs::segment_debug::segment_debug_enabled;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for TwoFootedJump {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, elapsed: f32) {
		let sample = self.resolve_sample(rig, elapsed);
		self.apply_sample(rig, sample);
	}

	fn effects_for(&self, rig: &HumanoidV0Rig, elapsed: f32) -> Effects {
		self.sample(rig.segment_lengths, elapsed).effects()
	}

	fn apply(&self, rig: &mut HumanoidV0Rig, elapsed: f32) -> Effects {
		let sample = self.resolve_sample(rig, elapsed);
		self.apply_sample(rig, sample);
		sample.effects()
	}
}

impl TwoFootedJump {
	fn resolve_sample(&self, rig: &HumanoidV0Rig, elapsed: f32) -> JumpSample {
		let sample = self.sample(rig.segment_lengths, elapsed);
		if segment_debug_enabled() {
			self.log_apply_debug(rig, elapsed, sample);
		}
		sample
	}

	fn apply_sample(&self, rig: &mut HumanoidV0Rig, sample: JumpSample) {
		let fall = Fall::default();
		sample.apply_humanoid_pose(rig, self, &fall, sample.local, &fall, 1.0);
	}

	fn log_apply_debug(&self, rig: &HumanoidV0Rig, elapsed: f32, sample: JumpSample) {
		let lengths = rig.segment_lengths;
		let local = sample.local;
		let timings = sample.timings;
		match sample.segment {
			JumpSegment::Fall if local > 0.9 => {
				let fall = Fall::default();
				info!(
					"jump fall end: elapsed={:.3} cycle_t={:.3} air_end={:.3} fall_local={:.4} fall_femur=0 fall_shoulder_flex={:.4}",
					elapsed,
					self.time_in_cycle(lengths, elapsed),
					timings.air_end(),
					local,
					fall.shoulder_flex(character_rigs::Side::Left, local),
				);
			}
			JumpSegment::Land if local < timings.land_descent_duration + 0.05 => {
				let land = self.landing_squat(lengths);
				let land_progress = local / timings.land_duration().max(f32::EPSILON);
				let blend_window = timings.land_pose_blend_duration();
				let transition_progress = if blend_window > f32::EPSILON {
					(local / blend_window).clamp(0.0, 1.0)
				} else {
					1.0
				};
				info!(
					"jump land start: elapsed={:.3} cycle_t={:.3} land_local={:.4} land_depth={:.4} land_femur={:.4} transition={:.4} land_desc_d={:.4} y={:.4}",
					elapsed,
					self.time_in_cycle(lengths, elapsed),
					local,
					land.depth(land_progress),
					land.femur_swing(land_progress),
					transition_progress,
					timings.land_descent_duration,
					sample.height,
				);
			}
			_ => {}
		}
	}

	pub fn log_landing_debug(&self, rig: &HumanoidV0Rig, elapsed: f32, label: &str) {
		let lengths = rig.segment_lengths;
		let sample = self.sample(lengths, elapsed);
		let timings = sample.timings;
		let time_in_cycle = self.time_in_cycle(lengths, elapsed);
		let land = self.landing_squat(lengths);
		let land_progress = sample.local / timings.land_duration().max(f32::EPSILON);

		info!(
			"{label}: elapsed={:.3} cycle_t={:.3} segment={:?} local={:.4} land_depth={:.4} y={:.4} timings[squat=({:.3},{:.3}) spring={:.3} air={:.3} land=({:.4},{:.3})] speeds[pre={:.3} landing={:.3}]",
			elapsed,
			time_in_cycle,
			sample.segment,
			sample.local,
			land.depth(land_progress),
			sample.height,
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
	use super::*;
	use crate::animations::{
		smoothstep, Fall, JumpSegment, Spring, Squat, DEFAULT_SPRING_DURATION, FALL_BLEND_FRACTION,
	};
	use crate::rigs::mix::blend_clips;
	use bevy::prelude::Vec3;
	use character_rigs::authoring::ArmatureOffset;

	fn default_jump() -> TwoFootedJump {
		TwoFootedJump::default()
	}

	/// Frozen pre-share `apply_for` from the independent jump clip.
	fn frozen_apply_for(jump: &TwoFootedJump, rig: &mut HumanoidV0Rig, elapsed: f32) {
		let lengths = rig.segment_lengths;
		let timings = jump.timings(lengths);
		let cycle = timings.cycle_duration();
		let time_in_cycle = if cycle <= f32::EPSILON { 0.0 } else { elapsed % cycle };
		let (segment, local) = TwoFootedJump::segment_at_time(time_in_cycle, &timings);

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

	/// Frozen pre-share `effects_for` / `vertical_offset` for a plain jump.
	fn frozen_effects(jump: &TwoFootedJump, rig: &HumanoidV0Rig, elapsed: f32) -> Effects {
		let lengths = rig.segment_lengths;
		let timings = jump.timings(lengths);
		let cycle = timings.cycle_duration();
		let time_in_cycle = if cycle <= f32::EPSILON { 0.0 } else { elapsed % cycle };
		let (segment, local) = TwoFootedJump::segment_at_time(time_in_cycle, &timings);
		let y = match segment {
			JumpSegment::Squat => {
				let squat = jump.prejump_squat(lengths);
				let p = local / timings.squat_duration().max(f32::EPSILON);
				-squat.vertical_drop(p, lengths)
			}
			JumpSegment::Spring | JumpSegment::Fall => {
				let since_launch = (time_in_cycle - timings.squat_end()).max(0.0);
				jump.ballistic_height(since_launch.min(jump.touchdown_time_since_launch()))
			}
			JumpSegment::Land => {
				let land = jump.landing_squat(lengths);
				let p = local / timings.land_duration().max(f32::EPSILON);
				-land.vertical_drop(p, lengths)
			}
		};
		if y.abs() > f32::EPSILON {
			ArmatureOffset::from_translation(Vec3::new(0.0, y, 0.0))
		} else {
			ArmatureOffset::IDENTITY
		}
	}

	fn phase_times(
		jump: &TwoFootedJump,
		lengths: character_rigs::humanoid::LegSegmentLengths,
	) -> Vec<(&'static str, f32)> {
		let timings = jump.timings(lengths);
		vec![
			("start", 0.0),
			("squat_mid", timings.squat_duration() * 0.5),
			("squat_end", timings.squat_end()),
			("spring_mid", timings.squat_end() + timings.spring_duration * 0.5),
			("spring_end", timings.spring_end()),
			("fall_blend", timings.spring_end() + timings.air_duration * FALL_BLEND_FRACTION * 0.5),
			("fall_mid", timings.spring_end() + timings.air_duration * 0.5),
			("air_end", timings.air_end()),
			("land_blend", timings.air_end() + timings.land_pose_blend_duration() * 0.5),
			("land_compress", timings.air_end() + timings.land_descent_duration * 0.99),
			(
				"land_recover",
				timings.air_end()
					+ timings.land_descent_duration
					+ timings.land_ascent_duration * 0.5,
			),
			("cycle_end", timings.cycle_duration() * 0.999),
		]
	}

	fn assert_pose_eq(a: &HumanoidV0Rig, b: &HumanoidV0Rig, label: &str) -> anyhow::Result<()> {
		for name in a.animation_bone_names() {
			let dot = a.rotation(name).dot(b.rotation(name)).abs();
			if dot <= 1.0 - 1e-5 {
				return Err(anyhow::anyhow!("{label}: pose drift on {name}, dot={dot}"));
			}
		}
		Ok(())
	}

	fn assert_effects_eq(a: Effects, b: Effects, label: &str) -> anyhow::Result<()> {
		if (a.0.translation - b.0.translation).length() > 1e-5 {
			return Err(anyhow::anyhow!(
				"{label}: translation {} vs {}",
				a.0.translation,
				b.0.translation
			));
		}
		if a.0.rotation.dot(b.0.rotation).abs() <= 1.0 - 1e-5 {
			return Err(anyhow::anyhow!("{label}: rotation mismatch"));
		}
		Ok(())
	}

	#[test]
	fn numeric_equivalence_vs_frozen_jump() -> anyhow::Result<()> {
		let jump = default_jump();
		let lengths = HumanoidV0Rig::imported().segment_lengths;
		for (label, elapsed) in phase_times(&jump, lengths) {
			let mut live = HumanoidV0Rig::imported();
			let live_effects = jump.apply(&mut live, elapsed);
			let mut frozen = HumanoidV0Rig::imported();
			frozen_apply_for(&jump, &mut frozen, elapsed);
			let frozen_effects = frozen_effects(&jump, &frozen, elapsed);
			assert_pose_eq(&live, &frozen, label)?;
			assert_effects_eq(live_effects, frozen_effects, label)?;

			let mut bones_only = HumanoidV0Rig::imported();
			jump.apply_for(&mut bones_only, elapsed);
			assert_pose_eq(&live, &bones_only, &format!("{label} bones-only"))?;

			let rest = HumanoidV0Rig::imported();
			let effects_only = jump.effects_for(&rest, elapsed);
			assert_effects_eq(live_effects, effects_only, &format!("{label} effects-only"))?;
			if rest.posed_angle("femur.L") > 1e-5 {
				return Err(anyhow::anyhow!("{label}: effects-only must not write bones"));
			}
		}
		Ok(())
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
}
