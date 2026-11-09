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

/// Loaded presented hosts, including ordinary unsupported tiles and retained
/// active tiles whose ChicoGroveHost component was removed. Not visibility proof.
#[cfg(feature = "packed-diagnostics")]
#[derive(Default)]
pub struct PackedAdmissionSnapshot {
	pub host_tiles: usize,
	pub rows: Vec<PackedAdmissionCount>,
}
#[cfg(feature = "packed-diagnostics")]
pub struct PackedAdmissionCount {
	pub family: &'static str,
	pub reason: &'static str,
	pub tiles: usize,
}
#[cfg(feature = "packed-diagnostics")]
impl PackedAdmissionSnapshot {
	pub fn read(world: &mut World) -> Self {
		use super::{cache::AdmissionReason, lifecycle::RestoringOrdinary, mode::PackMode};
		use vegetation_components::CanopyShadowMode;
		// Has<Disabled> opts in to disabled warm hosts too. The census is loaded
		// ownership coverage; per-view candidates and issued draws remain separate.
		let mut hosts = world
			.query::<(Entity, &crate::ChicoGroveHost, Has<bevy::ecs::entity_disabling::Disabled>)>()
			.iter(world)
			.map(|(entity, host, _)| (entity, host.clone()))
			.collect::<std::collections::HashMap<_, _>>();
		let mode = world.get_resource::<PackMode>().copied().unwrap_or_else(PackMode::current);
		let canopy = world.get_resource::<CanopyShadowMode>().copied().unwrap_or_default();
		let views_supported = super::lifecycle::supported_view_configuration(world);
		let cache = world.get_resource::<PackedGroveCache>();
		if let Some(cache) = cache {
			for (&entity, tile) in &cache.tiles {
				hosts.entry(entity).or_insert_with(|| tile.original.clone());
			}
		}
		let host_tiles = hosts.len();
		let mut counts = std::collections::BTreeMap::new();
		for (entity, host) in hosts {
			let reason = if world.get::<RestoringOrdinary>(entity).is_some() {
				AdmissionReason::RestoringOrdinary
			} else if mode.is_off() {
				AdmissionReason::ModeOff
			} else if !mode.accepts(&host.tile) {
				AdmissionReason::UnselectedFamily
			} else if host.tile.woody_lod().is_none() {
				AdmissionReason::UnsupportedAdapter
			} else if canopy.canopy_casts() {
				AdmissionReason::ShadowPolicy
			} else if !views_supported {
				AdmissionReason::ViewPolicy
			} else if let Some(reason) = cache
				.and_then(|cache| cache.unsupported.get(&entity))
				.map(|(_, reason)| *reason)
			{
				reason
			} else if cache.is_some_and(|cache| cache.denied.contains_key(&entity)) {
				AdmissionReason::Budget
			} else if let Some(tile) = cache.and_then(|cache| cache.tiles.get(&entity)) {
				if tile.signal.phase.load(Ordering::Acquire) == ACTIVE {
					AdmissionReason::Active
				} else if tile.proxies.is_empty() {
					AdmissionReason::PendingAssets
				} else {
					AdmissionReason::PendingGpuReadiness
				}
			} else {
				AdmissionReason::PendingAssets
			};
			*counts.entry((host.tile.family_name(), reason)).or_insert(0usize) += 1;
		}
		Self {
			host_tiles,
			rows: counts
				.into_iter()
				.map(|((family, reason), tiles)| PackedAdmissionCount {
					family,
					reason: reason.label(),
					tiles,
				})
				.collect(),
		}
	}
}

#[cfg(all(test, feature = "packed-diagnostics"))]
mod admission_tests {
	use super::super::{cache::*, mode::PackMode};
	use super::*;
	use crate::{ChicoGroveHost, ForestGroveKind, ForestLayer};
	use std::sync::Arc;
	#[test]
	fn census_counts_ordinary_unsupported_and_active_owners_once() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PackMode::Compatible);
		world.spawn((Camera3d::default(), Camera::default(), Transform::IDENTITY));
		let extent = vegetation_groves::GroveExtent::new(Vec3::ZERO, Vec3::splat(40.0));
		let terrain = vegetation_groves::FlatTerrainSample::default();
		let host = ChicoGroveHost::new(
			ForestGroveKind::HighBush.grow_tile(extent, &terrain),
			ForestLayer::UpperCanopy,
		);
		let entity = world.spawn(host.clone()).id();
		let grass = world
			.spawn(ChicoGroveHost::new(
				ForestGroveKind::CommonTufts.grow_tile(extent, &terrain),
				ForestLayer::Tufts,
			))
			.id();
		world.entity_mut(grass).insert(bevy::ecs::entity_disabling::Disabled);
		let payload = TilePayload::from_host(&host, 1, false)
			.map_err(|reason| anyhow::anyhow!("fixture rejected {reason:?}"))?;
		let signal = Arc::new(TileSignal::new([]));
		assert!(signal.activate());
		let mut cache = PackedGroveCache::default();
		cache.tiles.insert(
			entity,
			ResidentTile {
				original: host,
				payload: Arc::new(payload),
				signal,
				proxies: Vec::new(),
				last_kit_epoch: 0,
			},
		);
		world.insert_resource(cache);
		let both = PackedAdmissionSnapshot::read(&mut world);
		assert_eq!(both.host_tiles, 2);
		assert_eq!(both.rows.iter().map(|row| row.tiles).sum::<usize>(), 2);
		assert!(both
			.rows
			.iter()
			.any(|row| row.family == "HighBush" && row.reason == "active" && row.tiles == 1));
		assert!(both.rows.iter().any(|row| row.family == "CommonTufts"
			&& row.reason == "unsupported_adapter"
			&& row.tiles == 1));
		world.entity_mut(entity).remove::<ChicoGroveHost>();
		let active_only = PackedAdmissionSnapshot::read(&mut world);
		assert_eq!(active_only.host_tiles, 2);
		assert_eq!(active_only.rows.iter().map(|row| row.tiles).sum::<usize>(), 2);
		Ok(())
	}
}
