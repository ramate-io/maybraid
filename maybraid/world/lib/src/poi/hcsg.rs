//! World POIs as [`PoiOrigin`] schemes over existing HCSG grids.
//!
//! Distant-tile journeying calls [`DistantTilePois::collect_matching_in_xz_tile`],
//! which resolves each family through `get_or_generate_in` — O(tile), not a
//! walk of every global POI.

use std::marker::PhantomData;

use bevy::math::bounding::Aabb3d;
use bevy::math::{DVec3, IVec2, Vec3, Vec3Swizzles};
use chico::ChicoGrove;
use geneva::{DevelopmentPlaces, LanguageGround};
use lod::gen::{Id, OriginalId};
use lod::hcsg::{GenerationContext, HcsgStorage};
use mob_characters::{URBAN_POI, VEGETATION_POI};
use poi_intelligence::{
	collect_matching_in_xz_tile, hashed_poi_id, DistantTilePois, PoiEntry, PoiInterests, PoiOrigin,
	PoiSet, PoiSupport,
};
use richmond::{Built, PaddedTerrain, RichmondGround};
use terrain_layer_model::OnTerrain;

use super::{place_kind, support_for_pin};

const GROVE_POI_SALT: u64 = 0x6772_6f76_652d_706f;
const URBAN_POI_SALT: u64 = 0x7572_6261_6e2d_706f;
const INTERIOR_POI_SALT: u64 = 0x696e_7465_7269_6f72;

/// One vegetation destination per [`ChicoGrove`].
pub struct GrovePois;

impl PoiOrigin for GrovePois {
	const INDEX_SCALE: DVec3 = DVec3::splat(256.0);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<ChicoGrove>(region)
	}

	fn pois(cx: &mut GenerationContext, id: Id) -> Option<(Vec<PoiEntry>, Aabb3d)> {
		let (grove, bounds) = cx.get_or_generate_stored::<ChicoGrove>(id)?;
		let center = (Vec3::from(bounds.min) + Vec3::from(bounds.max)) * 0.5;
		let radius = ((bounds.max.x - bounds.min.x).max(bounds.max.z - bounds.min.z) * 0.5)
			.clamp(1.0, 256.0);
		let elevation = PaddedTerrain::<OnTerrain<durham::Durham>>::sample_in(cx, center.xz())
			.unwrap_or(center.y);
		Some((
			vec![PoiEntry {
				id: hashed_poi_id(GROVE_POI_SALT, grove.id()),
				kind: VEGETATION_POI,
				position: Vec3::new(center.x, elevation, center.z),
				arrival_radius: radius,
				salience: 1.25,
				local: false,
				global: true,
				support: PoiSupport::Terrain,
			}],
			bounds,
		))
	}
}

/// One urban destination per filled [`Built`] development.
pub struct UrbanPois<G>(PhantomData<fn() -> G>);

impl<G: RichmondGround> PoiOrigin for UrbanPois<G> {
	const INDEX_SCALE: DVec3 = DVec3::splat(256.0);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<Built<G>>(region)
	}

	fn pois(cx: &mut GenerationContext, id: Id) -> Option<(Vec<PoiEntry>, Aabb3d)> {
		let (built, bounds) = cx.get_or_generate_stored::<Built<G>>(id)?;
		let xz = built.setting_at.xz();
		let elevation = PaddedTerrain::<G>::sample_in(cx, xz).unwrap_or(built.setting_at.y);
		Some((
			vec![PoiEntry {
				id: hashed_poi_id(URBAN_POI_SALT, built.setting.id),
				kind: URBAN_POI,
				position: Vec3::new(built.setting_at.x, elevation, built.setting_at.z),
				arrival_radius: built.setting.arrival_radius,
				salience: 1.5,
				local: false,
				global: true,
				support: PoiSupport::Terrain,
			}],
			bounds,
		))
	}
}

/// Local discoverable places of one development, from [`DevelopmentPlaces`].
pub struct PlacePois<W>(PhantomData<fn() -> W>);

impl<W: LanguageGround> PoiOrigin for PlacePois<W> {
	const INDEX_SCALE: DVec3 = DVec3::splat(256.0);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<DevelopmentPlaces<W>>(region)
	}

	fn pois(cx: &mut GenerationContext, id: Id) -> Option<(Vec<PoiEntry>, Aabb3d)> {
		let (places, bounds) = cx.get_or_generate_stored::<DevelopmentPlaces<W>>(id)?;
		let pois = places
			.places
			.iter()
			.map(|place| {
				let kind = place_kind(place.place.label);
				let (position, support) = match place.interior_arrival {
					Some(arrival) => (arrival, support_for_pin(kind, true)),
					None => {
						let elevation =
							PaddedTerrain::<W::Built>::sample_in(cx, place.xz).unwrap_or(place.y);
						(Vec3::new(place.xz.x, elevation, place.xz.y), PoiSupport::Terrain)
					}
				};
				PoiEntry {
					id: hashed_poi_id(
						INTERIOR_POI_SALT ^ place.place.label.salt(),
						(place.place.host, place.place.local),
					),
					kind,
					position,
					arrival_radius: place.place.arrival_radius,
					salience: place.place.salience.unwrap_or(place.place.label.default_salience()),
					local: true,
					global: false,
					support,
				}
			})
			.collect();
		Some((pois, bounds))
	}
}

/// World tile lookup: groves, urban settings, and places on the live ground.
pub struct WorldPoiTiles;

impl DistantTilePois for WorldPoiTiles {
	fn collect_matching_in_xz_tile(
		&self,
		storage: &HcsgStorage,
		tile: IVec2,
		tile_size: f32,
		center_y: f32,
		interests: &PoiInterests,
		out: &mut Vec<PoiEntry>,
	) {
		type Ground = terrain_layer_model::OnTerrain<durham::Durham>;
		type Urban = urbanization_layer_model::Urbanization<richmond::Richmond<Ground>>;

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

/// Keeps world [`PoiSet`]s warm in the same neighborhoods that present groves
/// and developments, so distant-tile probes often hit already-published values.
pub fn register_world_poi_generation(app: &mut bevy::prelude::App) {
	use chico::GroveNeighborhood;
	use lod::hcsg::GenerationPlugin;
	use maybraid_game_mode_discover::InDiscovery;
	use richmond::DevelopmentNeighborhood;

	type Ground = terrain_layer_model::OnTerrain<durham::Durham>;
	type Urban = urbanization_layer_model::Urbanization<richmond::Richmond<Ground>>;

	if !app.is_plugin_added::<GenerationPlugin<InDiscovery<GroveNeighborhood>, PoiSet<GrovePois>>>()
	{
		app.add_plugins((
			GenerationPlugin::<InDiscovery<GroveNeighborhood>, PoiSet<GrovePois>>::default(),
			GenerationPlugin::<InDiscovery<DevelopmentNeighborhood>, PoiSet<UrbanPois<Ground>>>::default(),
			GenerationPlugin::<InDiscovery<DevelopmentNeighborhood>, PoiSet<PlacePois<Urban>>>::default(),
		));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::bounding::Aabb3d;
	use bevy::math::{Vec2, Vec3, Vec3Swizzles};
	use bevy::prelude::Handle;
	use durham::{
		fine_patch_cell_layout, mesh_assets, Durham, Terrain, TerrainConfig, TerrainCoverage,
		TerrainStampConfigs, WatershedConfigs,
	};
	use geneva::{DevelopmentPlace, DevelopmentPlaces, NameKey};
	use lod::gen::Id;
	use lod::hcsg::{universal_bounds, GenerationContext, HcsgStorage};
	use poi_intelligence::PoiOrigin;
	use richmond::{DevelopmentConfig, DevelopmentSites};
	use std::sync::Arc;
	use urbanization_cells::{UrbanizationKind, UrbanizationSelection};

	type Ground = OnTerrain<Durham>;
	type Urban = urbanization_layer_model::Urbanization<richmond::Richmond<Ground>>;

	fn seed_urban_patch(seed: u32, origin: bevy::math::IVec2) -> HcsgStorage {
		let storage = HcsgStorage::default();
		let layout = fine_patch_cell_layout(1, origin);
		storage.seed(layout, universal_bounds());
		storage.seed(TerrainStampConfigs::from_world_seed(seed), universal_bounds());
		storage.seed(WatershedConfigs::default().with_seed(seed), universal_bounds());
		storage.seed(
			mesh_assets(TerrainConfig::new(seed), Handle::default(), TerrainCoverage::FinePatch, 1),
			universal_bounds(),
		);
		storage.seed(
			DevelopmentConfig {
				sites: DevelopmentSites::Lattice,
				seed,
				..DevelopmentConfig::default()
			},
			universal_bounds(),
		);
		storage.seed(
			UrbanizationSelection {
				kind: Some(UrbanizationKind::Frontier),
				..UrbanizationSelection::default()
			},
			universal_bounds(),
		);
		storage
	}

	#[test]
	fn place_pois_keep_distinct_storey_arrivals_and_outdoor_padded_fallback() {
		let bounds = Aabb3d::from_min_max(Vec3::new(0.0, -8.0, 0.0), Vec3::new(40.0, 24.0, 40.0));
		let id = Id::from_cell(bounds);
		let place = |label, local, xz, y, interior| DevelopmentPlace {
			key: NameKey::Place { host: id, local },
			place: richmond::DiscoverablePlace::high(label, 3.0, 1.1).with_identity(id, local),
			xz,
			y,
			interior_arrival: interior,
			building: None,
		};
		let ground = Vec3::new(8.0, 4.0, 6.0);
		let loft = Vec3::new(8.0, 12.0, 6.0);
		let outdoor_xz = Vec2::new(20.0, 20.0);
		let places = DevelopmentPlaces::<Urban>::new(vec![
			place(richmond::DiscoverablePlaceLabel::Storey, 1, ground.xz(), ground.y, Some(ground)),
			place(richmond::DiscoverablePlaceLabel::Bedroom, 2, loft.xz(), loft.y, Some(loft)),
			place(richmond::DiscoverablePlaceLabel::House, 3, outdoor_xz, 1.0, None),
		]);
		let storage = HcsgStorage::default();
		storage.publish(id, Arc::new(places), bounds);
		let mut cx = GenerationContext::new(&storage);
		let (pois, _) = PlacePois::<Urban>::pois(&mut cx, id).expect("places");
		let ground_poi = pois.iter().find(|poi| poi.position == ground).expect("ground storey");
		let loft_poi = pois.iter().find(|poi| poi.position == loft).expect("loft");
		assert_eq!(ground_poi.support, PoiSupport::Floor);
		assert_eq!(loft_poi.support, PoiSupport::Floor);
		assert_ne!(ground_poi.position.y, loft_poi.position.y);
		let outdoor = pois.iter().find(|poi| poi.position.xz() == outdoor_xz).expect("outdoor");
		assert_eq!(outdoor.support, PoiSupport::Terrain);
		assert_eq!(outdoor.position.y, 1.0);
	}

	#[test]
	fn urban_pois_use_padded_height_not_durham() -> anyhow::Result<()> {
		use durham::CellTiling;
		use lod::gen::OriginalId;

		let seed = 42u32;
		let origin = bevy::math::IVec2::new(-1, -1);
		let storage = seed_urban_patch(seed, origin);
		let mut cx = GenerationContext::new(&storage);
		let layout = cx
			.get_or_generate::<durham::TerrainCellLayout>(Id::Universal)
			.ok_or_else(|| anyhow::anyhow!("terrain layout"))?;
		let region = layout.request_region();
		let cell_ids: Vec<Id> =
			layout.cell_ids(region).into_iter().map(|OriginalId(id)| id).collect();
		anyhow::ensure!(!cell_ids.is_empty(), "fine patch should list terrain cells");
		let mut sampled = 0usize;
		for id in cell_ids {
			let Some(padded) = cx.get_or_generate::<PaddedTerrain<Ground>>(id) else {
				continue;
			};
			let center =
				(Vec3::from(padded.surface.cell.min) + Vec3::from(padded.surface.cell.max)) * 0.5;
			let xz = center.xz();
			let expected = padded.height_at(xz.x, xz.y);
			let sampled_y = PaddedTerrain::<Ground>::sample_in(&mut cx, xz)
				.ok_or_else(|| anyhow::anyhow!("sample_in missed generated cell {id:?}"))?;
			anyhow::ensure!(
				(sampled_y - expected).abs() < 1e-3,
				"sample_in {sampled_y} vs height_at {expected} at {xz:?}"
			);
			if let Some(terrain) = cx.get_or_generate::<Terrain>(id) {
				let durham = terrain.sdf.terrain().height_at_with_all_modulations(xz.x, xz.y);
				let sentinel = durham + 1234.0;
				anyhow::ensure!(
					(sampled_y - sentinel).abs() > 1.0,
					"padded sample must not be a Durham sentinel"
				);
			}
			sampled += 1;
		}
		anyhow::ensure!(sampled > 0, "no padded cells generated");
		for OriginalId(id) in UrbanPois::<Ground>::original_ids_for(&mut cx, region) {
			let Some((pois, _)) = UrbanPois::<Ground>::pois(&mut cx, id) else {
				continue;
			};
			for poi in pois {
				let padded = PaddedTerrain::<Ground>::sample_in(&mut cx, poi.position.xz())
					.ok_or_else(|| anyhow::anyhow!("padded sample at {:?}", poi.position.xz()))?;
				anyhow::ensure!((poi.position.y - padded).abs() < 1e-3);
				anyhow::ensure!(poi.support == PoiSupport::Terrain);
			}
		}
		Ok(())
	}
}
