use bevy::prelude::*;

use crate::{BonePose, Name, RigPose, RiggedAxis, Side};

/// Axial chain from cranial mid-spine through the caudal peduncle.
#[derive(Debug, Clone, PartialEq)]
pub struct ForelimbedSpine {
	pub upper_mid_spine: BonePose,
	pub upper_spine: BonePose,
	pub lower_mid_spine: BonePose,
	pub lower_spine: BonePose,
	pub tailbone: BonePose,
	pub back_ridge: BonePose,
}

impl ForelimbedSpine {
	pub fn hydrate_from(&mut self, pose: &RigPose) {
		hydrate_bone(pose, &mut self.upper_mid_spine);
		hydrate_bone(pose, &mut self.upper_spine);
		hydrate_bone(pose, &mut self.lower_mid_spine);
		hydrate_bone(pose, &mut self.lower_spine);
		hydrate_bone(pose, &mut self.tailbone);
		hydrate_bone(pose, &mut self.back_ridge);
	}

	pub fn apply_to(&self, pose: &mut RigPose) {
		pose.insert(self.upper_mid_spine.clone());
		pose.insert(self.upper_spine.clone());
		pose.insert(self.lower_mid_spine.clone());
		pose.insert(self.lower_spine.clone());
		pose.insert(self.tailbone.clone());
		pose.insert(self.back_ridge.clone());
	}
}

/// Pectoral fin / forelimb chain (shoulder → upper arm → lower arm).
#[derive(Debug, Clone, PartialEq)]
pub struct ForelimbedFin {
	pub shoulder: BonePose,
	pub upper_arm: BonePose,
	pub lower_arm: BonePose,
}

impl ForelimbedFin {
	pub fn hydrate_from(&mut self, pose: &RigPose) {
		hydrate_bone(pose, &mut self.shoulder);
		hydrate_bone(pose, &mut self.upper_arm);
		hydrate_bone(pose, &mut self.lower_arm);
	}

	pub fn apply_to(&self, pose: &mut RigPose) {
		pose.insert(self.shoulder.clone());
		pose.insert(self.upper_arm.clone());
		pose.insert(self.lower_arm.clone());
	}
}

fn hydrate_bone(pose: &RigPose, bone: &mut BonePose) {
	if let Some(stored) = pose.get(&bone.name) {
		bone.transform = stored.transform;
		bone.swing = stored.swing;
		bone.flex = stored.flex;
		bone.twist = stored.twist;
	}
}

/// Procedural articulation surface for the shared forelimbed (fish / aquatic) rig.
pub trait ForelimbedRig {
	fn spine(&self) -> ForelimbedSpine;
	fn fin(&self, side: Side) -> ForelimbedFin;
	fn pose(&self) -> &RigPose;
	fn pose_mut(&mut self) -> &mut RigPose;

	fn animation_bones(&self) -> Vec<Name>;

	fn rigged_axis(&self, _bone: &Name) -> Option<RiggedAxis> {
		None
	}

	fn articulate_on_rig(&self, bone: BonePose, swing: f32, flex: f32) -> BonePose {
		self.articulate_on_rig_twisted(bone, swing, flex, 0.0)
	}

	fn articulate_on_rig_twisted(
		&self,
		mut bone: BonePose,
		swing: f32,
		flex: f32,
		twist: f32,
	) -> BonePose {
		if let Some(axis) = self.rigged_axis(&bone.name) {
			bone = bone.articulate(axis, swing, flex, twist);
		} else {
			bone.swing = swing;
			bone.flex = flex;
			bone.twist = twist;
		}
		bone
	}

	fn spine_pose(&self) -> ForelimbedSpine {
		let mut spine = self.spine();
		spine.hydrate_from(self.pose());
		spine
	}

	fn fin_pose(&self, side: Side) -> ForelimbedFin {
		let mut fin = self.fin(side);
		fin.hydrate_from(self.pose());
		fin
	}

	fn pose_spine(&mut self, spine: ForelimbedSpine) {
		spine.apply_to(self.pose_mut());
	}

	fn pose_fin(&mut self, fin: ForelimbedFin) {
		fin.apply_to(self.pose_mut());
	}

	fn parent_world_rotation(&self, _bone: &Name) -> Quat {
		Quat::IDENTITY
	}

	fn move_all(&mut self, world_displacement: Vec3) {
		let bones = self.animation_bones();
		for bone in bones {
			let parent_world = self.parent_world_rotation(&bone);
			let axis = self.rigged_axis(&bone).unwrap_or(RiggedAxis::DEFAULT);
			let Some(pose) = self.pose_mut().get_mut(&bone) else {
				continue;
			};
			let segment = pose.transform.translation;
			let delta = crate::articulation::axis_aware_translation_delta(
				segment,
				axis,
				world_displacement,
				parent_world,
			);
			pose.transform.translation += delta;
		}
	}
}
