//! Runtime ECS hosts that switch LOD level roots without despawning the host.

use bevy::ecs::query::QueryData;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};

use crate::scene::cull::closest_available_lod_level;
use crate::scene::level::LodSceneLevel;

/// Marker: this entity owns LOD level roots and a current [`LodSceneLevel`].
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodSceneHost;

/// Whether a visibility value counts as on-screen (warm-hold / cold-fill).
#[inline]
pub fn lod_root_is_shown(visibility: Visibility) -> bool {
	!matches!(visibility, Visibility::Hidden)
}

/// True when this entity or an ancestor [`LodSceneHost`] is [`Visibility::Hidden`].
///
/// Present-layer hide stamps Hidden on leaving grove parents so scene cull
/// does not nibble High roots on a host about to despawn.
pub fn lod_scene_host_or_ancestor_hidden(
	entity: Entity,
	child_of: &Query<&ChildOf>,
	hosts: &Query<(), With<LodSceneHost>>,
	visibilities: &Query<&Visibility>,
) -> bool {
	let mut current = Some(entity);
	while let Some(entity) = current {
		if hosts.contains(entity)
			&& visibilities.get(entity).is_ok_and(|vis| matches!(*vis, Visibility::Hidden))
		{
			return true;
		}
		current = child_of.get(entity).ok().map(|child| child.parent());
	}
	false
}

/// Whether `host` currently shows a [`LodLevelRoot`] at `level` (not Hidden).
pub fn host_shows_level_root(
	host: Entity,
	level: LodSceneLevel,
	children_q: &Query<&Children>,
	level_roots_bags: &Query<(), With<LodLevelRoots>>,
	root_keys: &Query<&LodLevelRoot>,
	visibilities: &Query<&Visibility>,
) -> bool {
	let Ok(host_kids) = children_q.get(host) else {
		return false;
	};
	let Some(bag) = lod_level_roots_entity(host_kids, level_roots_bags) else {
		return false;
	};
	let Ok(root_kids) = children_q.get(bag) else {
		return false;
	};
	root_kids.iter().any(|root_e| {
		root_keys.get(root_e).is_ok_and(|root| root.0 == level)
			&& visibilities.get(root_e).is_ok_and(|vis| lod_root_is_shown(*vis))
	})
}

/// Whether a nested host may run fine-phase **refresh** (level produce / cull / probe update).
///
/// Walks ancestors (skipping `entity` itself) and records the nearest enclosing
/// [`LodLevelRoot`]. When a parent [`LodSceneHost`] is found, refresh is allowed when
/// that root matches the parent's **desired** level **or** is currently **shown**
/// (Inherited/Visible warm-hold). No parent host → top-level → allowed. Under a parent
/// host but not under any level root → allowed (cold scaffolding).
///
/// Initial chunk fulfill (empty level-roots bag) is **not** gated by this — begin still
/// admits empty nested hosts; upgrades require this gate once any root exists.
pub fn nested_host_parent_allows_refresh(
	entity: Entity,
	child_of: &Query<&ChildOf>,
	host_levels: &Query<&LodSceneLevel, With<LodSceneHost>>,
	level_roots: &Query<&LodLevelRoot>,
	children_q: &Query<&Children>,
	level_roots_bags: &Query<(), With<LodLevelRoots>>,
	visibilities: &Query<&Visibility>,
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
				Some(root_level) => {
					root_level == *desired
						|| host_shows_level_root(
							current,
							root_level,
							children_q,
							level_roots_bags,
							level_roots,
							visibilities,
						)
				}
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

/// Direct child that owns [`LodLevelRoot`]s, if this host has one.
pub fn lod_level_roots_entity<D: QueryData>(
	host_children: &Children,
	bags: &Query<D, With<LodLevelRoots>>,
) -> Option<Entity> {
	host_children.iter().find(|&child| bags.contains(child))
}

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

/// Host currently shows more than one ready level root.
///
/// Inserted when a newly revealed ready desired overlaps last frame's shown
/// band so extract still has meshes. [`settle_lod_level_root_visibility`] runs
/// at the start of the next frame and hides extras.
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodLevelRootOverlap;

/// Whether this level root should be Inherited given desired + overlap.
pub(crate) fn lod_root_should_show(
	root_level: LodSceneLevel,
	desired: LodSceneLevel,
	is_pending: bool,
	has_ready_desired: bool,
	has_ready_any: bool,
	warm_hold_level: Option<LodSceneLevel>,
	desired_was_shown: bool,
	this_was_shown_ready: bool,
) -> bool {
	if root_level == desired && !is_pending {
		return true;
	}
	if root_level == desired && is_pending && !has_ready_any {
		return true;
	}
	if !is_pending && warm_hold_level == Some(root_level) {
		return true;
	}
	// One-frame overlap: first reveal of a ready desired keeps the previously
	// shown ready band so extract still has last frame's meshes.
	if has_ready_desired && !desired_was_shown && !is_pending && this_was_shown_ready {
		return true;
	}
	false
}

fn apply_lod_level_root_visibility(
	desired: LodSceneLevel,
	child_ids: &[Entity],
	root_keys: &Query<&LodLevelRoot>,
	pending: &Query<(), With<crate::LodLevelRootPending>>,
	wants_cull: &Query<(), With<crate::LodCullInFlight>>,
	visibilities: &mut Query<&mut Visibility>,
) -> bool {
	let mut has_ready_desired = false;
	let mut ready_levels: Vec<LodSceneLevel> = Vec::new();
	let mut desired_was_shown = false;
	let mut shown_ready: Vec<Entity> = Vec::new();
	for &child in child_ids {
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
			}
			if visibilities.get(child).is_ok_and(|v| lod_root_is_shown(*v)) {
				shown_ready.push(child);
				if root.0 == desired {
					desired_was_shown = true;
				}
			}
		}
	}

	let warm_hold_level = if !has_ready_desired {
		closest_available_lod_level(desired, ready_levels.iter().copied())
	} else {
		None
	};
	let has_ready_any = !ready_levels.is_empty();

	let mut shown_ready_after = 0usize;
	for &child in child_ids {
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
		let show = lod_root_should_show(
			root.0,
			desired,
			is_pending,
			has_ready_desired,
			has_ready_any,
			warm_hold_level,
			desired_was_shown,
			shown_ready.contains(&child),
		);
		*visibility = if show { Visibility::Inherited } else { Visibility::Hidden };
		if show && !is_pending {
			shown_ready_after += 1;
		}
	}
	shown_ready_after > 1
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
///    - first frame a ready desired is revealed → also keep the previously shown
///      ready band ([`LodLevelRootOverlap`]); next frame
///      [`settle_lod_level_root_visibility`] hides extras
/// 3. If no matching root exists yet, insert [`LodLevelSpawnRequest`].
///
/// Pending roots ([`crate::LodLevelRootPending`]) count as present for spawn
/// requests. Roots with [`crate::LodCullInFlight`] count as absent (dying).
/// Warm-swap reveal is owned by chunk fulfill completion (which also leaves the
/// prior ready band shown for one frame).
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
	for (host, level, host_children) in &hosts {
		let desired = *level;

		// No children yet → nothing to show/hide; ask for the first level root to be spawned.
		let Some(host_children) = host_children else {
			commands.entity(host).insert(LodLevelSpawnRequest { level: desired });
			continue;
		};

		let Some(roots_entity) = lod_level_roots_entity(host_children, &level_roots_heads) else {
			commands.entity(host).insert(LodLevelSpawnRequest { level: desired });
			continue;
		};

		let Ok(root_children) = level_roots_heads.get(roots_entity) else {
			continue;
		};

		let child_ids: Vec<Entity> = root_children.iter().collect();
		let mut found_desired = false;
		for &child in &child_ids {
			let Ok(root) = root_keys.get(child) else {
				continue;
			};
			if wants_cull.contains(child) {
				continue;
			}
			if root.0 == desired {
				found_desired = true;
			}
		}

		if apply_lod_level_root_visibility(
			desired,
			&child_ids,
			&root_keys,
			&pending,
			&wants_cull,
			&mut visibilities,
		) {
			commands.entity(host).insert(LodLevelRootOverlap);
		} else {
			commands.entity(host).remove::<LodLevelRootOverlap>();
		}

		// Desired root present (ready or pending) → drop stale spawn request.
		// Missing → request fulfill so a culled band can come back.
		if found_desired {
			commands.entity(host).remove::<LodLevelSpawnRequest>();
		} else {
			commands.entity(host).insert(LodLevelSpawnRequest { level: desired });
		}
	}
}

/// Hide extra ready roots after a one-frame overlap.
///
/// Runs at the start of [`crate::LodRefreshSystems::SyncRoots`], before
/// [`sync_lod_level_roots`], so a same-frame settle cannot undo the overlap
/// that sync just created. Chunk fulfill inserts [`LodLevelRootOverlap`] when
/// it first shows a completed root without hiding siblings.
pub fn settle_lod_level_root_visibility(
	mut commands: Commands,
	hosts: Query<
		(Entity, &LodSceneLevel, Option<&Children>),
		(With<LodSceneHost>, With<LodLevelRootOverlap>),
	>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	pending: Query<(), With<crate::LodLevelRootPending>>,
	wants_cull: Query<(), With<crate::LodCullInFlight>>,
	mut visibilities: Query<&mut Visibility>,
) {
	for (host, level, host_children) in &hosts {
		let Some(host_children) = host_children else {
			commands.entity(host).remove::<LodLevelRootOverlap>();
			continue;
		};
		let Some(roots_entity) = lod_level_roots_entity(host_children, &level_roots_heads) else {
			commands.entity(host).remove::<LodLevelRootOverlap>();
			continue;
		};
		let Ok(root_children) = level_roots_heads.get(roots_entity) else {
			continue;
		};
		let child_ids: Vec<Entity> = root_children.iter().collect();
		if !apply_lod_level_root_visibility(
			*level,
			&child_ids,
			&root_keys,
			&pending,
			&wants_cull,
			&mut visibilities,
		) {
			commands.entity(host).remove::<LodLevelRootOverlap>();
		}
	}
}

/// Plugin: marker types only. Prefer [`crate::LodRefreshCorePlugin`] for runtime systems
/// (track / sync / fulfill ordering).
pub struct LodSceneHostPlugin;

impl Plugin for LodSceneHostPlugin {
	fn build(&self, _app: &mut App) {}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ready_desired_always_shown() {
		assert!(lod_root_should_show(
			LodSceneLevel::High,
			LodSceneLevel::High,
			false,
			true,
			true,
			None,
			true,
			true,
		));
	}

	#[test]
	fn overlap_keeps_previous_until_desired_was_shown() {
		assert!(lod_root_should_show(
			LodSceneLevel::Medium,
			LodSceneLevel::High,
			false,
			true,
			true,
			None,
			false,
			true,
		));
	}

	#[test]
	fn after_desired_was_shown_previous_hides() {
		assert!(!lod_root_should_show(
			LodSceneLevel::Medium,
			LodSceneLevel::High,
			false,
			true,
			true,
			None,
			true,
			true,
		));
	}

	#[test]
	fn warm_hold_shows_closest_ready() {
		assert!(lod_root_should_show(
			LodSceneLevel::Medium,
			LodSceneLevel::High,
			false,
			false,
			true,
			Some(LodSceneLevel::Medium),
			false,
			true,
		));
	}
}
