use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::*;
use maybraid_memory_census::{
	GenerationCensus, MemoryCensus, StreamingCensus, VisibilityBacklogCensus,
};
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
	let mut streaming = StreamingCensus {
		worker_backlog_status: "unknown_no_aggregate_readout",
		gpu_residency_status: "unknown_main_world_only",
		generation: GenerationCensus { status: "resource_absent", ..default() },
		..default()
	};
	// Includes Disabled roots. No hierarchy walks, worker reads or changes to queues.
	for entity in world.iter_entities() {
		if entity.contains::<lod::LodSceneHost>() {
			streaming.scene_hosts += 1;
			streaming.disabled_scene_hosts += usize::from(entity.contains::<Disabled>());
		}
		streaming.content_streamed_hosts +=
			usize::from(entity.contains::<lod::LodSceneHostStreamed>());
		streaming.pending_level_roots += usize::from(entity.contains::<lod::LodLevelRootPending>());
		streaming.lazy_pending += usize::from(entity.contains::<lod::LodLazyPending>());
		if let Some(cull) = entity.get::<lod::LodCullInFlight>() {
			if cull.started {
				streaming.culls_started += 1;
			} else {
				streaming.culls_queued += 1;
			}
		}
		if entity.contains::<lod::hcsg::RetiredHost>() {
			streaming.retired_roots += 1;
			streaming.disabled_retired_roots += usize::from(entity.contains::<Disabled>());
			*streaming
				.successor_status_counts
				.entry(lod::hcsg::retired_successor_status(world, entity.id()))
				.or_default() += 1;
		}
	}
	streaming.visibility_queue =
		world.get_resource::<lod::scene::host::LodTreeVisQueue>().map(|queue| {
			let (show_jobs, hide_jobs, stack_entries) = queue.diagnostic_backlog();
			VisibilityBacklogCensus { show_jobs, hide_jobs, stack_entries }
		});
	if let Some(storage) = world.get_resource::<lod::hcsg::HcsgStorage>() {
		let (top, nested) = storage.rebuilds_after_eviction();
		streaming.generation.rebuilds_after_eviction_top = Some(top);
		streaming.generation.rebuilds_after_eviction_nested = Some(nested);
		match storage.try_store_sizes() {
			Ok(mut sizes) => {
				streaming.generation.status = "sampled";
				streaming.generation.total_stores = Some(sizes.len());
				streaming.generation.total_entries = Some(sizes.iter().map(|(_, n)| *n).sum());
				sizes.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
				streaming.generation.omitted_stores = sizes.len().saturating_sub(32);
				sizes.truncate(32);
				streaming.generation.store_entries =
					Some(sizes.into_iter().map(|(name, count)| (name.to_owned(), count)).collect());
			}
			Err(_) => streaming.generation.status = "busy_unknown",
		}
	}
	census.streaming = Some(streaming);
	census
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn streaming_snapshot_includes_disabled_retired_roots_and_preserves_unknowns(
	) -> anyhow::Result<()> {
		let mut world = World::new();
		world.spawn((lod::hcsg::RetiredHost, Disabled, lod::LodLazyPending));
		world.spawn(lod::LodCullInFlight { started: false });
		world.spawn(lod::LodCullInFlight { started: true });
		let before = world.entity_count();
		let absent =
			capture(&world).streaming.ok_or_else(|| anyhow::anyhow!("missing streaming"))?;
		assert_eq!(absent.retired_roots, 1);
		assert_eq!(absent.disabled_retired_roots, 1);
		assert_eq!(absent.successor_status_counts.get("unknown"), Some(&1));
		assert_eq!(absent.lazy_pending, 1);
		assert_eq!((absent.culls_queued, absent.culls_started), (1, 1));
		assert_eq!(absent.generation.status, "resource_absent");
		assert!(absent.generation.store_entries.is_none());
		assert!(absent.visibility_queue.is_none());
		assert_eq!(world.entity_count(), before);
		world.init_resource::<lod::hcsg::HcsgStorage>();
		let sampled =
			capture(&world).streaming.ok_or_else(|| anyhow::anyhow!("missing streaming"))?;
		assert_eq!(sampled.generation.status, "sampled");
		assert_eq!(sampled.generation.store_entries, Some(Vec::new()));
		assert_eq!(sampled.generation.total_entries, Some(0));
		assert_eq!(sampled.generation.rebuilds_after_eviction_top, Some(0));
		Ok(())
	}
}
