//! Spawn missing level roots from [`LodLevelSpawnRequest`].

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};

use crate::lod_ref::{
	collect_node_snapshots, lod_refs_for_bounds, LodNode, LodNodePose,
};
use crate::scene::host::{
	LodLevelRoot, LodLevelRoots, LodLevelSpawnRequest, LodSceneHost,
};
use crate::scene::LodScene;

use super::bounds::{ephemeral_bounds, LodHostBounds};
use super::update::dominant_lod_ref;

/// Spawn a missing level root under [`LodLevelRoots`], then clear the request.
///
/// Uses the dominant [`crate::lod_ref::LodRef`] among `FNode`-filtered [`LodNode`]s for
/// [`LodScene::scene_with_level`].
pub fn fulfill_lod_level_spawn<T, FHost, FNode>(
	mut commands: Commands,
	nodes: Query<(Entity, &LodNodePose), (With<LodNode>, FNode)>,
	hosts: Query<
		(Entity, &T, Option<&LodHostBounds>, &LodLevelSpawnRequest, &Children),
		(With<LodSceneHost>, FHost),
	>,
	level_roots_heads: Query<(Entity, Option<&Children>), With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
) where
	T: Component + LodScene,
	FHost: QueryFilter + 'static,
	FNode: QueryFilter + 'static,
{
	let snapshots = collect_node_snapshots(&nodes);
	if snapshots.is_empty() {
		return;
	}

	for (host, scene, host_bounds, request, host_children) in &hosts {
		let bounds = ephemeral_bounds(host_bounds);
		let refs = lod_refs_for_bounds(&snapshots, &bounds);
		let Some(lod_ref) = dominant_lod_ref(scene, &refs) else {
			continue;
		};

		let mut roots_entity = None;
		for child in host_children.iter() {
			if level_roots_heads.contains(child) {
				roots_entity = Some(child);
				break;
			}
		}

		let Some(roots_entity) = roots_entity else {
			commands.entity(host).remove::<LodLevelSpawnRequest>();
			continue;
		};

		if let Ok((_, Some(root_children))) = level_roots_heads.get(roots_entity) {
			let mut already_present = false;
			for child in root_children.iter() {
				if root_keys.get(child).is_ok_and(|root| root.0 == request.level) {
					already_present = true;
					break;
				}
			}
			if already_present {
				commands.entity(host).remove::<LodLevelSpawnRequest>();
				continue;
			}
			for child in root_children.iter() {
				commands.entity(child).insert(Visibility::Hidden);
			}
		}

		let content: Box<dyn bevy::scene::Scene> =
			Box::new(scene.scene_with_level(lod_ref, request.level));
		let children = vec![content];
		let level = request.level;
		let level_root = bsn! {
			template_value(LodLevelRoot(level))
			Transform::default()
			Visibility::Inherited
			Children [ {children} ]
		};
		let child = commands.spawn_scene(level_root).id();
		commands.entity(roots_entity).add_child(child);
		commands.entity(host).remove::<LodLevelSpawnRequest>();
	}
}
