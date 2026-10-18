use bevy::prelude::{Quat, Transform, Vec3};
use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use log::info;

use crate::animations::{JumpSample, JumpSegment, TwoFootedTuckedFlip};
use crate::rigs::segment_debug::segment_debug_enabled;
use crate::{Animation, Effects};

impl TwoFootedTuckedFlip {
	fn apply_sampled(&self, rig: &mut HumanoidV0Rig, elapsed: f32, sample: &JumpSample) {
		if segment_debug_enabled() {
			self.log_sampled_debug(elapsed, sample);
		}
		sample.apply_pose(rig, &self.flip.tuck, 1.0, 0.0);
	}

	fn effects_from_sample(
		&self,
		lengths: character_rigs::humanoid::LegSegmentLengths,
		sample: &JumpSample,
	) -> Effects {
		let y = sample.vertical_offset(&self.jump, lengths);
		let pitch = sample.flip_pitch_radians(&self.flip);
		if y.abs() <= f32::EPSILON && pitch.abs() <= f32::EPSILON {
			return ArmatureOffset::IDENTITY;
		}
		ArmatureOffset(Transform {
			translation: Vec3::new(0.0, y, 0.0),
			rotation: Quat::from_rotation_x(pitch),
			scale: Vec3::ONE,
		})
	}

	fn log_sampled_debug(&self, elapsed: f32, sample: &JumpSample) {
		match sample.segment {
			JumpSegment::Fall if sample.local > 0.9 => {
				info!(
					"tucked flip air end: elapsed={:.3} flip_local={:.4} pitch={:.4}",
					elapsed,
					sample.local,
					self.flip.pitch_radians(sample.local),
				);
			}
			JumpSegment::Land if sample.local < sample.timings.land_descent_duration + 0.05 => {
				info!(
					"tucked flip land start: elapsed={:.3} land_local={:.4} transition={:.4}",
					elapsed, sample.local, sample.blend_progress,
				);
			}
			_ => {}
		}
	}
}

impl Animation<HumanoidV0Rig> for TwoFootedTuckedFlip {
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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::FixedPosition;
	use crate::animations::DEFAULT_SPRING_DURATION;
	use crate::animations::{smoothstep, Fall, Spring, Squat, TwoFootedJump, FALL_BLEND_FRACTION};
	use crate::rigs::mix::blend_clips;

	fn default_flip() -> TwoFootedTuckedFlip {
		TwoFootedTuckedFlip::default()
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

	fn assert_pose_near(a: &HumanoidV0Rig, b: &HumanoidV0Rig, label: &str) {
		for name in a.animation_bone_names() {
			assert!(a.rotation(name).dot(b.rotation(name)).abs() > 1.0 - 1e-5, "{label}: {name}");
		}
	}

	fn legacy_apply_for(flip: &TwoFootedTuckedFlip, rig: &mut HumanoidV0Rig, elapsed: f32) {
		use crate::animations::Tuck;
		let lengths = rig.segment_lengths;
		let jump = &flip.jump;
		let timings = jump.timings(lengths);
		let cycle = timings.cycle_duration();
		let cycle_t = if cycle <= f32::EPSILON { 0.0 } else { elapsed % cycle };
		let (segment, local) = TwoFootedJump::segment_at_time(cycle_t, &timings);

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
				let tuck = Tuck::new(flip.flip.tuck.tightness());
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
					let _ = flip.flip.tuck.apply_fixed(rig);
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
						&flip.flip.tuck,
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

	fn legacy_effects_for(
		flip: &TwoFootedTuckedFlip,
		rig: &HumanoidV0Rig,
		elapsed: f32,
	) -> Effects {
		let y = flip.vertical_offset(rig.segment_lengths, elapsed);
		let pitch = flip.flip_pitch_radians(rig.segment_lengths, elapsed);
		if y.abs() <= f32::EPSILON && pitch.abs() <= f32::EPSILON {
			return ArmatureOffset::IDENTITY;
		}
		ArmatureOffset(Transform {
			translation: Vec3::new(0.0, y, 0.0),
			rotation: Quat::from_rotation_x(pitch),
			scale: Vec3::ONE,
		})
	}

	fn sample_elapsed_grid(
		flip: &TwoFootedTuckedFlip,
		lengths: character_rigs::humanoid::LegSegmentLengths,
	) -> Vec<f32> {
		let timings = flip.timings(lengths);
		vec![
			0.0,
			timings.squat_end() * 0.5,
			timings.squat_end() + timings.spring_duration * 0.5,
			timings.spring_end() + timings.air_duration * FALL_BLEND_FRACTION * 0.4,
			timings.spring_end() + timings.air_duration * 0.5,
			timings.air_end() + timings.land_pose_blend_duration() * 0.5,
			timings.air_end() + timings.land_descent_duration * 0.25,
			timings.cycle_duration() * 0.99,
		]
	}

	#[test]
	fn tucked_flip_pose_and_effects_match_legacy_split_path() -> anyhow::Result<()> {
		let flip = default_flip();
		let lengths = HumanoidV0Rig::imported().segment_lengths;
		for elapsed in sample_elapsed_grid(&flip, lengths) {
			let mut legacy = HumanoidV0Rig::imported();
			legacy_apply_for(&flip, &mut legacy, elapsed);
			let legacy_effects = legacy_effects_for(&flip, &legacy, elapsed);

			let mut split = HumanoidV0Rig::imported();
			flip.apply_for(&mut split, elapsed);
			let split_effects = flip.effects_for(&split, elapsed);

			let mut combined = HumanoidV0Rig::imported();
			let combined_effects = flip.apply(&mut combined, elapsed);

			assert_pose_near(&legacy, &split, &format!("split elapsed={elapsed}"));
			assert_pose_near(&legacy, &combined, &format!("combined elapsed={elapsed}"));
			assert_eq!(legacy_effects, split_effects, "split effects elapsed={elapsed}");
			assert_eq!(legacy_effects, combined_effects, "combined effects elapsed={elapsed}");
		}
		Ok(())
	}

	#[test]
	fn tucked_flip_does_not_use_fall_airborne_pose() -> anyhow::Result<()> {
		let mut flip_rig = HumanoidV0Rig::imported();
		let mut fall_rig = HumanoidV0Rig::imported();
		let flip = default_flip();
		let lengths = flip_rig.segment_lengths;
		let timings = flip.timings(lengths);
		let elapsed = timings.spring_end() + timings.air_duration * 0.5;
		flip.apply_for(&mut flip_rig, elapsed);
		Fall::default().apply_for(&mut fall_rig, 1.0);
		assert!(
			flip_rig.posed_angle("shin.L") > fall_rig.posed_angle("shin.L") + 0.3,
			"flip airborne must tuck, not fall-spread"
		);
		Ok(())
	}
}
