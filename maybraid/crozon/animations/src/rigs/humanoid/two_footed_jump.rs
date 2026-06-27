use bevy::prelude::{Transform, Vec3};
use crozon_rigs::humanoid::HumanoidRig;

use crate::animations::{
	FALL_BLEND_FRACTION, Fall, JumpSegment, Land, Smooth, Spring, Squat, TwoFootedJump,
};
use crate::{Effects, Animation};

impl<R: HumanoidRig> Animation<R> for TwoFootedJump<R> {
	fn apply(&self, rig: &mut R) -> Effects {
		let (segment, local) = self.segment();
		let lengths = rig.segment_lengths();

		match segment {
			JumpSegment::Squat => {
				Squat::<R>::new(local).apply(rig);
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
				Land::<R>::new(local, Squat::<R>::default()).apply(rig);
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

	use super::*;
	use crate::animations::{DEFAULT_SPRING_DURATION, DEFAULT_SQUAT_SPEED, Squat};

	fn jump_at_elapsed(elapsed: f32) -> TwoFootedJump<HumanoidV0Rig> {
		TwoFootedJump::new(elapsed)
	}

	#[test]
	fn spring_end_legs_straight() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = jump_at_elapsed(1.0 / DEFAULT_SQUAT_SPEED + DEFAULT_SPRING_DURATION * 0.99);
		jump.apply(&mut rig);

		let femur = rig.pose().get(&rig.leg(Side::Left).femur.name).expect("femur");
		let shin = rig.pose().get(&rig.leg(Side::Left).shin.name).expect("shin");
		assert!(femur.swing.abs() < 0.05);
		assert!(shin.flex.abs() < 0.05);
		Ok(())
	}

	#[test]
	fn spring_midpoint_blends_from_stand() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		crate::rigs::mix::seed_bind_pose(&mut rig);
		let jump = jump_at_elapsed(1.0 / DEFAULT_SQUAT_SPEED + DEFAULT_SPRING_DURATION * 0.5);
		jump.apply(&mut rig);

		let shoulder = rig.pose().get(&rig.arm(Side::Left).shoulder.name).expect("shoulder");
		let full = Spring::<HumanoidV0Rig>::extended().shoulder_swing();
		assert!(shoulder.swing < 0.0);
		assert!(shoulder.swing.abs() < full.abs());
		Ok(())
	}

	#[test]
	fn fall_blends_from_spring_at_air_start() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		crate::rigs::mix::seed_bind_pose(&mut rig);
		let jump = jump_at_elapsed(jump_at_elapsed(0.0).timings().spring_end() + 0.01);
		jump.apply(&mut rig);

		let shoulder = rig.pose().get(&rig.arm(Side::Left).shoulder.name).expect("shoulder");
		let spread = Fall::<HumanoidV0Rig>::spread().shoulder_flex(Side::Left);
		assert!(shoulder.flex.abs() < spread.abs());
		Ok(())
	}

	#[test]
	fn fall_spreads_arms() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = jump_at_elapsed(0.0);
		let elapsed = jump.timings().spring_end() + jump.timings().air_duration * 0.5;
		jump_at_elapsed(elapsed).apply(&mut rig);

		let left = rig.pose().get(&rig.arm(Side::Left).shoulder.name).expect("shoulder");
		let right = rig.pose().get(&rig.arm(Side::Right).shoulder.name).expect("shoulder");
		assert!(left.flex.abs() > 0.1);
		assert!(right.flex.abs() > 0.1);
		Ok(())
	}

	#[test]
	fn land_peak_below_full_squat() -> anyhow::Result<()> {
		let mut rig_squat = HumanoidV0Rig::imported();
		Squat::<HumanoidV0Rig>::new(0.5).apply(&mut rig_squat);
		let squat_femur =
			rig_squat.pose().get(&rig_squat.leg(Side::Left).femur.name).expect("femur").swing;

		let mut rig_land = HumanoidV0Rig::imported();
		let timings = jump_at_elapsed(0.0).timings();
		jump_at_elapsed(timings.air_end() + timings.land_duration * 0.5).apply(&mut rig_land);
		let land_femur =
			rig_land.pose().get(&rig_land.leg(Side::Left).femur.name).expect("femur").swing;

		assert!(land_femur.abs() < squat_femur.abs());
		Ok(())
	}

	#[test]
	fn jump_effects_near_zero_at_cycle_boundaries() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let start = jump_at_elapsed(0.0).apply(&mut rig);
		assert!(start.r#move.is_none() || start.r#move.unwrap().translation.y.abs() < 1e-4);

		let mut rig = HumanoidV0Rig::imported();
		let jump = jump_at_elapsed(0.0);
		let end = jump_at_elapsed(jump.cycle_duration() * 0.999).apply(&mut rig);
		let y = end.r#move.map(|t| t.translation.y).unwrap_or(0.0);
		assert!(y.abs() < 0.1);
		Ok(())
	}

	#[test]
	fn squat_to_spring_vertical_is_continuous() -> anyhow::Result<()> {
		let lengths = crozon_rigs::humanoid::LegSegmentLengths::default();
		let jump = TwoFootedJump::<()>::default();
		let timings = jump.timings();
		let squat_end = TwoFootedJump::<()>::new(timings.squat_end() * 0.99);
		let spring_start = TwoFootedJump::<()>::new(timings.squat_end());
		assert!(
			(squat_end.vertical_offset(lengths) - spring_start.vertical_offset(lengths)).abs() < 0.05
		);
		Ok(())
	}
}
