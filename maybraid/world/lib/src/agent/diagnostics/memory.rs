use bevy::prelude::*;
use maybraid_memory_census::MemoryCensus;
use render_item::mesh::handle::EnforcedCaches;
use serde::Serialize;
use std::collections::BTreeMap;

/// Exact component membership transitions since diagnostic observer installation.
/// Replacing a value is not an addition; removal includes entity despawn.
/// These counters do not count all entity lifetimes or archetype migrations.
#[derive(Clone, Default, Serialize)]
struct MembershipTransitions {
	additions: u64,
	removals: u64,
}

#[derive(Clone, Default, Resource, Serialize)]
struct LifetimeCounters(BTreeMap<&'static str, MembershipTransitions>);

impl LifetimeCounters {
	fn observe<T: Component>(app: &mut App, label: &'static str) {
		app.add_observer(move |_: On<Add, T>, mut counters: ResMut<Self>| {
			counters.0.entry(label).or_default().additions += 1;
		});
		app.add_observer(move |_: On<Remove, T>, mut counters: ResMut<Self>| {
			counters.0.entry(label).or_default().removals += 1;
		});
	}
}

/// Installed only for a memory-census build using --agent-diagnostics. There is
/// no per-frame scan; snapshots remain explicit and outside timing windows.
pub(super) fn install(app: &mut App) {
	app.init_resource::<LifetimeCounters>();
	LifetimeCounters::observe::<chico::ChicoGroveHost>(app, "grove_hosts");
	LifetimeCounters::observe::<lod::LodLevelRoot>(app, "lod_level_roots");
	LifetimeCounters::observe::<bevy::world_serialization::WorldAssetRoot>(
		app,
		"world_asset_roots",
	);
	LifetimeCounters::observe::<Mesh3d>(app, "mesh3d");
}

#[derive(Serialize)]
pub(crate) struct ObservedMemoryCensus {
	#[serde(flatten)]
	census: MemoryCensus,
	/// None means observers were not installed, not zero transitions.
	component_membership_transitions: Option<LifetimeCounters>,
}

pub(crate) fn capture(world: &World) -> ObservedMemoryCensus {
	let mut census = MemoryCensus::capture(world);
	// EnforcedCaches and terrain overlay references alias the same mailbox. Sample
	// each concrete map once, without counting those Arc aliases again.
	if let Some(cache) = world.get_resource::<EnforcedCaches<durham::TerrainMeshBuilder>>() {
		census.add_handle_cache(world, "durham_terrain", cache.handle_map().try_snapshot());
	}
	if let Some(cache) = world.get_resource::<EnforcedCaches<durham::ComposedWater>>() {
		census.add_handle_cache(world, "durham_water", cache.handle_map().try_snapshot());
	}
	ObservedMemoryCensus {
		census,
		component_membership_transitions: world.get_resource::<LifetimeCounters>().cloned(),
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Component)]
	struct Tracked;

	#[test]
	fn membership_counts_reentry_and_despawn_but_not_replacement() -> anyhow::Result<()> {
		let mut app = App::new();
		app.init_resource::<LifetimeCounters>();
		LifetimeCounters::observe::<Tracked>(&mut app, "tracked");
		let entity = app.world_mut().spawn(Tracked).id();
		app.world_mut().entity_mut(entity).insert(Tracked);
		app.world_mut().entity_mut(entity).remove::<Tracked>();
		app.world_mut().entity_mut(entity).insert(Tracked);
		app.world_mut().despawn(entity);
		let counters = app.world().resource::<LifetimeCounters>();
		let transitions = counters.0.get("tracked").ok_or_else(|| {
			anyhow::anyhow!("registered membership observers did not record transitions")
		})?;
		assert_eq!(transitions.additions, 2);
		assert_eq!(transitions.removals, 2);
		Ok(())
	}
}
