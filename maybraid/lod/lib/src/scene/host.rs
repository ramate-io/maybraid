//! Runtime ECS hosts that switch LOD level roots without despawning the host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use crate::scene::level::LodSceneLevel;

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
#[derive(Debug, Clone, Copy, Component, Default)]
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
	let host_bounds = crate::scene::refresh::LodHostBounds(bounds);
	bsn! {
		LodSceneHost
		template_value(level)
		template_value(host_bounds)
		Transform::default()
		Visibility::Inherited
		Children [ {host_children} ]
	}
}

/// Host with an empty [`LodLevelRoots`] bag and a spawn request for `level`.
///
/// Chunk fulfill streams [`crate::LodScene::scene_chunks_with_level`] into the
/// pending root — used when skipping warm multi-root prewarm.
pub fn lod_host_scene_pending(level: LodSceneLevel, bounds: Aabb3d) -> impl Scene + 'static {
	let roots: Box<dyn Scene> = Box::new(bsn! {
		LodLevelRoots
		Transform::default()
		Visibility::Inherited
	});
	let host_children: Vec<Box<dyn Scene>> = vec![roots];
	let host_bounds = crate::scene::refresh::LodHostBounds(bounds);
	bsn! {
		LodSceneHost
		template_value(level)
		template_value(host_bounds)
		template_value(LodLevelSpawnRequest { level })
		Transform::default()
		Visibility::Inherited
		Children [ {host_children} ]
	}
}

/// After [`LodSceneLevel`] changes on a host: request missing roots and flip visibility.
///
/// Flow per host:
/// 1. Find the [`LodLevelRoots`] child (or request a spawn if the host/roots bag is missing).
/// 2. Show a **ready** root matching the desired level; keep pending roots Hidden.
/// 3. If no matching root exists yet, insert [`LodLevelSpawnRequest`] so a system can build it.
///
/// Pending roots ([`crate::LodLevelRootPending`]) count as present for spawn
/// requests but stay Hidden until chunk fulfill completes.
pub fn sync_lod_level_roots(
	mut commands: Commands,
	hosts: Query<
		(Entity, &LodSceneLevel, Option<&Children>),
		(With<LodSceneHost>, Changed<LodSceneLevel>),
	>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	pending: Query<(), With<crate::LodLevelRootPending>>,
	mut visibilities: Query<&mut Visibility>,
) {
	let t0 = std::time::Instant::now();
	let mut n = 0u32;
	let mut requested = 0u32;
	for (host, level, host_children) in &hosts {
		n += 1;
		let desired = *level;

		// No children yet → nothing to show/hide; ask for the first level root to be spawned.
		let Some(host_children) = host_children else {
			commands.entity(host).insert(LodLevelSpawnRequest { level: desired });
			requested += 1;
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
			requested += 1;
			continue;
		};

		let Ok(root_children) = level_roots_heads.get(roots_entity) else {
			continue;
		};

		// Ready desired → Inherited; pending desired stays Hidden; others Hidden.
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
				if pending.contains(child) {
					*visibility = Visibility::Hidden;
				} else {
					*visibility = Visibility::Inherited;
				}
			} else {
				*visibility = Visibility::Hidden;
			}
		}

		// Desired root present (ready or pending) → drop stale spawn request.
		// Missing → request fulfill so a culled band can come back.
		if found {
			commands.entity(host).remove::<LodLevelSpawnRequest>();
		} else {
			commands.entity(host).insert(LodLevelSpawnRequest { level: desired });
			requested += 1;
		}
	}
	if n > 0 {
		info!(
			"[lod.refresh] sync_lod_level_roots: hosts={n} spawn_requests={requested} in {:.2}ms",
			t0.elapsed().as_secs_f64() * 1000.0
		);
	}
}

/// Plugin: marker types only. Prefer [`crate::LodRefreshCorePlugin`] for runtime systems
/// (track / sync / fulfill ordering).
pub struct LodSceneHostPlugin;

impl Plugin for LodSceneHostPlugin {
	fn build(&self, _app: &mut App) {}
}
