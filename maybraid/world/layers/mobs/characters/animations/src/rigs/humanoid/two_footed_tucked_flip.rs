use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use log::info;

use crate::animations::{JumpSample, JumpSegment, Tuck, TwoFootedTuckedFlip};
use crate::rigs::segment_debug::segment_debug_enabled;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for TwoFootedTuckedFlip {
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

impl TwoFootedTuckedFlip {
	fn resolve_sample(&self, rig: &HumanoidV0Rig, elapsed: f32) -> JumpSample {
		let sample = self.sample(rig.segment_lengths, elapsed);
		if segment_debug_enabled() {
			self.log_apply_debug(elapsed, sample);
		}
		sample
	}

	fn apply_sample(&self, rig: &mut HumanoidV0Rig, sample: JumpSample) {
		let tuck = Tuck::new(self.flip.tuck.tightness());
		sample.apply_humanoid_pose(rig, &self.jump, &tuck, 1.0, &self.flip.tuck, 0.0);
	}

	fn log_apply_debug(&self, elapsed: f32, sample: JumpSample) {
		let local = sample.local;
		let timings = sample.timings;
		match sample.segment {
			JumpSegment::Fall if local > 0.9 => {
				info!(
					"tucked flip air end: elapsed={:.3} flip_local={:.4} pitch={:.4}",
					elapsed,
					local,
					self.flip.pitch_radians(local),
				);
			}
			JumpSegment::Land if local < timings.land_descent_duration + 0.05 => {
				let blend_window = timings.land_pose_blend_duration();
				let transition_progress = if blend_window > f32::EPSILON {
					(local / blend_window).clamp(0.0, 1.0)
				} else {
					1.0
				};
				info!(
					"tucked flip land start: elapsed={:.3} land_local={:.4} transition={:.4}",
					elapsed, local, transition_progress,
				);
			}
			_ => {}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::{
		smoothstep, FixedPosition, JumpSegment, Spring, Squat, TwoFootedTuckedFlip,
		DEFAULT_SPRING_DURATION, FALL_BLEND_FRACTION,
	};
	use crate::rigs::mix::blend_clips;
	use bevy::prelude::{Quat, Transform, Vec3};
	use character_rigs::authoring::ArmatureOffset;

	fn default_flip() -> TwoFootedTuckedFlip {
		TwoFootedTuckedFlip::default()
	}

	/// Frozen pre-share `apply_for` from the independent tucked-flip clip.
	fn frozen_apply_for(flip_clip: &TwoFootedTuckedFlip, rig: &mut HumanoidV0Rig, elapsed: f32) {
		let lengths = rig.segment_lengths;
		let jump = &flip_clip.jump;
		let flip = &flip_clip.flip;
		let timings = jump.timings(lengths);
		let cycle = timings.cycle_duration();
		let time_in_cycle = if cycle <= f32::EPSILON { 0.0 } else { elapsed % cycle };
		let (segment, local) =
			crate::animations::TwoFootedJump::segment_at_time(time_in_cycle, &timings);

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
				let tuck = Tuck::new(flip.tuck.tightness());
				let blend_end = FALL_BLEND_FRACTION;
				if local < blend_end {
					let transition_progress = (local / blend_end).clamp(0.0, 1.0);
					blend_clips(
						rig,
						&Spring::default(),
						1.0,
						&tuck,
						1.0,
						smoothstep(transition_progress),
					);
				} else {
					let _ = flip.tuck.apply_fixed(rig);
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
						&flip.tuck,
						0.0,
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

	/// Frozen pre-share height + pitch (independent `vertical_offset` / `flip_pitch_radians`).
	fn frozen_effects(
		flip_clip: &TwoFootedTuckedFlip,
		rig: &HumanoidV0Rig,
		elapsed: f32,
	) -> Effects {
		let lengths = rig.segment_lengths;
		let jump = &flip_clip.jump;
		let timings = jump.timings(lengths);
		let cycle = timings.cycle_duration();
		let time_in_cycle = if cycle <= f32::EPSILON { 0.0 } else { elapsed % cycle };
		let (segment, local) =
			crate::animations::TwoFootedJump::segment_at_time(time_in_cycle, &timings);
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
		let pitch =
			if segment == JumpSegment::Fall { flip_clip.flip.pitch_radians(local) } else { 0.0 };
		if y.abs() <= f32::EPSILON && pitch.abs() <= f32::EPSILON {
			return ArmatureOffset::IDENTITY;
		}
		ArmatureOffset(Transform {
			translation: Vec3::new(0.0, y, 0.0),
			rotation: Quat::from_rotation_x(pitch),
			scale: Vec3::ONE,
		})
	}

	fn phase_times(
		flip: &TwoFootedTuckedFlip,
		lengths: character_rigs::humanoid::LegSegmentLengths,
	) -> Vec<(&'static str, f32)> {
		let timings = flip.timings(lengths);
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
			("land_compress", timings.air_end() + timings.land_descent_duration * 0.25),
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
	fn numeric_equivalence_vs_frozen_tucked_flip() -> anyhow::Result<()> {
		let flip = default_flip();
		let lengths = HumanoidV0Rig::imported().segment_lengths;
		for (label, elapsed) in phase_times(&flip, lengths) {
			let mut live = HumanoidV0Rig::imported();
			let live_effects = flip.apply(&mut live, elapsed);
			let mut frozen = HumanoidV0Rig::imported();
			frozen_apply_for(&flip, &mut frozen, elapsed);
			let frozen_fx = frozen_effects(&flip, &frozen, elapsed);
			assert_pose_eq(&live, &frozen, label)?;
			assert_effects_eq(live_effects, frozen_fx, label)?;

			let mut bones_only = HumanoidV0Rig::imported();
			flip.apply_for(&mut bones_only, elapsed);
			assert_pose_eq(&live, &bones_only, &format!("{label} bones-only"))?;

			let rest = HumanoidV0Rig::imported();
			let effects_only = flip.effects_for(&rest, elapsed);
			assert_effects_eq(live_effects, effects_only, &format!("{label} effects-only"))?;
			if rest.posed_angle("shin.L") > 1e-5 {
				return Err(anyhow::anyhow!("{label}: effects-only must not write bones"));
			}
		}
		Ok(())
	}

	#[test]
	fn spring_end_legs_straight() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let flip = default_flip();
		let lengths = rig.segment_lengths;
		let elapsed = flip.timings(lengths).squat_end() + DEFAULT_SPRING_DURATION * 0.99;
		flip.apply(&mut rig, elapsed);

		assert!(rig.posed_angle("femur.L") < 0.05, "femur should be straight");
		assert!(rig.posed_angle("shin.L") < 0.05, "shin should be straight");
		Ok(())
	}

	#[test]
	fn mid_air_applies_tuck_and_forward_pitch() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let flip = default_flip();
		let lengths = rig.segment_lengths;
		let timings = flip.timings(lengths);
		let elapsed = timings.spring_end() + timings.air_duration * 0.5;
		let effects = flip.apply(&mut rig, elapsed);

		assert!(rig.posed_angle("shin.L") > 0.5, "knee folds in the tuck");

		assert!(effects.0.translation.y > 0.0);
		assert!(effects.0.rotation.to_euler(bevy::prelude::EulerRot::XYZ).0 > 0.0);
		Ok(())
	}

	#[test]
	fn land_blends_leg_compression_with_tuck_arms() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let flip = default_flip();
		let lengths = rig.segment_lengths;
		let timings = flip.timings(lengths);
		let blend = timings.land_pose_blend_duration();
		flip.apply(&mut rig, timings.air_end() + blend * 0.5);

		let mut tucked = HumanoidV0Rig::imported();
		flip.flip.tuck.apply_fixed(&mut tucked);
		assert!(
			rig.posed_angle("shoulder.L") < tucked.posed_angle("shoulder.L"),
			"landing blend is short of the held tuck"
		);
		assert!(rig.posed_angle("femur.L") > 0.01, "legs should be partway into landing squat");
		Ok(())
	}

	#[test]
	fn land_starts_compression_after_touchdown() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let flip = default_flip();
		let lengths = rig.segment_lengths;
		let timings = flip.timings(lengths);
		flip.apply(&mut rig, timings.air_end() + timings.land_descent_duration * 0.25);

		assert!(rig.posed_angle("femur.L") > 0.01);
		Ok(())
	}
}
