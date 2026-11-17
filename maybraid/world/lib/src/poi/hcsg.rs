//! World POIs as [`PoiOrigin`] schemes over existing HCSG grids.
//!
//! Distant-tile journeying requests existing PoiSet schemes through the shared
//! HCSG worker, then polls completed published values without generating in Update.
//! Synchronous collectors remain available for parity tests and timing fixtures.

use std::marker::PhantomData;

use bevy::math::{DVec3, IVec2, Vec3};
use bevy::shape::Aabb3d;
use chico::ChicoGrove;
use geneva::{DevelopmentPlaces, LanguageGround};
use lod::gen::{Id, OriginalId};
use lod::hcsg::{Busy, GenerationContext, HcsgDemand, HcsgStorage};
use mob_characters::{LOCAL_POI, SALOON_POI, URBAN_POI, VEGETATION_POI};
use poi_intelligence::{
	generate_matching_in_xz_tile_offline, hashed_poi_id, DistantTilePois, DistantTileRequest,
	PoiEntry, PoiInterests, PoiOrigin, PoiSet, PoiSupport, PoiTilePoll, PoiTileQuery,
	PoiTileRequest,
};
use richmond::{Built, PaddedTerrain, RichmondGround};

use super::place_kind;

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
		let mut center = (Vec3::from(bounds.min) + Vec3::from(bounds.max)) * 0.5;
		type Ground = terrain_layer_model::OnTerrain<durham::Durham>;
		center.y = PaddedTerrain::<Ground>::sample_height_in(cx, center.x, center.z)?;
		let radius = ((bounds.max.x - bounds.min.x).max(bounds.max.z - bounds.min.z) * 0.5)
			.clamp(1.0, 256.0);
		Some((
			vec![PoiEntry {
				id: hashed_poi_id(GROVE_POI_SALT, grove.id()),
				kind: VEGETATION_POI,
				position: center,
				arrival_radius: radius,
				support: PoiSupport::Terrain,
				salience: 1.25,
				local: false,
				global: true,
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
		let mut position = built.setting_at;
		position.y = PaddedTerrain::<G>::sample_height_in(cx, position.x, position.z)?;
		Some((
			vec![PoiEntry {
				id: hashed_poi_id(URBAN_POI_SALT, built.setting.id),
				kind: URBAN_POI,
				position,
				arrival_radius: built.setting.arrival_radius,
				support: PoiSupport::Terrain,
				salience: 1.5,
				local: false,
				global: true,
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
		let mut pois = Vec::with_capacity(places.places.len());
		for place in &places.places {
			let (position, support) = if let Some(arrival) = place.interior_arrival {
				(arrival, PoiSupport::Floor)
			} else {
				let x = place.xz.x;
				let z = place.xz.y;
				let y = PaddedTerrain::<W::Built>::sample_height_in(cx, x, z)?;
				(Vec3::new(x, y, z), PoiSupport::Terrain)
			};
			pois.push(PoiEntry {
				id: hashed_poi_id(
					INTERIOR_POI_SALT ^ place.place.label.salt(),
					(place.place.host, place.place.local),
				),
				kind: place_kind(place.place.label),
				position,
				arrival_radius: place.place.arrival_radius,
				support,
				salience: place.place.salience.unwrap_or(place.place.label.default_salience()),
				local: true,
				global: false,
			});
		}
		Some((pois, bounds))
	}
}

/// World tile lookup: groves, urban settings, and places on the live ground.
pub struct WorldPoiTiles;

impl DistantTilePois for WorldPoiTiles {
	fn request_in_xz_tile(&self, query: PoiTileQuery) -> Option<Box<dyn DistantTileRequest>> {
		type Ground = terrain_layer_model::OnTerrain<durham::Durham>;
		type Urban = urbanization_layer_model::Urbanization<richmond::Richmond<Ground>>;
		let mut families: Vec<Box<dyn DistantTileRequest>> = Vec::new();
		if query.interests.contains(VEGETATION_POI) {
			families.push(Box::new(PoiTileRequest::<GrovePois>::new(query.clone())));
		}
		if query.interests.contains(URBAN_POI) {
			families.push(Box::new(PoiTileRequest::<UrbanPois<Ground>>::new(query.clone())));
		}
		if query.interests.contains(LOCAL_POI) || query.interests.contains(SALOON_POI) {
			families.push(Box::new(PoiTileRequest::<PlacePois<Urban>>::new(query)));
		}
		Some(Box::new(WorldPoiRequest { families }))
	}
}

impl WorldPoiTiles {
	/// Synchronous reference generation for offline fixtures and timing only.
	/// Frame callers use the asynchronous DistantTilePois request interface.
	pub fn generate_matching_in_xz_tile_offline(
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
		let mut cx = GenerationContext::with_origin(storage, "journey-poi");
		let mut scratch = Vec::new();
		if interests.contains(VEGETATION_POI) {
			let span = bevy::log::trace_span!(
				target: "maybraid.poi.profile",
				"poi.grove_tile",
				tile_x = tile.x,
				tile_z = tile.y,
				requested = interests.contains(VEGETATION_POI),
				prior_matches = out.len(),
			);
			let _entered = span.enter();
			generate_matching_in_xz_tile_offline::<GrovePois>(
				&mut cx,
				tile,
				tile_size,
				center_y,
				interests,
				&mut scratch,
			);
			out.extend(scratch.iter().copied());
		}
		if interests.contains(URBAN_POI) {
			let span = bevy::log::trace_span!(
				target: "maybraid.poi.profile",
				"poi.urban_tile",
				tile_x = tile.x,
				tile_z = tile.y,
				requested = interests.contains(URBAN_POI),
				prior_matches = out.len(),
			);
			let _entered = span.enter();
			generate_matching_in_xz_tile_offline::<UrbanPois<Ground>>(
				&mut cx,
				tile,
				tile_size,
				center_y,
				interests,
				&mut scratch,
			);
			out.extend(scratch.iter().copied());
		}
		if interests.contains(LOCAL_POI) || interests.contains(SALOON_POI) {
			let span = bevy::log::trace_span!(
				target: "maybraid.poi.profile",
				"poi.place_tile",
				tile_x = tile.x,
				tile_z = tile.y,
				requested_local = interests.contains(LOCAL_POI),
				requested_saloon = interests.contains(SALOON_POI),
				prior_matches = out.len(),
			);
			let _entered = span.enter();
			generate_matching_in_xz_tile_offline::<PlacePois<Urban>>(
				&mut cx,
				tile,
				tile_size,
				center_y,
				interests,
				&mut scratch,
			);
			out.extend(scratch.iter().copied());
		}
		out.sort_unstable_by_key(|poi| poi.id);
		out.dedup_by_key(|poi| poi.id);
	}
}

/// Read each family independently. Available candidates never wait for another
/// family or request; completion is separate and used only for empty proof.
struct WorldPoiRequest {
	families: Vec<Box<dyn DistantTileRequest>>,
}

impl DistantTileRequest for WorldPoiRequest {
	fn release(&mut self) {
		for family in &mut self.families {
			family.release();
		}
	}

	fn poll(
		&mut self,
		storage: &HcsgStorage,
		demand: &HcsgDemand,
		out: &mut Vec<PoiEntry>,
		allow_request: bool,
	) -> Result<PoiTilePoll, Busy> {
		out.clear();
		let epoch = demand.try_epoch()?;
		let mut scratch = Vec::new();
		let mut complete = true;
		let mut capacity = false;
		let mut failed = false;
		let mut stale = false;
		for family in &mut self.families {
			scratch.clear();
			let status = family.poll(storage, demand, &mut scratch, allow_request);
			out.extend(scratch.iter().copied());
			match status {
				Ok(PoiTilePoll::Ready | PoiTilePoll::ReadyEmpty) => {}
				Ok(PoiTilePoll::Capacity) => {
					capacity = true;
					complete = false;
				}
				Ok(PoiTilePoll::Failed) => {
					failed = true;
					complete = false;
				}
				Ok(PoiTilePoll::Stale) => {
					stale = true;
					complete = false;
				}
				Ok(PoiTilePoll::Pending) | Err(_) => complete = false,
			}
		}
		match demand.try_epoch() {
			Ok(current) if current == epoch => {}
			Ok(_) => {
				out.clear();
				self.release();
				return Ok(PoiTilePoll::Stale);
			}
			Err(busy) => {
				out.clear();
				return Err(busy);
			}
		}
		if capacity {
			self.release();
		}
		out.sort_unstable_by_key(|poi| poi.id);
		out.dedup_by_key(|poi| poi.id);
		Ok(if complete {
			if out.is_empty() {
				PoiTilePoll::ReadyEmpty
			} else {
				PoiTilePoll::Ready
			}
		} else if stale {
			PoiTilePoll::Stale
		} else if failed {
			PoiTilePoll::Failed
		} else if capacity {
			PoiTilePoll::Capacity
		} else {
			PoiTilePoll::Pending
		})
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
	use bevy::math::Vec2;
	use geneva::{DevelopmentPlace, DevelopmentPlaces, NameKey};
	use lod::hcsg::HcsgStorage;
	use poi_intelligence::PoiSupport;
	use richmond::{DiscoverablePlace, DiscoverablePlaceLabel};
	use std::sync::Arc;

	type Ground = terrain_layer_model::OnTerrain<durham::Durham>;
	type Urban = urbanization_layer_model::Urbanization<richmond::Richmond<Ground>>;

	#[test]
	fn authored_floor_arrivals_keep_each_storey_height() -> anyhow::Result<()> {
		let bounds = Aabb3d::from_min_max(
			Vec3::new(-10.0, -100_000.0, -10.0),
			Vec3::new(10.0, 100_000.0, 10.0),
		);
		let host = Id::from_cell(bounds);
		let make = |local, y| DevelopmentPlace {
			key: geneva::NameKey::Place { host, local },
			place: DiscoverablePlace::host(DiscoverablePlaceLabel::Storey, 8.0, 1.1)
				.with_identity(host, local),
			xz: Vec2::new(2.0, 3.0),
			y,
			interior_arrival: Some(Vec3::new(2.0, y, 3.0)),
			building: None,
		};
		let storage = HcsgStorage::default();
		storage.publish(
			host,
			Arc::new(DevelopmentPlaces::<Urban>::new(vec![make(1, 4.0), make(2, 12.0)])),
			bounds,
		);
		let mut cx = GenerationContext::new(&storage);
		let (entries, _) = PlacePois::<Urban>::pois(&mut cx, host)
			.ok_or_else(|| anyhow::anyhow!("stored floor places should resolve"))?;
		assert_eq!(entries.len(), 2);
		assert_eq!(entries.iter().map(|poi| poi.position.y).collect::<Vec<_>>(), [4.0, 12.0]);
		assert!(entries.iter().all(|poi| poi.support == PoiSupport::Floor));
		Ok(())
	}

	#[test]
	fn outdoor_places_wait_for_published_padded_ground() -> anyhow::Result<()> {
		let bounds = Aabb3d::from_min_max(
			Vec3::new(-10.0, -100_000.0, -10.0),
			Vec3::new(10.0, 100_000.0, 10.0),
		);
		let host = Id::from_cell(bounds);
		let place = DevelopmentPlace {
			key: NameKey::Place { host, local: 4 },
			place: DiscoverablePlace::host(DiscoverablePlaceLabel::House, 8.0, 1.1)
				.with_identity(host, 4),
			xz: Vec2::new(2.0, 3.0),
			y: 7.0,
			interior_arrival: None,
			building: None,
		};
		let storage = HcsgStorage::default();
		storage.publish(host, Arc::new(DevelopmentPlaces::<Urban>::new(vec![place])), bounds);
		let mut cx = GenerationContext::new(&storage);
		assert!(PlacePois::<Urban>::pois(&mut cx, host).is_none());
		Ok(())
	}
}

#[cfg(test)]
mod interest_tests;

#[cfg(test)]
mod async_tests;
