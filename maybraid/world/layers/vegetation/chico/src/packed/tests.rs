//! Packed orchard CPU path: grouping, cache reuse, band select, visual leaves.

use anyhow::Result;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Transform, Vec3};
use lod::gen::{Id, Version};
use lod::lod_ref::LodRef;
use lod::{LodSceneLevel, VisualLodKind, VisualLodScene};
use vegetation_groves::{FlatTerrainSample, GroveExtent, OrchardParams};

use crate::host::ChicoGroveHost;
use crate::kind::ForestLayer;
use crate::packed::cache::{ground_fingerprint, PackedGroveCache, Reactivation};
use crate::packed::instances::{pack_orchard_batches, packed_kind_count, visual_chunks_for_orchard};
use crate::packed::mode::PackMode;
use crate::packed::select::{select_band, selected_primitives, PackedGroveSelection, SelectedTile};
use crate::{ForestGroveKind, ForestGroveTile};

fn grown_orchard() -> vegetation_groves::Orchard {
	OrchardParams::default()
		.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0)))
		.with_tree_variants(4)
		.build_on(&FlatTerrainSample::default())
}

fn dummy_lod() -> (Transform, Aabb3d) {
	(Transform::from_translation(Vec3::new(0.0, 20.0, 40.0)), Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE))
}

#[test]
fn pack_mode_parse_accepts_orchard_aliases() -> Result<()> {
	assert_eq!(PackMode::parse("orchard"), PackMode::Orchard);
	assert_eq!(PackMode::parse("1"), PackMode::Orchard);
	assert_eq!(PackMode::parse("true"), PackMode::Orchard);
	assert_eq!(PackMode::parse("off"), PackMode::Off);
	assert!(PackMode::Orchard.packs_orchard());
	assert!(PackMode::Off.is_off());
	Ok(())
}

#[test]
fn orchard_high_batches_share_kits_and_have_instances() -> Result<()> {
	let orchard = grown_orchard();
	anyhow::ensure!(!orchard.plants.is_empty(), "grown orchard has plants");
	let batches = pack_orchard_batches(&orchard, LodSceneLevel::High);
	anyhow::ensure!(!batches.is_empty(), "high packs at least one kit group");
	assert!(batches.iter().all(|batch| !batch.instances.is_empty()));
	assert!(batches.iter().all(|batch| batch.key.kit.ends_with(".glb")));
	let chunks = visual_chunks_for_orchard(&orchard, LodSceneLevel::High);
	assert!(packed_kind_count(&chunks) >= 1);
	let selection = PackedGroveSelection {
		tiles: vec![SelectedTile {
			id: Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE)),
			level: LodSceneLevel::High,
			batches: batches.clone(),
			upload: true,
		}],
		..PackedGroveSelection::default()
	};
	assert_eq!(selected_primitives(&selection).len(), batches.len());
	Ok(())
}

#[test]
fn cache_reuses_unchanged_revision() -> Result<()> {
	let orchard = grown_orchard();
	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	let mut cache = PackedGroveCache::with_budget_bytes(64 * 1024 * 1024);
	assert_eq!(
		cache.ensure(id, Version(1), &orchard, ForestLayer::UpperCanopy, 1),
		Reactivation::Fresh
	);
	assert_eq!(
		cache.ensure(id, Version(1), &orchard, ForestLayer::UpperCanopy, 2),
		Reactivation::Retained
	);
	assert_eq!(
		cache.ensure(id, Version(2), &orchard, ForestLayer::UpperCanopy, 3),
		Reactivation::Rebuilt
	);
	let first = ground_fingerprint(Version(1), &orchard, ForestLayer::UpperCanopy);
	let same = ground_fingerprint(Version(1), &orchard, ForestLayer::UpperCanopy);
	assert_eq!(first, same);
	assert_ne!(first, ground_fingerprint(Version(2), &orchard, ForestLayer::UpperCanopy));
	Ok(())
}

#[test]
fn cull_does_not_drop_cached_tile() -> Result<()> {
	let orchard = grown_orchard();
	let id = Id::from_cell(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE));
	let mut cache = PackedGroveCache::with_budget_bytes(64 * 1024 * 1024);
	cache.ensure(id, Version(1), &orchard, ForestLayer::UpperCanopy, 1);
	assert_eq!(cache.len(), 1);
	cache.evict_to_budget();
	assert_eq!(cache.len(), 1, "cull / budget leave a single resident tile");
	Ok(())
}

#[test]
fn select_band_matches_orchard_factors() -> Result<()> {
	let center = Vec3::new(50.0, 4.0, 50.0);
	let radius = 50.0;
	assert_eq!(select_band(center, radius, Vec3::new(50.0, 2.0, 80.0)), LodSceneLevel::High);
	assert_eq!(select_band(center, radius, Vec3::new(50.0, 2.0, 420.0)), LodSceneLevel::Medium);
	assert_eq!(select_band(center, radius, Vec3::new(50.0, 2.0, 700.0)), LodSceneLevel::Low);
	assert_eq!(select_band(center, radius, Vec3::new(50.0, 2.0, 1600.0)), LodSceneLevel::UltraLow);
	Ok(())
}

#[test]
fn visual_scene_emits_packed_leaves_for_orchard() -> Result<()> {
	let orchard = grown_orchard();
	let tile = ForestGroveTile::Orchard(orchard);
	let host = ChicoGroveHost::new(tile, ForestLayer::UpperCanopy);
	let (xf, bounds) = dummy_lod();
	let lod_ref = LodRef {
		entity: bevy::prelude::Entity::PLACEHOLDER,
		previous_transform: &xf,
		current_transform: &xf,
		bounds: &bounds,
	};
	let chunks = host.visual_chunks_with_level(&lod_ref, LodSceneLevel::High);
	if PackMode::current().packs_orchard() {
		assert!(packed_kind_count(&chunks) >= 1);
		let prims = chunks.into_primitives();
		assert!(matches!(prims[0].1.kind, VisualLodKind::PackedBatch { .. }));
	} else {
		assert_eq!(packed_kind_count(&chunks), 0);
	}
	let _ = ForestGroveKind::Orchard;
	Ok(())
}
