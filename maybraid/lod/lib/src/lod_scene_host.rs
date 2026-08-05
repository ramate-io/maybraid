//! Runtime ECS hosts that switch LOD level roots without despawning the host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use crate::lod_level::LodSceneLevel;

/// Marker: this entity owns LOD level roots and a current [`LodSceneLevel`].
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodSceneHost;

/// Parent of level-root children (keeps level variants out of the structural child bag).
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodLevelRoots;

/// One spawned LOD variant under [`LodLevelRoots`] (keyed by [`LodSceneLevel`]).
#[derive(Debug, Clone, Copy, Component, Default)]
pub struct LodLevelRoot(pub LodSceneLevel);

/// Request that a missing level root be spawned for this host.
#[derive(Debug, Clone, Copy, Component)]
pub struct LodLevelSpawnRequest {
	pub level: LodSceneLevel,
}

/// Build an initial host scene with a single active level root (lazy further levels).
pub fn lod_host_scene(
	level: LodSceneLevel,
	bounds: Aabb3d,
	content: impl Scene + 'static,
) -> impl Scene + 'static {
	let content_children: Vec<Box<dyn Scene>> = vec![Box::new(content)];
	let level_root: Box<dyn Scene> = Box::new(bsn! {
		template_value(LodLevelRoot(level))
		Transform::default()
		Visibility::Inherited
		Children [ {content_children} ]
	});
	let level_roots_children: Vec<Box<dyn Scene>> = vec![level_root];
	let roots: Box<dyn Scene> = Box::new(bsn! {
		LodLevelRoots
		Transform::default()
		Visibility::Inherited
		Children [ {level_roots_children} ]
	});
	let host_children: Vec<Box<dyn Scene>> = vec![roots];
	let host_bounds = crate::fine_pass::LodHostBounds(bounds);
	bsn! {
		LodSceneHost
		template_value(level)
		template_value(host_bounds)
		Transform::default()
		Visibility::Inherited
		Children [ {host_children} ]
	}
}

/// After [`LodSceneLevel`] changes on a host: request missing roots and flip visibility.
///
/// Flow per host:
/// 1. Find the [`LodLevelRoots`] child (or request a spawn if the host/roots bag is missing).
/// 2. Show the root matching the desired level; hide the rest.
/// 3. If no matching root exists yet, insert [`LodLevelSpawnRequest`] so a system can build it.
pub fn sync_lod_level_roots(
	mut commands: Commands,
	hosts: Query<
		(Entity, &LodSceneLevel, Option<&Children>),
		(With<LodSceneHost>, Changed<LodSceneLevel>),
	>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	mut visibilities: Query<&mut Visibility>,
) {
	for (host, level, host_children) in &hosts {
		let desired = *level;

		// No children yet → nothing to show/hide; ask for the first level root to be spawned.
		let Some(host_children) = host_children else {
			commands.entity(host).insert(LodLevelSpawnRequest { level: desired });
			continue;
		};

		// Locate the LodLevelRoots bag among the host's direct children.
		let mut roots_entity = None;
		for child in host_children.iter() {
			if level_roots_heads.contains(child) {
				roots_entity = Some(child);
				break;
			}
		}

		// Host has children but no LodLevelRoots yet → same as cold start: request a spawn.
		let Some(roots_entity) = roots_entity else {
			commands.entity(host).insert(LodLevelSpawnRequest { level: desired });
			continue;
		};

		let Ok(root_children) = level_roots_heads.get(roots_entity) else {
			continue;
		};

		// Flip visibility: Inherited for the desired level, Hidden for every other level root.
		let child_ids: Vec<Entity> = root_children.iter().collect();
		let mut found = false;
		for child in child_ids {
			let Ok(root) = root_keys.get(child) else {
				continue;
			};
			let Ok(mut visibility) = visibilities.get_mut(child) else {
				continue;
			};
			if root.0 == desired {
				found = true;
				*visibility = Visibility::Inherited;
			} else {
				*visibility = Visibility::Hidden;
			}
		}

		// Desired root present → drop any stale spawn request (e.g. left over after a
		// prior cull). Missing → request fulfill so a culled band can come back.
		if found {
			commands.entity(host).remove::<LodLevelSpawnRequest>();
		} else {
			commands.entity(host).insert(LodLevelSpawnRequest { level: desired });
		}
	}
}

/// Plugin: marker types only. Prefer [`crate::LodFinePassPlugin`] for runtime systems
/// (track / sync / fulfill ordering).
pub struct LodSceneHostPlugin;

impl Plugin for LodSceneHostPlugin {
	fn build(&self, _app: &mut App) {}
}
