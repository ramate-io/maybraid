//! Shared humanoid takeoff / landing routing for [`JumpSample`].

use bevy::prelude::{Quat, Transform, Vec3};
use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{
	smoothstep, JumpSample, JumpSegment, Spring, Squat, TwoFootedJump, FALL_BLEND_FRACTION,
};
use crate::rigs::jump_transition_cache::{JumpTransitionCache, JumpTransitionKey};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects};

impl JumpSample {
	/// Armature offset from this sample: height, plus flip pitch when present.
	pub fn effects(self) -> Effects {
		let y = self.height;
		let pitch = self.flip_pitch.unwrap_or(0.0);
		if y.abs() <= f32::EPSILON && pitch.abs() <= f32::EPSILON {
			return ArmatureOffset::IDENTITY;
		}
		if pitch.abs() <= f32::EPSILON {
			return ArmatureOffset::from_translation(Vec3::new(0.0, y, 0.0));
		}
		ArmatureOffset(Transform {
			translation: Vec3::new(0.0, y, 0.0),
			rotation: Quat::from_rotation_x(pitch),
			scale: Vec3::ONE,
		})
	}

	/// Shared squat / spring / land routing. `air` is fall spread or tuck.
	pub fn apply_humanoid_pose<Air, LandFrom>(
		self,
		rig: &mut HumanoidV0Rig,
		jump: &TwoFootedJump,
		air: &Air,
		air_progress: f32,
		land_from: &LandFrom,
		land_from_progress: f32,
	) where
		Air: Animation<HumanoidV0Rig>,
		LandFrom: Animation<HumanoidV0Rig>,
	{
		self.apply_humanoid_pose_with_cache(
			rig,
			jump,
			air,
			air_progress,
			land_from,
			land_from_progress,
			None,
		);
	}

	pub(crate) fn apply_humanoid_pose_with_cache<Air, LandFrom>(
		self,
		rig: &mut HumanoidV0Rig,
		jump: &TwoFootedJump,
		air: &Air,
		air_progress: f32,
		land_from: &LandFrom,
		land_from_progress: f32,
		mut cache: Option<&mut JumpTransitionCache>,
	) where
		Air: Animation<HumanoidV0Rig>,
		LandFrom: Animation<HumanoidV0Rig>,
	{
		let lengths = rig.segment_lengths;
		let local = self.local;
		let timings = &self.timings;
		match self.segment {
			JumpSegment::Squat => {
				let squat = jump.prejump_squat(lengths);
				let progress = local / timings.squat_duration().max(f32::EPSILON);
				squat.apply_for(rig, progress);
			}
			JumpSegment::Spring => {
				blend_jump_clips(
					cache.as_deref_mut(),
					JumpTransitionKey::SpringFromSquat,
					rig,
					&Squat::for_loop(1.0, 1.0),
					0.0,
					&Spring::default(),
					local,
					smoothstep(local),
				);
			}
			JumpSegment::Fall => {
				let blend_end = FALL_BLEND_FRACTION;
				if local < blend_end {
					let transition_progress = (local / blend_end).clamp(0.0, 1.0);
					blend_jump_clips(
						cache.as_deref_mut(),
						JumpTransitionKey::FallFromSpring,
						rig,
						&Spring::default(),
						1.0,
						air,
						air_progress,
						smoothstep(transition_progress),
					);
				} else {
					air.apply_for(rig, air_progress);
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
					blend_jump_clips(
						cache.as_deref_mut(),
						JumpTransitionKey::LandFromFall,
						rig,
						land_from,
						land_from_progress,
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
}

fn blend_jump_clips<A, B>(
	cache: Option<&mut JumpTransitionCache>,
	key: JumpTransitionKey,
	rig: &mut HumanoidV0Rig,
	from: &A,
	from_progress: f32,
	to: &B,
	to_progress: f32,
	weight: f32,
) where
	A: Animation<HumanoidV0Rig>,
	B: Animation<HumanoidV0Rig>,
{
	if let Some(cache) = cache {
		cache.blend(rig, key, from, from_progress, to, to_progress, weight);
	} else {
		blend_clips(rig, from, from_progress, to, to_progress, weight);
	}
}
