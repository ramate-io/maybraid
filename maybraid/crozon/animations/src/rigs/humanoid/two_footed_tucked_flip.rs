use bevy::prelude::{Quat, Transform, Vec3};
use crozon_rigs::humanoid::HumanoidRig;
use log::info;

use crate::animations::{
	FixedPosition, JumpSegment, Spring, Squat, Transition, TransitionCurve, Tuck,
	TwoFootedTuckedFlip, FALL_BLEND_FRACTION,
};
use crate::rigs::transition::capture_animation_pose;
use crate::{Animation, Effects};

fn segment_debug_enabled() -> bool {
	std::env::var("CROZON_ANIMATION_DEBUG").is_ok()
}

impl<R: HumanoidRig> Animation<R> for TwoFootedTuckedFlip<R> {
	fn apply(&self, rig: &mut R, elapsed: f32) -> Effects {
		let lengths = rig.segment_lengths();
		let (segment, local) = self.segment(lengths, elapsed);
		let timings = self.timings(lengths);
		let jump = &self.jump;
		let flip = &self.flip;

		match segment {
			JumpSegment::Squat => {
				let squat = jump.prejump_squat(lengths);
				let progress = local / timings.squat_duration().max(f32::EPSILON);
				squat.apply(rig, progress);
			}
			JumpSegment::Spring => {
				let from_pose = capture_animation_pose(&Squat::<R>::for_loop(1.0, 1.0), rig, 0.0);
				Transition::from_pose(Spring::<R>::default(), from_pose)
					.with_curve(TransitionCurve::SmoothStep)
					.apply(rig, local, local);
			}
			JumpSegment::Fall => {
				let tuck = Tuck::<R>::new(flip.tuck.tightness());
				let blend_end = FALL_BLEND_FRACTION;
				if segment_debug_enabled() && local > 0.9 {
					info!(
						"tucked flip air end: elapsed={:.3} flip_local={:.4} pitch={:.4}",
						elapsed,
						local,
						flip.pitch_radians(local),
					);
				}
				if local < blend_end {
					let from_pose = capture_animation_pose(&Spring::<R>::default(), rig, 1.0);
					let transition_progress = (local / blend_end).clamp(0.0, 1.0);
					Transition::from_pose(tuck, from_pose)
						.with_curve(TransitionCurve::SmoothStep)
						.apply(rig, 1.0, transition_progress);
				} else {
					flip.tuck.apply_fixed(rig);
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
				if segment_debug_enabled() && local < timings.land_descent_duration + 0.05 {
					info!(
						"tucked flip land start: elapsed={:.3} land_local={:.4} transition={:.4}",
						elapsed, local, transition_progress,
					);
				}
				if transition_progress < 1.0 {
					let from_pose = capture_animation_pose(&flip.tuck, rig, 0.0);
					Transition::from_pose(land, from_pose)
						.with_curve(TransitionCurve::SmoothStep)
						.apply(rig, land_progress, transition_progress);
				} else {
					land.apply(rig, land_progress);
				}
			}
		}

		let y = self.vertical_offset(lengths, elapsed);
		let pitch = self.flip_pitch_radians(lengths, elapsed);
		Effects {
			r#move: (y.abs() > f32::EPSILON || pitch.abs() > f32::EPSILON).then(|| Transform {
				translation: Vec3::new(0.0, y, 0.0),
				rotation: Quat::from_rotation_x(pitch),
				..Default::default()
			}),
		}
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;
	use crate::animations::DEFAULT_SPRING_DURATION;

	fn default_flip() -> TwoFootedTuckedFlip<HumanoidV0Rig> {
		TwoFootedTuckedFlip::default()
	}

	#[test]
	fn spring_end_legs_straight() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let flip = default_flip();
		let lengths = rig.segment_lengths();
		let elapsed = flip.timings(lengths).squat_end() + DEFAULT_SPRING_DURATION * 0.99;
		flip.apply(&mut rig, elapsed);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur");
		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("shin");
		assert!(femur.swing.abs() < 0.05);
		assert!(shin.flex.abs() < 0.05);
		Ok(())
	}

	#[test]
	fn mid_air_applies_tuck_and_forward_pitch() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let flip = default_flip();
		let lengths = rig.segment_lengths();
		let timings = flip.timings(lengths);
		let elapsed = timings.spring_end() + timings.air_duration * 0.5;
		let effects = flip.apply(&mut rig, elapsed);

		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("shin");
		assert!(shin.flex > 1.0);

		let offset = effects.r#move.expect("combined effect");
		assert!(offset.translation.y > 0.0);
		assert!(offset.rotation.to_euler(bevy::prelude::EulerRot::XYZ).0 > 0.0);
		Ok(())
	}

	#[test]
	fn land_blends_leg_compression_with_neutral_arms() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		crate::rigs::mix::seed_bind_pose(&mut rig);
		let flip = default_flip();
		let lengths = rig.segment_lengths();
		let timings = flip.timings(lengths);
		let blend = timings.land_pose_blend_duration();
		flip.apply(&mut rig, timings.air_end() + blend * 0.5);

		let shoulder = rig.pose().get(&rig.arm(Side::Left).shoulder.name).expect("shoulder");
		assert!(
			shoulder.swing.abs() < 0.05,
			"landing blends from tuck at progress 0, which keeps arms neutral"
		);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur");
		assert!(femur.swing.abs() > 0.01, "legs should be partway into landing squat");
		Ok(())
	}

	#[test]
	fn land_starts_compression_after_touchdown() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		crate::rigs::mix::seed_bind_pose(&mut rig);
		let flip = default_flip();
		let lengths = rig.segment_lengths();
		let timings = flip.timings(lengths);
		flip.apply(&mut rig, timings.air_end() + timings.land_descent_duration * 0.25);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur");
		assert!(femur.swing.abs() > 0.01);
		Ok(())
	}
}
