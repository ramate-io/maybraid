//! Typed renderer telemetry. No subscribers, text formatting or population scans.
use super::cache::{PackedGroveCache, ACTIVE};
use bevy::prelude::*;
use std::sync::atomic::Ordering;
#[derive(Default)]
pub struct PackedSnapshot {
	pub resident_tiles: usize,
	pub active_tiles: usize,
	pub resident_batches: usize,
	pub resident_instances_all_lods: usize,
	pub source_ir_bytes: u64,
	pub encoded_cpu_bytes: u64,
	pub resident_gpu_payload_bytes: u64,
	pub cache_builds: u64,
	pub budget_denials: u64,
}
impl PackedSnapshot {
	pub fn read(world: &World) -> Self {
		let Some(cache) = world.get_resource::<PackedGroveCache>() else { return Self::default() };
		Self {
			resident_tiles: cache.tiles.len(),
			active_tiles: cache
				.tiles
				.values()
				.filter(|tile| tile.signal.phase.load(Ordering::Acquire) == ACTIVE)
				.count(),
			resident_batches: cache.tiles.values().map(|tile| tile.payload.batch_count()).sum(),
			resident_instances_all_lods: cache
				.tiles
				.values()
				.map(|tile| tile.payload.instances())
				.sum(),
			source_ir_bytes: cache.retained_cpu_bytes(),
			encoded_cpu_bytes: cache
				.tiles
				.values()
				.flat_map(|tile| tile.proxies.iter())
				.filter_map(|entity| world.get::<super::gpu::PackedBatchInstance>(*entity))
				.map(|batch| batch.bytes.len() as u64)
				.sum(),
			resident_gpu_payload_bytes: cache.retained_gpu_bytes(),
			cache_builds: cache.builds,
			budget_denials: cache.budget_denials,
		}
	}
}
