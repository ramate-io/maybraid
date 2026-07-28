//! Fulfill lazy [`LodLevelSpawnRequest`]s for [`WizardsTower`] hosts.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;
use lod::lod_scene_host::{LodLevelRoot, LodLevelRoots, LodLevelSpawnRequest};
use lod::{LodSceneHost, LodSceneLevel};

use crate::wizards_tower::tower_lod::TowerLodFootprint;
use crate::wizards_tower::WizardsTower;

/// Spawn a missing tower level root under [`LodLevelRoots`], then clear the request.
pub fn fulfill_tower_lod_spawn(
	mut commands: Commands,
	camera: Query<(Entity, &Transform), With<Camera3d>>,
	hosts: Query<(Entity, &WizardsTower, &LodLevelSpawnRequest, &Children)>,
	level_roots_heads: Query<(Entity, Option<&Children>), With<LodLevelRoots>>,
) {
	let Ok((cam_entity, cam_tf)) = camera.single() else {
		return;
	};

	for (host, tower, request, host_children) in &hosts {
		let bounds = &tower.constraints.aabb;
		let lod_ref = LodRef {
			entity: cam_entity,
			previous_transform: cam_tf,
			current_transform: cam_tf,
			bounds,
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
			for child in root_children.iter() {
				commands.entity(child).insert(Visibility::Hidden);
			}
		}

		let content: Box<dyn bevy::scene::Scene> =
			Box::new(tower.scene_with_level(&lod_ref, request.level));
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

/// Fine-phase: set tower host [`LodSceneLevel`] from camera pose.
pub fn update_tower_host_levels(
	camera: Query<&Transform, With<Camera3d>>,
	mut hosts: Query<(&WizardsTower, &mut LodSceneLevel), With<LodSceneHost>>,
) {
	let Ok(viewer) = camera.single() else {
		return;
	};
	for (tower, mut level) in &mut hosts {
		let desired = tower.level_for(viewer);
		if *level != desired {
			*level = desired;
		}
	}
}
