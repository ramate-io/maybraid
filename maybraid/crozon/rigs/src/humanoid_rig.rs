use super::{BoneTable, SymmetryTable};
use bevy::prelude::*;

/// Store the bones of a rig in a semantically reasonable hierarchy.
///
/// This will be expanded upon to give a useful semantic interface
/// for programming sizing and animations.
#[derive(Component, Debug, Clone)]
pub struct HumanoidRig {
	pub bones: BoneTable,
	pub symmetries: SymmetryTable,
}
