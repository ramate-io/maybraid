//! Compare dispatch against the unconditional collector using live Richmond schemes.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use bevy::log::tracing::{self, span::Attributes, span::Id as SpanId, Subscriber};
use bevy::log::tracing_subscriber::{layer::Context, prelude::*, Layer};
use bevy::math::{IVec2, UVec2, Vec2, Vec3};
use bevy::shape::Aabb3d;
use chico::{ForestSelection, LayeringKind};
use durham::{BaseTerrainNoise, Terrain, TerrainCellLayout, TerrainConfig};
use lod::hcsg::{universal_bounds, GenerationContext, HcsgStorage};
use mob_characters::{CHARACTER_POI, LOCAL_POI, SALOON_POI, URBAN_POI, VEGETATION_POI};
use poi_intelligence::{
	collect_matching_in_xz_tile, DistantTilePois, PoiEntry, PoiInterest, PoiInterests,
};
use richmond::{
	AuthoredDevelopment, AuthoredDevelopments, DevelopmentConfig, DevelopmentKind, DevelopmentSites,
};
use urbanization_cells::UrbanizationSelection;

use crate::poi::hcsg::{GrovePois, PlacePois, UrbanPois, WorldPoiTiles};

type Ground = terrain_layer_model::OnTerrain<durham::Durham>;
type Urban = urbanization_layer_model::Urbanization<richmond::Richmond<Ground>>;

const TILE_SIZE: f32 = 256.0;

struct Fixture {
	storage: HcsgStorage,
}

impl Fixture {
	/// Real authored Richmond roots, with one local building and one market.
	/// Only the underlying terrain cells are published; Built, DevelopmentPlaces,
	/// padded ground and all three PoiSets still generate through their schemes.
	fn new() -> Self {
		let storage = HcsgStorage::default();
		let layout = TerrainCellLayout {
			cell_size: 2048.0,
			origin: IVec2::splat(-1),
			extents: UVec2::splat(3),
			outer_rings: Vec::new(),
			stream_rings: Vec::new(),
			..TerrainCellLayout::default()
		};
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
		for z in -1..=1 {
			for x in -1..=1 {
				let terrain = Terrain::base_cell_for_test(&layout, x, z, base.clone());
				let cell = terrain.cell;
				storage.publish(lod::gen::Id::from_cell(cell), Arc::new(terrain), cell);
			}
		}
		storage.seed(layout, universal_bounds());
		storage.seed(
			ForestSelection { layering: Some(LayeringKind::LushJungle), ..Default::default() },
			universal_bounds(),
		);
		storage.seed(
			DevelopmentConfig { sites: DevelopmentSites::Authored, ..Default::default() },
			universal_bounds(),
		);
		storage.seed(UrbanizationSelection::default(), universal_bounds());
		storage.seed(
			AuthoredDevelopments(vec![
				Self::development(
					Vec2::new(16.0, 16.0),
					Vec2::new(144.0, 144.0),
					DevelopmentKind::LesHalles,
				),
				Self::development(
					Vec2::new(272.0, 16.0),
					Vec2::new(496.0, 240.0),
					DevelopmentKind::OldCityMarket,
				),
			]),
			universal_bounds(),
		);
		Self { storage }
	}

	fn development(min: Vec2, max: Vec2, kind: DevelopmentKind) -> AuthoredDevelopment {
		AuthoredDevelopment {
			cell: Aabb3d::from_min_max(Vec3::new(min.x, 0.0, min.y), Vec3::new(max.x, 1.0, max.y)),
			kinds: vec![kind],
			height: 0.0,
			config: DevelopmentConfig::default(),
			courtyard: None,
		}
	}

	fn collect(&self, lookup: &impl DistantTilePois, interests: &PoiInterests) -> Vec<PoiEntry> {
		let mut result = Vec::new();
		let mut tile_matches = Vec::new();
		for z in 0..4 {
			for x in 0..4 {
				lookup.collect_matching_in_xz_tile(
					&self.storage,
					IVec2::new(x, z),
					TILE_SIZE,
					0.0,
					interests,
					&mut tile_matches,
				);
				result.extend(tile_matches.iter().copied());
			}
		}
		result
	}
}

/// The exact pre-optimization dispatch, without the new interest guards.
struct UnconditionalWorldPoiTiles;

impl DistantTilePois for UnconditionalWorldPoiTiles {
	fn collect_matching_in_xz_tile(
		&self,
		storage: &HcsgStorage,
		tile: IVec2,
		tile_size: f32,
		center_y: f32,
		interests: &PoiInterests,
		out: &mut Vec<PoiEntry>,
	) {
		out.clear();
		let mut cx = GenerationContext::new(storage);
		let mut scratch = Vec::new();
		collect_matching_in_xz_tile::<GrovePois>(
			&mut cx,
			tile,
			tile_size,
			center_y,
			interests,
			&mut scratch,
		);
		out.extend(scratch.iter().copied());
		collect_matching_in_xz_tile::<UrbanPois<Ground>>(
			&mut cx,
			tile,
			tile_size,
			center_y,
			interests,
			&mut scratch,
		);
		out.extend(scratch.iter().copied());
		collect_matching_in_xz_tile::<PlacePois<Urban>>(
			&mut cx,
			tile,
			tile_size,
			center_y,
			interests,
			&mut scratch,
		);
		out.extend(scratch.iter().copied());
		out.sort_unstable_by_key(|poi| poi.id);
		out.dedup_by_key(|poi| poi.id);
	}
}

/// Observe actual top-level collector entry, even when that family returns no
/// POIs. Each existing family span encloses its collector and output extension;
/// this avoids adding injection hooks or fake dispatch logic to production.
#[derive(Clone, Default)]
struct FamilyCalls(Arc<[AtomicUsize; 3]>);

impl FamilyCalls {
	fn counts(&self) -> [usize; 3] {
		std::array::from_fn(|index| self.0[index].load(Ordering::Relaxed))
	}
}

impl<S: Subscriber> Layer<S> for FamilyCalls {
	fn on_new_span(&self, attributes: &Attributes<'_>, _id: &SpanId, _context: Context<'_, S>) {
		if attributes.metadata().target() != "maybraid.poi.profile" {
			return;
		}
		let index = match attributes.metadata().name() {
			"poi.grove_tile" => 0,
			"poi.urban_tile" => 1,
			"poi.place_tile" => 2,
			_ => return,
		};
		self.0[index].fetch_add(1, Ordering::Relaxed);
	}
}

#[test]
fn interest_dispatch_preserves_cold_and_warm_world_pois_and_skips_unrequested_collectors(
) -> anyhow::Result<()> {
	let cases = [
		("vegetation", PoiInterests::one(VEGETATION_POI), [1, 0, 0]),
		("urban", PoiInterests::one(URBAN_POI), [0, 1, 0]),
		("local", PoiInterests::one(LOCAL_POI), [0, 0, 1]),
		("saloon", PoiInterests::one(SALOON_POI), [0, 0, 1]),
		("mixed", PoiInterests::one(VEGETATION_POI).with_weight(LOCAL_POI, 1.0), [1, 0, 1]),
		(
			"all",
			PoiInterests::new([
				PoiInterest::new(VEGETATION_POI, 1.0),
				PoiInterest::new(URBAN_POI, 1.0),
				PoiInterest::new(LOCAL_POI, 1.0),
				PoiInterest::new(SALOON_POI, 1.0),
			]),
			[1, 1, 1],
		),
		("empty", PoiInterests::default(), [0, 0, 0]),
		(
			"zero",
			PoiInterests::new([
				PoiInterest::new(VEGETATION_POI, 0.0),
				PoiInterest::new(URBAN_POI, 0.0),
				PoiInterest::new(LOCAL_POI, 0.0),
				PoiInterest::new(SALOON_POI, 0.0),
			]),
			[0, 0, 0],
		),
		(
			"zero-first-duplicate",
			PoiInterests::new([PoiInterest::new(LOCAL_POI, 0.0), PoiInterest::new(LOCAL_POI, 1.0)]),
			[0, 0, 0],
		),
		("character-only", PoiInterests::one(CHARACTER_POI), [0, 0, 0]),
	];
	for (name, interests, families) in cases {
		let reference = Fixture::new();
		let optimized = Fixture::new();
		let expected_cold = reference.collect(&UnconditionalWorldPoiTiles, &interests);
		let calls = FamilyCalls::default();
		let subscriber = bevy::log::tracing_subscriber::registry().with(calls.clone());
		let (actual_cold, actual_warm) = tracing::subscriber::with_default(subscriber, || {
			(
				optimized.collect(&WorldPoiTiles, &interests),
				optimized.collect(&WorldPoiTiles, &interests),
			)
		});
		let expected_warm = reference.collect(&UnconditionalWorldPoiTiles, &interests);
		assert_eq!(actual_cold, expected_cold, "cold case {name}");
		assert_eq!(actual_warm, expected_warm, "warm case {name}");
		assert_eq!(expected_cold, expected_warm, "reference remains stable: {name}");
		assert_eq!(
			calls.counts(),
			families.map(|enabled| enabled * 32),
			"actual family dispatch: {name}"
		);
		if name == "all" {
			for kind in [VEGETATION_POI, URBAN_POI, LOCAL_POI, SALOON_POI] {
				anyhow::ensure!(
					expected_cold.iter().any(|poi| poi.kind == kind),
					"real fixture must produce {}",
					kind.name()
				);
			}
			anyhow::ensure!(
				expected_cold
					.iter()
					.any(|poi| poi.support == poi_intelligence::PoiSupport::Floor),
				"real Richmond fixture must exercise interior floor arrivals"
			);
		}
	}
	Ok(())
}
