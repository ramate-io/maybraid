pub mod humanoid_rig;
pub mod sliders;

use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name(pub String);

/// The orientation of the bone in the rig,
/// relative to the intended geometry.
///
/// Useful when it is discovered that the default assumption is not correctly
/// adhered to in in the rig:
///
/// Default assumption:
/// 1. -Y (Blender) = +Z (Bevy) is forward.
/// 2. +Z (Blender) = +Y (Bevy) is up.
/// 3. +X (Blender) = +X (Bevy) is right.
///
/// NOTE: I believe the below can be reduced from three axes to two,
/// plus a sign.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RiggedAxis {
	pub forward: Vec3,
	pub up: Vec3,
	pub right: Vec3,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Bone {
	/// Typically used as a constant to reference the loaded bone.
	pub name: Name,
	/// The orientation of the bone in the rig, relative to the intended geometry.
	///
	/// Often this will just be the default.
	pub relative_axis: RiggedAxis,
	/// The transform of the rig needed to achieve a particular articulation.
	pub transform: Transform,
}

#[derive(Debug, Clone)]
pub struct BoneTable(HashMap<Name, Bone>);

impl BoneTable {
	pub fn new() -> Self {
		Self(HashMap::new())
	}

	pub fn insert(&mut self, bone: Bone) {
		self.0.insert(bone.name, bone);
	}

	pub fn get(&self, name: &Name) -> Option<&Bone> {
		self.0.get(name)
	}

	pub fn get_mut(&mut self, name: &Name) -> Option<&mut Bone> {
		self.0.get_mut(name)
	}

	pub fn remove(&mut self, name: &Name) {
		self.0.remove(name);
	}

	pub fn iter(&self) -> impl Iterator<Item = &Bone> {
		self.0.values()
	}

	pub fn len(&self) -> usize {
		self.0.len()
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

/// A section of the rig that
/// is intended to be used symmetrically across some set of axes.
///
/// This is intended as a semantic helper for users of the rig,
/// it suggests symmetry lines.
#[derive(Debug, Clone)]
pub struct Symmetry(pub Vec3);

#[derive(Debug, Clone)]
pub struct SymmetryTable(HashMap<Name, Symmetry>);
