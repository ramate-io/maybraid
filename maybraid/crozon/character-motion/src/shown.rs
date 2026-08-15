//! Resolve the currently shown [`LodLevelRoot`] under a host.

use bevy::prelude::*;
use lod::{lod_root_is_shown, LodLevelRoot, LodLevelRoots};

/// Shown [`LodLevelRoot`] under `host`, if any.
pub fn shown_level_root(
	host: Entity,
	children_q: &Query<&Children>,
	level_roots_bags: &Query<(), With<LodLevelRoots>>,
	root_keys: &Query<&LodLevelRoot>,
	visibilities: &Query<&Visibility>,
) -> Option<Entity> {
	let host_kids = children_q.get(host).ok()?;
	for kid in host_kids.iter() {
		if !level_roots_bags.contains(kid) {
			continue;
		}
		let Ok(root_kids) = children_q.get(kid) else {
			continue;
		};
		for root_e in root_kids.iter() {
			if root_keys.get(root_e).is_err() {
				continue;
			}
			let Ok(vis) = visibilities.get(root_e) else {
				continue;
			};
			if lod_root_is_shown(*vis) {
				return Some(root_e);
			}
		}
	}
	None
}

/// Whether the shown level (or `host` when no level exists yet) carries `M`.
///
/// Markers are stamped on the level-root **content** child (`scene_with_level`),
/// so this walks a few descendants.
pub fn shown_level_has<M: Component>(
	host: Entity,
	children_q: &Query<&Children>,
	level_roots_bags: &Query<(), With<LodLevelRoots>>,
	root_keys: &Query<&LodLevelRoot>,
	visibilities: &Query<&Visibility>,
	markers: &Query<(), With<M>>,
) -> bool {
	match shown_level_root(host, children_q, level_roots_bags, root_keys, visibilities) {
		Some(root) => subtree_has_marker(root, children_q, markers, 3),
		None => markers.contains(host),
	}
}

fn subtree_has_marker<M: Component>(
	entity: Entity,
	children_q: &Query<&Children>,
	markers: &Query<(), With<M>>,
	depth: u32,
) -> bool {
	if markers.contains(entity) {
		return true;
	}
	if depth == 0 {
		return false;
	}
	let Ok(kids) = children_q.get(entity) else {
		return false;
	};
	kids.iter().any(|kid| subtree_has_marker(kid, children_q, markers, depth - 1))
}
