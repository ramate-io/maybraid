//! Runtime rig markers shared by LodScene hosts and preview systems.

use std::collections::HashMap;

use bevy::prelude::*;
use crozon_rigs::ResolvedRigPose;

use crate::assembly::CharacterPartSlot;

/// Skeleton family used to gate bone-map readiness.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RigSkeletonKind {
	#[default]
	Humanoid,
	Quadruped,
	Forelimbed,
	/// Multi-bone neck armature (`neck_base` … `head_socket`).
	Neck,
}

impl RigSkeletonKind {
	pub fn from_body_rig_label(label: &str) -> Self {
		match label {
			"Quadruped" => Self::Quadruped,
			"Forelimbed" => Self::Forelimbed,
			_ => Self::Humanoid,
		}
	}

	pub fn landmark_bones(self) -> &'static [&'static str] {
		match self {
			Self::Humanoid => &["root", "pelvis.L", "chest.L", "waist.L"],
			Self::Quadruped => &["head_socket", "shoulder.L", "tailbone", "waist.L"],
			Self::Forelimbed => &["head_socket", "shoulder.L", "tailbone", "upper_mid_spine"],
			Self::Neck => &["neck_base", "head_socket"],
		}
	}
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CharacterRigRole {
	#[default]
	Body,
	Neck,
	Head,
}

#[derive(Component, Clone, Copy, Default)]
pub struct CharacterRig {
	pub role: CharacterRigRole,
	pub skeleton: RigSkeletonKind,
}

/// Marks a [`CharacterRig`] that came from a [`crate::RigNode`] LodScene host.
#[derive(Component, Clone, Copy, Default)]
pub struct LodCharacterRig;

/// Marks which logical character slot an instantiated part occupies.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CharacterPart {
	pub slot: CharacterPartSlot,
}

#[derive(Component)]
pub struct PartRigRef {
	pub rig_root: Entity,
}

#[derive(Component)]
pub struct NeedsSkinRemap;

#[derive(Component)]
pub struct NeedsDuplicateScenePrune {
	pub keep: Vec<Entity>,
}

/// Part mesh was skinned to a skeleton that does not match the active rig.
#[derive(Component, Debug)]
pub struct NoMatchingArmature {
	pub missing_joints: Vec<String>,
}

/// Named-bone index for one [`CharacterRig`] (scoped to that armature).
#[derive(Component, Default, Clone)]
pub struct BoneMap {
	pub by_name: HashMap<String, Entity>,
}

/// Resolved proportional layers to maintain on this rig across frames.
#[derive(Component, Clone, Default)]
pub struct ActiveRigPose {
	pub pose: ResolvedRigPose,
}

/// Bind-pose bone TRS captured once each named bone appears in the rig map.
#[derive(Component, Default, Clone)]
pub struct RigBindScales {
	pub scales: HashMap<String, Vec3>,
	pub translations: HashMap<String, Vec3>,
	pub rotations: HashMap<String, Quat>,
}

/// Inserted the first frame [`ActiveRigPose`] is applied to a ready rig.
#[derive(Component)]
pub struct ResolvedPoseApplied;

pub fn bone_map_ready(map: &BoneMap, skeleton: RigSkeletonKind) -> bool {
	skeleton.landmark_bones().iter().all(|bone| map.by_name.contains_key(*bone))
}

pub fn missing_landmark_bones(map: &BoneMap, skeleton: RigSkeletonKind) -> Vec<&'static str> {
	skeleton
		.landmark_bones()
		.iter()
		.copied()
		.filter(|bone| !map.by_name.contains_key(*bone))
		.collect()
}

pub fn bind_scales_ready(
	bind_scales: &RigBindScales,
	bone_map: &BoneMap,
	skeleton: RigSkeletonKind,
) -> bool {
	skeleton
		.landmark_bones()
		.iter()
		.all(|bone| bind_scales.scales.contains_key(*bone) && bone_map.by_name.contains_key(*bone))
}

/// Rebuild each rig's [`BoneMap`] from named descendants, stopping at nested
/// [`CharacterRig`] / [`CharacterPart`] boundaries.
pub fn build_rig_bone_map(
	mut rig_roots: Query<(Entity, &Children, &mut BoneMap), With<CharacterRig>>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
	boundaries: Query<(), Or<(With<CharacterRig>, With<CharacterPart>)>>,
) {
	for (_rig_root, children, mut map) in &mut rig_roots {
		map.by_name.clear();

		let mut stack: Vec<Entity> = children.iter().collect();
		while let Some(entity) = stack.pop() {
			if boundaries.contains(entity) {
				continue;
			}
			if let Ok(name) = names_q.get(entity) {
				map.by_name.insert(name.to_string(), entity);
			}
			if let Ok(children) = children_q.get(entity) {
				stack.extend(children.iter());
			}
		}
	}
}
