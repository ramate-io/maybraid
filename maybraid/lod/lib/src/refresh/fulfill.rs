//! Spawn missing level roots from [`LodLevelSpawnRequest`].

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};

use crate::gen::LodScene;
use crate::lod_scene_host::{
	LodLevelRoot, LodLevelRoots, LodLevelSpawnRequest, LodSceneHost,
};

use super::bounds::{ephemeral_bounds, LodHostBounds};
use super::viewer::LodViewerState;

/// Spawn a missing level root under [`LodLevelRoots`], then clear the request.
///
/// Culls only despawn; this is what brings a band back when
/// [`crate::sync_lod_level_roots`] inserts [`LodLevelSpawnRequest`].
pub fn fulfill_lod_level_spawn<T: Component + LodScene, F: QueryFilter + 'static>(
	mut commands: Commands,
	viewer: Res<LodViewerState>,
	hosts: Query<
		(Entity, &T, Option<&LodHostBounds>, &LodLevelSpawnRequest, &Children),
		(With<LodSceneHost>, F),
	>,
	level_roots_heads: Query<(Entity, Option<&Children>), With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
) {
	if viewer.entity == Entity::PLACEHOLDER {
		return;
	}

	for (host, scene, host_bounds, request, host_children) in &hosts {
		let bounds = ephemeral_bounds(host_bounds);
		let lod_ref = viewer.lod_ref(&bounds);

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
			Box::new(scene.scene_with_level(&lod_ref, request.level));
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
