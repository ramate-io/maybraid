use bevy::prelude::{Transform, Vec3};
use crozon_rigs::{humanoid::HumanoidRig, Side};
use log::info;

use crate::animations::{
	Fall, JumpSegment, Spring, Squat, Transition, TransitionCurve, TwoFootedJump,
	FALL_BLEND_FRACTION,
};
use crate::rigs::transition::capture_animation_pose;
use crate::{Animation, Effects};

fn segment_debug_enabled() -> bool {
	std::env::var("CROZON_ANIMATION_DEBUG").is_ok()
}

impl<R: HumanoidRig> Animation<R> for TwoFootedJump<R> {
	fn apply(&self, rig: &mut R, elapsed: f32) -> Effects {
		let lengths = rig.segment_lengths();
		let (segment, local) = self.segment(lengths, elapsed);
		let timings = self.timings(lengths);

		match segment {
			JumpSegment::Squat => {
				let squat = self.prejump_squat(lengths);
				let progress = local / timings.squat_duration().max(f32::EPSILON);
				squat.apply(rig, progress);
			}
			JumpSegment::Spring => {
				let from_pose =
					capture_animation_pose(&Squat::<R>::for_loop(1.0, 1.0), rig, 0.0);
				Transition::from_pose(Spring::<R>::default(), from_pose)
					.with_curve(TransitionCurve::SmoothStep)
					.apply(rig, local, local);
			}
			JumpSegment::Fall => {
				let fall = Fall::<R>::default();
				let blend_end = FALL_BLEND_FRACTION;
				if segment_debug_enabled() && local > 0.9 {
					info!(
						"jump fall end: elapsed={:.3} cycle_t={:.3} air_end={:.3} fall_local={:.4} fall_femur=0 fall_shoulder_flex={:.4}",
						elapsed,
						self.time_in_cycle(lengths, elapsed),
						timings.air_end(),
						local,
						fall.shoulder_flex(Side::Left, 1.0),
					);
				}
				if local < blend_end {
					let from_pose =
						capture_animation_pose(&Spring::<R>::default(), rig, 1.0);
					let transition_progress = (local / blend_end).clamp(0.0, 1.0);
					Transition::from_pose(fall, from_pose)
						.with_curve(TransitionCurve::SmoothStep)
						.apply(rig, 1.0, transition_progress);
				} else {
					fall.apply(rig, 1.0);
				}
			}
			JumpSegment::Land => {
				let land = self.landing_squat(lengths);
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
						"jump land start: elapsed={:.3} cycle_t={:.3} land_local={:.4} land_depth={:.4} land_femur={:.4} transition={:.4} land_desc_d={:.4} y={:.4}",
						elapsed,
						self.time_in_cycle(lengths, elapsed),
						local,
						land.depth(land_progress),
						land.femur_swing(land_progress),
						transition_progress,
						timings.land_descent_duration,
						self.vertical_offset(lengths, elapsed),
					);
				}
				if transition_progress < 1.0 {
					let from_pose =
						capture_animation_pose(&Fall::<R>::default(), rig, 1.0);
					Transition::from_pose(land, from_pose)
						.with_curve(TransitionCurve::SmoothStep)
						.apply(rig, land_progress, transition_progress);
				} else {
					land.apply(rig, land_progress);
				}
			}
		}

		let y = self.vertical_offset(lengths, elapsed);
		Effects {
			r#move: (y.abs() > f32::EPSILON)
				.then(|| Transform::from_translation(Vec3::new(0.0, y, 0.0))),
		}
	}
}

impl<R: HumanoidRig> TwoFootedJump<R> {
	pub fn log_landing_debug(&self, rig: &R, elapsed: f32, label: &str) {
		let lengths = rig.segment_lengths();
		let timings = self.timings(lengths);
		let time_in_cycle = self.time_in_cycle(lengths, elapsed);
		let (segment, local) = self.segment(lengths, elapsed);
		let land = self.landing_squat(lengths);
		let land_progress = local / timings.land_duration().max(f32::EPSILON);
		let y = self.vertical_offset(lengths, elapsed);

		info!(
			"{label}: elapsed={:.3} cycle_t={:.3} segment={:?} local={:.4} land_depth={:.4} y={:.4} timings[squat=({:.3},{:.3}) spring={:.3} air={:.3} land=({:.4},{:.3})] speeds[pre={:.3} landing={:.3}]",
			elapsed,
			time_in_cycle,
			segment,
			local,
			land.depth(land_progress),
			y,
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
	use crozon_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

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
}
