//! Per-jump playback cache for transition source poses.
//!
//! Jump segments blend from fixed animation endpoints (Squat@0, Spring@1, Fall@1). Those
//! endpoints only write a subset of bones; untouched bones are taken from the incoming
//! rig each frame. Caching stores only bones the source animation changes so unrelated
//! motion on other bones is not frozen.

use character_rigs::{humanoid::HumanoidRig, Name, RigPose};

use crate::animations::JumpSegment;
use crate::rigs::mix::{restore_pose, sample_pose, snapshot_pose};
use crate::Animation;

/// Offset applied when probing whether an animation overwrites a bone independent of rest.
const PROBE_DELTA: f32 = 0.37;

/// Which jump segment transition source is cached.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JumpTransitionKey {
	SpringFromSquat,
	FallFromSpring,
	LandFromFall,
}

/// Source pose for a transition, retaining only bones written by the capture animation.
#[derive(Debug, Clone, Default)]
pub struct MaskedTransitionSource {
	bones: RigPose,
}

impl MaskedTransitionSource {
	/// Capture an animation endpoint, caching only bones the animation overwrites.
	///
	/// Bones the animation leaves rest-dependent (untouched or layered) are omitted so
	/// [`Self::merge_into_current`] can keep following the live rig each frame.
	pub fn capture<A, R>(anim: &A, rig: &mut R, progress: f32) -> Self
	where
		A: Animation<R>,
		R: HumanoidRig,
	{
		let rest = snapshot_pose(rig);
		let probe = probe_rest(&rest);
		let from_rest = sample_pose(anim, rig, &rest, progress);
		let from_probe = sample_pose(anim, rig, &probe, progress);
		restore_pose(rig, &rest);

		let mut bones = RigPose::new();
		for bone in rig.animation_bones() {
			let at_rest = from_rest.get(&bone);
			let at_probe = from_probe.get(&bone);
			if at_rest == at_probe {
				if let Some(pose) = at_rest {
					bones.insert(pose.clone());
				}
			}
		}
		Self { bones }
	}

	/// Bones written by the source animation at capture time.
	pub fn masked_bones(&self) -> impl Iterator<Item = &Name> {
		self.bones.iter().map(|(name, _)| name)
	}

	/// Build a full from-pose: cached source bones plus the current rig elsewhere.
	pub fn merge_into_current<R: HumanoidRig>(&self, rig: &R) -> RigPose {
		let mut merged = snapshot_pose(rig);
		for (_, pose) in self.bones.iter() {
			merged.insert(pose.clone());
		}
		merged
	}
}

fn probe_rest(rest: &RigPose) -> RigPose {
	let mut probe = RigPose::new();
	for (name, bone) in rest.iter() {
		probe.insert(character_rigs::BonePose {
			name: name.clone(),
			transform: bone.transform,
			swing: bone.swing + PROBE_DELTA,
			flex: bone.flex + PROBE_DELTA,
			twist: bone.twist + PROBE_DELTA,
		});
	}
	probe
}

/// Per-character cache for jump transition sources. Invalidate on segment change rules below.
#[derive(Debug, Clone, Default)]
pub struct JumpTransitionCache {
	spring_from: Option<MaskedTransitionSource>,
	fall_from: Option<MaskedTransitionSource>,
	land_from: Option<MaskedTransitionSource>,
	last_elapsed: f32,
}

impl JumpTransitionCache {
	/// Drop all cached transition sources (clip change, jump restart, params change, rig reset).
	pub fn invalidate(&mut self) {
		*self = Self::default();
	}

	/// Detect jump cycle restarts when elapsed time moves backwards.
	pub fn note_elapsed(&mut self, elapsed: f32) {
		if self.last_elapsed > f32::EPSILON && elapsed + 1e-4 < self.last_elapsed {
			self.invalidate();
		}
		self.last_elapsed = elapsed;
	}

	pub fn get_or_capture<R>(
		&mut self,
		key: JumpTransitionKey,
		rig: &mut R,
		capture: impl FnOnce(&mut R) -> MaskedTransitionSource,
	) -> &MaskedTransitionSource
	where
		R: HumanoidRig,
	{
		let slot = match key {
			JumpTransitionKey::SpringFromSquat => &mut self.spring_from,
			JumpTransitionKey::FallFromSpring => &mut self.fall_from,
			JumpTransitionKey::LandFromFall => &mut self.land_from,
		};
		if slot.is_none() {
			*slot = Some(capture(rig));
		}
		slot.as_ref().expect("transition source just cached")
	}

	/// Clear cached sources past their blend window so a later cycle can recapture.
	pub fn clear_after_segment(&mut self, segment: JumpSegment) {
		match segment {
			JumpSegment::Squat => {}
			JumpSegment::Spring => self.spring_from = None,
			JumpSegment::Fall => self.fall_from = None,
			JumpSegment::Land => self.land_from = None,
		}
	}
}

#[cfg(test)]
mod tests {
	use character_rigs::{rigs::humanoid_v0::HumanoidV0Rig, Side};

	use super::*;
	use crate::animations::{Fall, Squat};
	use crate::rigs::mix::{pose_from_animation, seed_bind_pose};

	#[test]
	fn masked_capture_omits_unrelated_bones() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		let shoulder_name = rig.arm(Side::Left).shoulder.name.clone();
		let shoulder = rig.pose().get(&shoulder_name).expect("shoulder").clone();
		rig.pose_mut().insert(character_rigs::BonePose {
			name: shoulder.name.clone(),
			transform: shoulder.transform,
			swing: shoulder.swing + 0.3,
			flex: shoulder.flex + 0.2,
			twist: shoulder.twist,
		});

		let masked = MaskedTransitionSource::capture(
			&Squat::<HumanoidV0Rig>::for_loop(1.0, 1.0),
			&mut rig,
			0.0,
		);
		assert!(
			!masked.masked_bones().any(|name| *name == shoulder_name),
			"squat@0 should not mask unrelated shoulder motion"
		);
		assert!(masked.masked_bones().any(|name| *name == rig.leg(Side::Left).femur.name));
		Ok(())
	}

	#[test]
	fn merge_into_current_tracks_live_unrelated_bones() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		let masked =
			MaskedTransitionSource::capture(&Fall::<HumanoidV0Rig>::default(), &mut rig, 1.0);
		let root_name = rig.spine().root.name.clone();
		assert!(
			!masked.masked_bones().any(|name| *name == root_name),
			"fall spread should not mask the root"
		);
		let root = rig.pose().get(&root_name).expect("root").clone();
		let live_swing = root.swing + 0.4;
		rig.pose_mut().insert(character_rigs::BonePose {
			name: root.name.clone(),
			transform: root.transform,
			swing: live_swing,
			flex: root.flex,
			twist: root.twist,
		});

		let merged = masked.merge_into_current(&rig);
		let merged_root = merged.get(&root_name).expect("merged root");
		assert!((merged_root.swing - live_swing).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn cache_invalidates_on_backward_elapsed() -> anyhow::Result<()> {
		let mut cache = JumpTransitionCache::default();
		cache.note_elapsed(1.0);
		cache.spring_from = Some(MaskedTransitionSource::default());
		cache.note_elapsed(0.5);
		assert!(cache.spring_from.is_none());
		Ok(())
	}

	#[test]
	fn masked_capture_matches_per_frame_for_fall_endpoint() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		seed_bind_pose(&mut rig);
		Fall::<HumanoidV0Rig>::default().apply(&mut rig, 0.5);
		let per_frame = pose_from_animation(&Fall::<HumanoidV0Rig>::default(), &mut rig, 1.0);
		let masked =
			MaskedTransitionSource::capture(&Fall::<HumanoidV0Rig>::default(), &mut rig, 1.0);
		let merged = masked.merge_into_current(&rig);

		for bone in rig.animation_bones() {
			let expected = per_frame.get(&bone).or_else(|| rig.pose().get(&bone));
			let actual = merged.get(&bone);
			if let (Some(e), Some(a)) = (expected, actual) {
				assert!((e.swing - a.swing).abs() < 1e-5, "bone {:?}", bone);
			}
		}
		Ok(())
	}
}
