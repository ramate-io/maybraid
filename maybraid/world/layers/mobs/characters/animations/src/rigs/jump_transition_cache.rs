//! Reuse fixed jump endpoint poses across live mailbox samples.
//!
//! Humanoid V0 samplers resolve a complete pose from binding rest. The cache
//! therefore stores complete PoseBuffers, replacing the old partial RigPose mask.

use std::sync::Arc;

use character_rigs::authoring::{PoseBuffer, RigDefinition};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::Animation;

#[derive(Debug, Clone, Copy)]
pub(crate) enum JumpTransitionKey {
	SpringFromSquat,
	FallFromSpring,
	LandFromFall,
}

#[derive(Debug, Clone, Default)]
pub struct JumpTransitionCache {
	sources: [Option<PoseBuffer>; 3],
	rest: Option<PoseBuffer>,
	definition: Option<Arc<RigDefinition>>,
	last_elapsed: Option<f32>,
}

impl JumpTransitionCache {
	pub fn invalidate(&mut self) {
		*self = Self::default();
	}

	pub(crate) fn prepare(&mut self, rig: &HumanoidV0Rig, elapsed: f32) {
		let same_definition = self
			.definition
			.as_ref()
			.is_some_and(|definition| Arc::ptr_eq(definition, &rig.binding.definition));
		if !same_definition
			|| self.rest.as_ref() != Some(&rig.binding.effective_rest)
			|| self.last_elapsed.is_some_and(|previous| elapsed < previous)
		{
			self.invalidate();
			self.rest = Some(rig.binding.effective_rest.clone());
			self.definition = Some(rig.binding.definition.clone());
		}
		self.last_elapsed = Some(elapsed);
	}

	pub(crate) fn blend<A, B>(
		&mut self,
		rig: &mut HumanoidV0Rig,
		key: JumpTransitionKey,
		from: &A,
		from_progress: f32,
		to: &B,
		to_progress: f32,
		weight: f32,
	) where
		A: Animation<HumanoidV0Rig>,
		B: Animation<HumanoidV0Rig>,
	{
		let weight = weight.clamp(0.0, 1.0);
		if weight >= 1.0 {
			to.apply_for(rig, to_progress);
			return;
		}
		let slot = &mut self.sources[key as usize];
		let source = slot.get_or_insert_with(|| {
			from.apply_for(rig, from_progress);
			rig.pose.clone()
		});
		if weight <= 0.0 {
			rig.pose.copy_from(source);
			return;
		}
		// Use the same nesting-safe scratch discipline as blend_clips.
		let depth = rig.scratch.depth;
		rig.scratch.depth = depth + 1;
		rig.scratch.capture_from(depth, source);
		to.apply_for(rig, to_progress);
		rig.scratch.capture_to(depth, &rig.pose);
		rig.scratch.blend_saved(depth, weight, &mut rig.pose);
		rig.scratch.depth = depth;
	}
}

#[cfg(test)]
mod tests {
	use std::cell::Cell;

	use super::*;
	use crate::animations::{Spring, Squat};

	struct CountSource<'a>(&'a Cell<usize>);
	impl Animation<HumanoidV0Rig> for CountSource<'_> {
		fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
			self.0.set(self.0.get() + 1);
			Squat::for_loop(1.0, 1.0).apply_for(rig, progress);
		}
	}

	#[test]
	fn endpoint_reuse_invalidates_on_restart_and_binding_change() -> anyhow::Result<()> {
		let calls = Cell::new(0);
		let mut rig = HumanoidV0Rig::for_clip_test();
		let mut cache = JumpTransitionCache::default();
		for (elapsed, expected) in [(1.0, 1), (1.1, 1), (0.5, 2)] {
			cache.prepare(&rig, elapsed);
			cache.blend(
				&mut rig,
				JumpTransitionKey::SpringFromSquat,
				&CountSource(&calls),
				0.0,
				&Spring::default(),
				0.5,
				0.5,
			);
			assert_eq!(calls.get(), expected);
		}
		rig.seed_rest(
			"root",
			bevy::prelude::Transform::from_scale(bevy::prelude::Vec3::splat(1.2)),
		);
		cache.prepare(&rig, 0.6);
		cache.blend(
			&mut rig,
			JumpTransitionKey::SpringFromSquat,
			&CountSource(&calls),
			0.0,
			&Spring::default(),
			0.5,
			0.5,
		);
		assert_eq!(calls.get(), 3);
		Ok(())
	}
}
