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
	generate_matching_in_xz_tile_offline, DistantTilePois, PoiEntry, PoiInterest, PoiInterests,
};
use richmond::{
	AuthoredDevelopment, AuthoredDevelopments, Built, DevelopmentConfig, DevelopmentKind,
	DevelopmentSites,
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

	fn generate_offline(
		&self,
		lookup: &impl OfflineWorldTiles,
		interests: &PoiInterests,
	) -> Vec<PoiEntry> {
		let mut result = Vec::new();
		let mut tile_matches = Vec::new();
		for z in 0..4 {
			for x in 0..4 {
				lookup.generate_matching_in_xz_tile_offline(
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

/// Test-only synchronous reference dispatch; production lookup traits expose no
/// synchronous generation entry point.
trait OfflineWorldTiles {
	fn generate_matching_in_xz_tile_offline(
		&self,
		storage: &HcsgStorage,
		tile: IVec2,
		tile_size: f32,
		center_y: f32,
		interests: &PoiInterests,
		out: &mut Vec<PoiEntry>,
	);
}

impl OfflineWorldTiles for WorldPoiTiles {
	fn generate_matching_in_xz_tile_offline(
		&self,
		storage: &HcsgStorage,
		tile: IVec2,
		tile_size: f32,
		center_y: f32,
		interests: &PoiInterests,
		out: &mut Vec<PoiEntry>,
	) {
		WorldPoiTiles::generate_matching_in_xz_tile_offline(
			self, storage, tile, tile_size, center_y, interests, out,
		);
	}
}

/// The exact pre-optimization dispatch, without the new interest guards.
struct UnconditionalWorldPoiTiles;

impl OfflineWorldTiles for UnconditionalWorldPoiTiles {
	fn generate_matching_in_xz_tile_offline(
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
		generate_matching_in_xz_tile_offline::<GrovePois>(
			&mut cx,
			tile,
			tile_size,
			center_y,
			interests,
			&mut scratch,
		);
		out.extend(scratch.iter().copied());
		generate_matching_in_xz_tile_offline::<UrbanPois<Ground>>(
			&mut cx,
			tile,
			tile_size,
			center_y,
			interests,
			&mut scratch,
		);
		out.extend(scratch.iter().copied());
		generate_matching_in_xz_tile_offline::<PlacePois<Urban>>(
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
		let expected_cold = reference.generate_offline(&UnconditionalWorldPoiTiles, &interests);
		let calls = FamilyCalls::default();
		let subscriber = bevy::log::tracing_subscriber::registry().with(calls.clone());
		let (actual_cold, actual_warm) = tracing::subscriber::with_default(subscriber, || {
			(
				optimized.generate_offline(&WorldPoiTiles, &interests),
				optimized.generate_offline(&WorldPoiTiles, &interests),
			)
		});
		let expected_warm = reference.generate_offline(&UnconditionalWorldPoiTiles, &interests);
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

#[test]
fn worker_requests_preserve_exact_ordered_real_family_output() -> anyhow::Result<()> {
	use lod::hcsg::{spawn_worker, HcsgDemand};
	use poi_intelligence::{PoiTilePoll, PoiTileQuery};
	use std::time::{Duration, Instant};

	for interests in [
		PoiInterests::one(VEGETATION_POI),
		PoiInterests::one(URBAN_POI),
		PoiInterests::new([PoiInterest::new(LOCAL_POI, 1.0), PoiInterest::new(SALOON_POI, 1.0)]),
		PoiInterests::new([
			PoiInterest::new(VEGETATION_POI, 1.0),
			PoiInterest::new(URBAN_POI, 1.0),
			PoiInterest::new(LOCAL_POI, 1.0),
			PoiInterest::new(SALOON_POI, 1.0),
		]),
	] {
		let tile = IVec2::ZERO;
		let reference = Fixture::new();
		let mut expected = Vec::new();
		WorldPoiTiles.generate_matching_in_xz_tile_offline(
			&reference.storage,
			tile,
			TILE_SIZE,
			0.0,
			&interests,
			&mut expected,
		);
		let fixture = Fixture::new();
		let demand = HcsgDemand::default();
		// Match the live world's terrain subscription. The fixture publishes
		// terrain directly rather than seeding its full generation roots, so
		// retain that input across the worker's initial reach-based sweep.
		let _terrain = demand.subscribe::<Terrain>(
			None,
			vec![poi_intelligence::xz_tile_region(tile, TILE_SIZE, 0.0)
				.ok_or_else(|| anyhow::anyhow!("terrain retention region"))?],
			None,
			lod::hcsg::HcsgClass::Near,
		);
		let _worker = spawn_worker(fixture.storage.clone(), demand.clone())?;
		assert!(demand.wait_idle(Duration::from_secs(30)));
		let terrain_region = poi_intelligence::xz_tile_region(tile, TILE_SIZE, 0.0)
			.ok_or_else(|| anyhow::anyhow!("terrain query region"))?;
		assert!(
			!fixture.storage.overlapping::<Terrain>(terrain_region).is_empty(),
			"fixture terrain must survive the initial worker sweep"
		);
		assert!(fixture
			.storage
			.overlapping::<poi_intelligence::PoiSet<GrovePois>>(terrain_region)
			.is_empty());
		assert!(fixture.storage.overlapping::<Built<Ground>>(terrain_region).is_empty());
		let mut request = WorldPoiTiles
			.request_in_xz_tile(PoiTileQuery {
				tile,
				tile_size: TILE_SIZE,
				center_y: 0.0,
				interests,
			})
			.ok_or_else(|| anyhow::anyhow!("world request"))?;
		let mut actual = Vec::new();
		let deadline = Instant::now() + Duration::from_secs(30);
		loop {
			match request.poll(&fixture.storage, &demand, &mut actual, true) {
				Ok(PoiTilePoll::Ready | PoiTilePoll::ReadyEmpty) => break,
				Ok(PoiTilePoll::Failed | PoiTilePoll::Stale) => {
					anyhow::bail!("request failed or went stale")
				}
				_ => anyhow::ensure!(Instant::now() < deadline, "request did not complete"),
			}
			assert!(demand.wait_idle(Duration::from_secs(30)));
		}
		assert_eq!(actual, expected, "cold worker output must match synchronous fixture exactly");
		assert!(matches!(
			request.poll(&fixture.storage, &demand, &mut actual, true),
			Ok(PoiTilePoll::Ready | PoiTilePoll::ReadyEmpty)
		));
		assert_eq!(
			actual, expected,
			"warm output preserves IDs, ordering, support and destinations"
		);
	}
	Ok(())
}
