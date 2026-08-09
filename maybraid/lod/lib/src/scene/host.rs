//! Runtime ECS hosts that switch LOD level roots without despawning the host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use crate::scene::cull::closest_available_lod_level;
use crate::scene::level::LodSceneLevel;

/// Marker: this entity owns LOD level roots and a current [`LodSceneLevel`].
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodSceneHost;

/// Whether a nested host may run fine-phase **refresh** (level produce / cull / probe update).
///
/// Walks ancestors (skipping `entity` itself) and records the nearest enclosing
/// [`LodLevelRoot`]. When a parent [`LodSceneHost`] is found, refresh is allowed iff
/// that root's level equals the parent's desired [`LodSceneLevel`]. No parent host →
/// top-level → allowed. Under a parent host but not under any level root → allowed
/// (cold scaffolding).
///
/// Initial chunk fulfill (empty level-roots bag) is **not** gated by this — begin still
/// admits empty nested hosts; upgrades require this gate once any root exists.
pub fn nested_host_parent_allows_refresh(
	entity: Entity,
	child_of: &Query<&ChildOf>,
	host_levels: &Query<&LodSceneLevel, With<LodSceneHost>>,
	level_roots: &Query<&LodLevelRoot>,
) -> bool {
	let Ok(parent) = child_of.get(entity) else {
		return true;
	};
	let mut current = parent.parent();
	let mut enclosing_root: Option<LodSceneLevel> = None;
	loop {
		if enclosing_root.is_none() {
			if let Ok(root) = level_roots.get(current) {
				enclosing_root = Some(root.0);
			}
		}
		if let Ok(desired) = host_levels.get(current) {
			return match enclosing_root {
				Some(root_level) => root_level == *desired,
				None => true,
			};
		}
		let Ok(next) = child_of.get(current) else {
			return true;
		};
		current = next.parent();
	}
}

/// Desired [`LodSceneLevel`] of the nearest ancestor [`LodSceneHost`], or
/// [`LodSceneLevel::High`] when there is none (top-level ranking).
///
/// `host` is skipped — walk starts at its parent. Used for fulfill drain priority
/// `(parent_level, self_level)`.
pub fn parent_host_desired_or_high(
	host: Entity,
	child_of: &Query<&ChildOf>,
	host_levels: &Query<&LodSceneLevel, With<LodSceneHost>>,
) -> LodSceneLevel {
	let Ok(parent) = child_of.get(host) else {
		return LodSceneLevel::High;
	};
	let mut current = parent.parent();
	loop {
		if let Ok(level) = host_levels.get(current) {
			return *level;
		}
		let Ok(next) = child_of.get(current) else {
			return LodSceneLevel::High;
		};
		current = next.parent();
	}
}

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
/// 2. Visibility:
///    - ready desired → show it
///    - pending desired + **cold** (no ready root yet) → show pending (stream in)
///    - desired not ready yet + **warm** (some other ready root exists) → show the
///      single ready root [`closest_available_lod_level`] to desired (not every warm
///      root), including the frame *before* fulfill creates the pending desired root
/// 3. If no matching root exists yet, insert [`LodLevelSpawnRequest`].
///
/// Pending roots ([`crate::LodLevelRootPending`]) count as present for spawn
/// requests. Roots with [`crate::LodCullInFlight`] count as absent (dying).
/// Warm-swap reveal is owned by chunk fulfill completion.
pub fn sync_lod_level_roots(
	mut commands: Commands,
	hosts: Query<
		(Entity, &LodSceneLevel, Option<&Children>),
		(With<LodSceneHost>, Changed<LodSceneLevel>),
	>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	pending: Query<(), With<crate::LodLevelRootPending>>,
	wants_cull: Query<(), With<crate::LodCullInFlight>>,
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

		let child_ids: Vec<Entity> = root_children.iter().collect();
		let mut found_desired = false;
		let mut has_ready_desired = false;
		let mut ready_levels: Vec<LodSceneLevel> = Vec::new();
		for &child in &child_ids {
			let Ok(root) = root_keys.get(child) else {
				continue;
			};
			if wants_cull.contains(child) {
				continue;
			}
			let is_pending = pending.contains(child);
			if !is_pending {
				ready_levels.push(root.0);
				if root.0 == desired {
					has_ready_desired = true;
					found_desired = true;
				}
			} else if root.0 == desired {
				found_desired = true;
			}
		}

		// Warm hold while desired is pending *or* not spawned yet (SyncRoots runs
		// before Fulfill begin — requiring an existing pending root hid the prior
		// ready root for a frame and caused empty swaps).
		let warm_hold_level = if !has_ready_desired {
			closest_available_lod_level(desired, ready_levels.iter().copied())
		} else {
			None
		};
		let has_ready_any = !ready_levels.is_empty();

		for child in child_ids {
			let Ok(root) = root_keys.get(child) else {
				continue;
			};
			let Ok(mut visibility) = visibilities.get_mut(child) else {
				continue;
			};
			if wants_cull.contains(child) {
				*visibility = Visibility::Hidden;
				continue;
			}
			let is_pending = pending.contains(child);
			let show = if root.0 == desired && !is_pending {
				// Ready desired.
				true
			} else if root.0 == desired && is_pending && !has_ready_any {
				// Cold: stream the pending desired root.
				true
			} else if !is_pending && warm_hold_level == Some(root.0) {
				// Warm hold: single closest ready band until desired is ready.
				true
			} else {
				false
			};
			*visibility = if show {
				Visibility::Inherited
			} else {
				Visibility::Hidden
			};
		}

		// Desired root present (ready or pending) → drop stale spawn request.
		// Missing → request fulfill so a culled band can come back.
		if found_desired {
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
