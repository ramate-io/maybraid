use bevy::prelude::{Transform, Vec3};
use crozon_rigs::humanoid::HumanoidRig;

use crate::animations::{
	FALL_BLEND_FRACTION, Fall, JumpSegment, LAND_BLEND_FRACTION, Smooth, Spring, Squat,
	TwoFootedJump,
};
use crate::{Animation, Effects};

impl<R: HumanoidRig> Animation<R> for TwoFootedJump<R> {
	fn apply(&self, rig: &mut R) -> Effects {
		let lengths = rig.segment_lengths();
		let (segment, local) = self.segment(lengths);

		match segment {
			JumpSegment::Squat => {
				self.prejump_squat(lengths).apply(rig);
			}
			JumpSegment::Spring => {
				Smooth::<_, _, R>::new(
					Squat::<R>::new(0.0),
					Spring::<R>::extended(),
					local,
				)
				.apply(rig);
			}
			JumpSegment::Fall => {
				let blend = (local / FALL_BLEND_FRACTION).clamp(0.0, 1.0);
				if blend < 1.0 {
					Smooth::<_, _, R>::new(
						Spring::<R>::extended(),
						Fall::<R>::spread(),
						blend,
					)
					.apply(rig);
				} else {
					Fall::<R>::spread().apply(rig);
				}
			}
			JumpSegment::Land => {
				let land = self.landing_squat(lengths);
				let timings = self.timings(lengths);
				let blend_window = timings.land_duration() * LAND_BLEND_FRACTION;
				let blend = if blend_window > f32::EPSILON {
					(local / blend_window).clamp(0.0, 1.0)
				} else {
					1.0
				};
				if blend < 1.0 {
					Smooth::<_, _, R>::new(Fall::<R>::spread(), land, blend).apply(rig);
				} else {
					land.apply(rig);
				}
			}
		}

		let y = self.vertical_offset(lengths);
		Effects {
			r#move: (y.abs() > f32::EPSILON)
				.then(|| Transform::from_translation(Vec3::new(0.0, y, 0.0))),
		}
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use crozon_rigs::humanoid::LegSegmentLengths;

	use super::*;
	use crate::animations::{DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT, DEFAULT_SPRING_DURATION, Squat};

	fn jump_at_elapsed(elapsed: f32) -> TwoFootedJump<HumanoidV0Rig> {
		let lengths = LegSegmentLengths::default();
		TwoFootedJump::auto_scale(elapsed, DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT, lengths)
	}

	#[test]
	fn spring_end_legs_straight() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = jump_at_elapsed(0.0);
		let lengths = rig.segment_lengths();
		let elapsed = jump.timings(lengths).squat_end() + DEFAULT_SPRING_DURATION * 0.99;
		jump_at_elapsed(elapsed).apply(&mut rig);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur");
		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("shin");
		assert!(femur.swing.abs() < 0.05);
		assert!(shin.flex.abs() < 0.05);
		Ok(())
	}

	#[test]
	fn land_blends_from_fall_at_touchdown() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		crate::rigs::mix::seed_bind_pose(&mut rig);
		let jump = jump_at_elapsed(0.0);
		let lengths = rig.segment_lengths();
		jump_at_elapsed(jump.timings(lengths).air_end() + 0.01).apply(&mut rig);

		let shoulder = rig.pose().get(&rig.arm(Side::Left).shoulder.name).expect("shoulder");
		let spread = Fall::<HumanoidV0Rig>::spread().shoulder_flex(Side::Left);
		assert!(shoulder.flex.abs() < spread.abs());

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur");
		assert!(femur.swing.abs() < 0.05);
		Ok(())
	}

	#[test]
	fn land_peak_below_full_squat() -> anyhow::Result<()> {
		let mut rig_squat = HumanoidV0Rig::imported();
		Squat::<HumanoidV0Rig>::new(0.5).apply(&mut rig_squat);
		let squat_femur =
			rig_squat.pose().get(&rig_squat.leg(Side::Left).femur.name).expect("femur").swing;

		let mut rig_land = HumanoidV0Rig::imported();
		let jump = jump_at_elapsed(0.0);
		let lengths = rig_land.segment_lengths();
		let timings = jump.timings(lengths);
		jump_at_elapsed(timings.air_end() + timings.land_descent_duration * 0.99).apply(&mut rig_land);
		let land_femur =
			rig_land.pose().get(&rig_land.leg(Side::Left).femur.name).expect("femur").swing;

		assert!(land_femur.abs() < squat_femur.abs());
		Ok(())
	}
}
