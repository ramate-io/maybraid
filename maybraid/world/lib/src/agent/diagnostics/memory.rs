use bevy::prelude::*;
use maybraid_memory_census::MemoryCensus;
use render_item::mesh::handle::EnforcedCaches;

pub(crate) fn capture(world: &World) -> MemoryCensus {
	let mut census = MemoryCensus::capture(world);
	// EnforcedCaches and terrain overlay references alias the same mailbox. Sample
	// each concrete map once, without counting those Arc aliases again.
	if let Some(cache) = world.get_resource::<EnforcedCaches<durham::TerrainMeshBuilder>>() {
		census.add_handle_cache(world, "durham_terrain", cache.handle_map().try_snapshot());
	}
	if let Some(cache) = world.get_resource::<EnforcedCaches<durham::ComposedWater>>() {
		census.add_handle_cache(world, "durham_water", cache.handle_map().try_snapshot());
	}
	census
}
