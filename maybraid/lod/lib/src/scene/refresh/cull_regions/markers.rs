//! Cheap eligibility markers for nested refresh / cull filters.

use bevy::prelude::*;

use crate::scene::host::{
	lod_level_roots_entity, nested_host_parent_allows_refresh, LodLevelRoot, LodLevelRoots,
	LodSceneHost,
};
use crate::scene::level::LodSceneLevel;

use super::super::sync::LodCullInFlight;
use super::super::{ensure_refresh_core, LodRefreshSystems};

/// Nested host may participate in fine-phase refresh / region cull.
///
/// Allowed when under the parent host's **desired** or **shown** (warm-hold)
/// [`LodLevelRoot`]. Maintained from parent [`LodSceneLevel`] changes (and on host
/// add) so hot paths can filter with `With<LodNestedRefreshAllowed>`.
/// Mutually exclusive with [`LodNestedRefreshBlocked`].
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodNestedRefreshAllowed;

/// Nested host is gated off (not under parent's desired or shown level root).
/// See [`LodNestedRefreshAllowed`].
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodNestedRefreshBlocked;

/// Host currently has at least one non-current, non-culling level root.
#[derive(Debug, Clone, Copy, Default, Component)]
pub struct LodHostHasCullableRoots;

/// Cap nested-refresh gate work per frame.
#[derive(Resource, Debug, Clone, Copy)]
pub struct LodNestedRefreshSyncBudget {
	/// Max hosts stamped with allowed/blocked this frame.
	pub hosts_per_frame: u32,
	/// Max hierarchy nodes visited while expanding dirty roots this frame.
	pub expand_nodes_per_frame: u32,
}

impl Default for LodNestedRefreshSyncBudget {
	fn default() -> Self {
		Self { hosts_per_frame: 64, expand_nodes_per_frame: 128 }
	}
}

/// Overflow work not finished this frame.
#[derive(Resource, Debug, Default)]
pub(crate) struct LodNestedRefreshPending {
	/// Hosts waiting for an allowed/blocked stamp.
	stamp: Vec<Entity>,
	/// Hierarchy nodes whose children still need BFS expansion into [`Self::stamp`].
	expand: Vec<Entity>,
}

fn set_nested_refresh_gate(
	commands: &mut Commands,
	entity: Entity,
	child_of: &Query<&ChildOf>,
	host_levels: &Query<&LodSceneLevel, With<LodSceneHost>>,
	level_roots: &Query<&LodLevelRoot>,
	children_q: &Query<&Children>,
	level_roots_bags: &Query<(), With<LodLevelRoots>>,
	visibilities: &Query<&Visibility>,
	allowed: &Query<(), With<LodNestedRefreshAllowed>>,
	blocked: &Query<(), With<LodNestedRefreshBlocked>>,
) {
	let want = nested_host_parent_allows_refresh(
		entity,
		child_of,
		host_levels,
		level_roots,
		children_q,
		level_roots_bags,
		visibilities,
	);
	let has_allowed = allowed.contains(entity);
	let has_blocked = blocked.contains(entity);
	if want {
		if !has_allowed {
			commands.entity(entity).insert(LodNestedRefreshAllowed);
		}
		if has_blocked {
			commands.entity(entity).remove::<LodNestedRefreshBlocked>();
		}
	} else {
		if !has_blocked {
			commands.entity(entity).insert(LodNestedRefreshBlocked);
		}
		if has_allowed {
			commands.entity(entity).remove::<LodNestedRefreshAllowed>();
		}
	}
}

/// Visit one hierarchy node: enqueue child hosts for stamp, continue BFS into children.
fn expand_one(
	node: Entity,
	children_q: &Query<&Children>,
	hosts: &Query<(), With<LodSceneHost>>,
	stamp: &mut Vec<Entity>,
	expand: &mut Vec<Entity>,
) {
	let Ok(children) = children_q.get(node) else {
		return;
	};
	for child in children.iter() {
		if hosts.contains(child) {
			stamp.push(child);
		}
		expand.push(child);
	}
}

/// On new / ungated hosts + when any host level or root visibility changes, refresh gate markers.
///
/// Descendant fan-out is **budgeted BFS** into [`LodNestedRefreshPending`] — level/vis
/// changes do not walk the whole subtree in one frame.
#[allow(private_interfaces)]
pub fn sync_nested_refresh_allowed(
	mut commands: Commands,
	budget: Res<LodNestedRefreshSyncBudget>,
	mut pending: ResMut<LodNestedRefreshPending>,
	added: Query<Entity, Added<LodSceneHost>>,
	ungated: Query<
		Entity,
		(With<LodSceneHost>, Without<LodNestedRefreshAllowed>, Without<LodNestedRefreshBlocked>),
	>,
	changed_levels: Query<Entity, (With<LodSceneHost>, Changed<LodSceneLevel>)>,
	changed_root_vis: Query<(Entity, &ChildOf), (With<LodLevelRoot>, Changed<Visibility>)>,
	children_q: Query<&Children>,
	hosts: Query<(), With<LodSceneHost>>,
	child_of: Query<&ChildOf>,
	host_levels: Query<&LodSceneLevel, With<LodSceneHost>>,
	level_roots: Query<&LodLevelRoot>,
	level_roots_bags: Query<(), With<LodLevelRoots>>,
	visibilities: Query<&Visibility>,
	allowed: Query<(), With<LodNestedRefreshAllowed>>,
	blocked: Query<(), With<LodNestedRefreshBlocked>>,
) {
	let mut stamp = std::mem::take(&mut pending.stamp);
	let mut expand = std::mem::take(&mut pending.expand);

	stamp.extend(added.iter());
	stamp.extend(ungated.iter());

	// Level change: stamp the host now; expand descendants across frames.
	for entity in &changed_levels {
		stamp.push(entity);
		expand.push(entity);
	}
	// Vis flip: stamp owning host; expand under **that root** only.
	for (root, root_of) in &changed_root_vis {
		let bag = root_of.parent();
		let Ok(bag_of) = child_of.get(bag) else {
			continue;
		};
		let host = bag_of.parent();
		if hosts.contains(host) {
			stamp.push(host);
		}
		expand.push(root);
	}

	let expand_limit = budget.expand_nodes_per_frame as usize;
	let mut expanded = 0usize;
	while expanded < expand_limit {
		let Some(node) = expand.pop() else {
			break;
		};
		expand_one(node, &children_q, &hosts, &mut stamp, &mut expand);
		expanded += 1;
	}

	if stamp.is_empty() && expand.is_empty() {
		return;
	}

	stamp.sort_unstable();
	stamp.dedup();

	let stamp_limit = budget.hosts_per_frame as usize;
	let (this_frame, rest) = if stamp.len() > stamp_limit {
		let rest = stamp.split_off(stamp_limit);
		(stamp, rest)
	} else {
		(stamp, Vec::new())
	};
	pending.stamp = rest;
	pending.expand = expand;

	for entity in this_frame {
		set_nested_refresh_gate(
			&mut commands,
			entity,
			&child_of,
			&host_levels,
			&level_roots,
			&children_q,
			&level_roots_bags,
			&visibilities,
			&allowed,
			&blocked,
		);
	}
}

fn host_has_cullable_roots(
	current: LodSceneLevel,
	host_children: &Children,
	level_roots_heads: &Query<&Children, With<LodLevelRoots>>,
	root_keys: &Query<&LodLevelRoot>,
	wants_cull: &Query<(), With<LodCullInFlight>>,
) -> bool {
	let Some(roots_entity) = lod_level_roots_entity(host_children, level_roots_heads) else {
		return false;
	};
	let Ok(root_children) = level_roots_heads.get(roots_entity) else {
		return false;
	};
	root_children.iter().any(|child| {
		let Ok(root) = root_keys.get(child) else {
			return false;
		};
		root.0 != current && !wants_cull.contains(child)
	})
}

fn apply_cullable_marker(
	commands: &mut Commands,
	host: Entity,
	want: bool,
	marked: &Query<(), With<LodHostHasCullableRoots>>,
) {
	let has = marked.contains(host);
	if want && !has {
		commands.entity(host).insert(LodHostHasCullableRoots);
	} else if !want && has {
		commands.entity(host).remove::<LodHostHasCullableRoots>();
	}
}

/// Maintain [`LodHostHasCullableRoots`] when host level / children / roots bag change.
pub fn sync_cullable_roots_marker(
	mut commands: Commands,
	host_levels: Query<&LodSceneLevel, With<LodSceneHost>>,
	host_children_q: Query<&Children, With<LodSceneHost>>,
	changed_hosts: Query<
		Entity,
		(With<LodSceneHost>, Or<(Changed<LodSceneLevel>, Changed<Children>, Added<LodSceneHost>)>),
	>,
	changed_bags: Query<&ChildOf, (With<LodLevelRoots>, Changed<Children>)>,
	level_roots_heads: Query<&Children, With<LodLevelRoots>>,
	root_keys: Query<&LodLevelRoot>,
	wants_cull: Query<(), With<LodCullInFlight>>,
	marked: Query<(), With<LodHostHasCullableRoots>>,
) {
	let mut dirty: Vec<Entity> = changed_hosts.iter().collect();
	for child_of in &changed_bags {
		dirty.push(child_of.parent());
	}
	dirty.sort_unstable();
	dirty.dedup();

	for host in dirty {
		let Ok(current) = host_levels.get(host) else {
			continue;
		};
		let Ok(host_children) = host_children_q.get(host) else {
			continue;
		};
		let want = host_has_cullable_roots(
			*current,
			host_children,
			&level_roots_heads,
			&root_keys,
			&wants_cull,
		);
		apply_cullable_marker(&mut commands, host, want, &marked);
	}
}

/// Plugin: maintain nested-refresh + cullable-root markers.
pub struct LodCullMarkerPlugin;

impl Plugin for LodCullMarkerPlugin {
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.init_resource::<LodNestedRefreshSyncBudget>()
			.init_resource::<LodNestedRefreshPending>()
			.add_systems(
				Update,
				(
					sync_nested_refresh_allowed
						.after(LodRefreshSystems::UpdateLevels)
						.before(LodRefreshSystems::Cull),
					sync_cullable_roots_marker
						.after(LodRefreshSystems::SyncRoots)
						.before(LodRefreshSystems::Cull),
				),
			);
	}
}
