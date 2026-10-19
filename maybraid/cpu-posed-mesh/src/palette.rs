//! Joint palette: `joint_global.to_matrix() * inverse_bind`.

use bevy::ecs::entity::Entity;
use bevy::math::Mat4;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::prelude::GlobalTransform;

use crate::policy::CollapsedJoint;

/// One palette entry. `None` is a missing influence (unresolved joint).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointSlot {
	/// `joint_global.to_matrix() * inverse_bind`.
	pub skin: Mat4,
	pub collapsed: bool,
}

impl JointSlot {
	pub fn from_world(world: GlobalTransform, bindpose: Mat4, collapsed: CollapsedJoint) -> Self {
		let scale = world.to_scale_rotation_translation().0.max_element();
		let is_collapsed = match collapsed {
			CollapsedJoint::DeadBelow { max_scale } => scale < max_scale,
		};
		Self { skin: world.to_matrix() * bindpose, collapsed: is_collapsed }
	}
}

/// Resolved skin matrices for one [`SkinnedMesh`].
#[derive(Clone, Debug, PartialEq)]
pub struct JointPalette {
	slots: Vec<Option<JointSlot>>,
}

impl JointPalette {
	pub fn from_slots(slots: Vec<Option<JointSlot>>) -> Self {
		Self { slots }
	}

	pub fn from_skin(
		skin: &SkinnedMesh,
		bindposes: &SkinnedMeshInverseBindposes,
		world_of: impl FnMut(Entity) -> Option<GlobalTransform>,
	) -> Self {
		Self::from_skin_with(skin, bindposes, CollapsedJoint::DEFAULT, world_of)
	}

	pub fn from_skin_with(
		skin: &SkinnedMesh,
		bindposes: &SkinnedMeshInverseBindposes,
		collapsed: CollapsedJoint,
		mut world_of: impl FnMut(Entity) -> Option<GlobalTransform>,
	) -> Self {
		let slots = skin
			.joints
			.iter()
			.zip(bindposes.iter())
			.map(|(&joint, bindpose)| {
				world_of(joint).map(|world| JointSlot::from_world(world, *bindpose, collapsed))
			})
			.collect();
		Self { slots }
	}

	pub fn get(&self, index: usize) -> Option<JointSlot> {
		self.slots.get(index).copied().flatten()
	}

	pub fn len(&self) -> usize {
		self.slots.len()
	}

	pub fn is_empty(&self) -> bool {
		self.slots.is_empty()
	}

	pub fn slots(&self) -> &[Option<JointSlot>] {
		&self.slots
	}
}
