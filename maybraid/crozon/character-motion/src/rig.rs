//! Rig identity the mailbox and pitch math need. Not species recipes.

use std::collections::HashMap;

use bevy::prelude::*;

/// Skeleton family used to gate bone-map readiness and pitch weights.
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

/// Named-bone index for one [`CharacterRig`] (scoped to that armature).
#[derive(Component, Default, Clone)]
pub struct BoneMap {
	pub by_name: HashMap<String, Entity>,
}

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
