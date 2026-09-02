//! Named-bone index for one armature, scoped to that [`RigRoot`].

use std::collections::HashMap;

use bevy::prelude::*;

use crate::member::AssemblyHost;

/// Opaque identity for one armature inside an assembly.
///
/// Domain crates choose the strings (`"body"`, `"receiver"`, …). This crate
/// only stores and compares them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct RigKey(pub &'static str);

impl RigKey {
	pub const fn named(name: &'static str) -> Self {
		Self(name)
	}

	pub const fn as_str(self) -> &'static str {
		self.0
	}
}

impl From<&'static str> for RigKey {
	fn from(value: &'static str) -> Self {
		Self(value)
	}
}

/// Marker: this entity owns a [`BoneMap`] over its named descendants.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RigRoot {
	pub key: RigKey,
	/// Bones that must exist before the map is considered ready. Empty = always ready.
	pub landmarks: &'static [&'static str],
}

impl RigRoot {
	pub const fn new(key: RigKey) -> Self {
		Self { key, landmarks: &[] }
	}

	pub const fn with_landmarks(self, landmarks: &'static [&'static str]) -> Self {
		Self { key: self.key, landmarks }
	}
}

/// Named-bone index for one [`RigRoot`] (scoped to that armature).
#[derive(Component, Default, Clone)]
pub struct BoneMap {
	pub by_name: HashMap<String, Entity>,
}

pub fn bone_map_ready(map: &BoneMap, landmarks: &[&str]) -> bool {
	landmarks.iter().all(|bone| map.by_name.contains_key(*bone))
}

pub fn missing_landmark_bones<'a>(map: &BoneMap, landmarks: &'a [&'a str]) -> Vec<&'a str> {
	landmarks
		.iter()
		.copied()
		.filter(|bone| !map.by_name.contains_key(*bone))
		.collect()
}

/// Rebuild each rig's [`BoneMap`] from named descendants, stopping at nested
/// [`AssemblyHost`]s (nested rigs and parts).
pub fn build_bone_maps(
	mut rig_roots: Query<(Entity, &Children, &mut BoneMap), With<RigRoot>>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
	hosts: Query<(), With<AssemblyHost>>,
) {
	for (_rig_root, children, mut map) in &mut rig_roots {
		map.by_name.clear();
		let mut stack: Vec<Entity> = children.iter().collect();
		while let Some(entity) = stack.pop() {
			if hosts.contains(entity) {
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty_landmarks_are_ready() {
		assert!(bone_map_ready(&BoneMap::default(), &[]));
	}

	#[test]
	fn missing_landmarks_are_listed() {
		let map = BoneMap::default();
		assert_eq!(missing_landmark_bones(&map, &["root", "grip"]), vec!["root", "grip"]);
	}
}
