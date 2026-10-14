//! Generated POIs as an HCSG [`GenerationScheme`].
//!
//! Nearby and xz-tile lookups go through [`GenerationContext::get_or_generate_in`]:
//! a source's `original_ids_for` is the discovery set for that region, and
//! `build_with_id` resolves one source cell. [`HcsgStorage`] is the index;
//! there is no side list or extra spatial structure for this path.

use std::marker::PhantomData;
use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::math::DVec3;
use bevy::prelude::*;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext, HcsgStorage};

use crate::hash::unit_f32;
use crate::{
	NearbyChoice, NearbyQuery, PoiId, PoiInterests, PoiKind, PoiObservation, PoiSource,
	MAX_POI_ARRIVAL_RADIUS,
};

const MAX_TILE_QUERY: f32 = 1_000.0;
const MAX_NEARBY_QUERY_RADIUS: f32 = 1_000.0;
/// XZ-tile discovery is a vertical column: origins that only overlap on XZ
/// must still be found when the probe Y is far from the pin.
const XZ_COLUMN_Y: f32 = 1.0e9;

/// One generated POI pin. Not an ECS entity; pair with [`Self::observation`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoiEntry {
	pub id: PoiId,
	pub kind: PoiKind,
	pub position: Vec3,
	pub arrival_radius: f32,
	pub salience: f32,
	pub local: bool,
	pub global: bool,
}

impl PoiEntry {
	pub fn observation(self, user: Entity, source: PoiSource) -> PoiObservation {
		PoiObservation {
			user,
			id: self.id,
			entity: None,
			kind: self.kind,
			position: self.position,
			arrival_radius: self.arrival_radius,
			salience: self.salience,
			confidence: 1.0,
			source,
		}
	}

	pub fn discovery_source(self) -> PoiSource {
		if self.global {
			PoiSource::GLOBAL_SCAN
		} else {
			PoiSource::LOCAL_SCAN
		}
	}
}

/// How one family of world POIs is discovered and built.
///
/// Mirror of Geneva's [`geneva::NameSource`]: reuse an existing grid's ids,
/// then build the pins that sit on that id.
pub trait PoiOrigin: Send + Sync + 'static {
	const INDEX_SCALE: DVec3;

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId>;

	fn pois(cx: &mut GenerationContext, id: Id) -> Option<(Vec<PoiEntry>, Aabb3d)>;
}

/// The generated POIs of origin `S`, stored under `S`'s ids.
pub struct PoiSet<S> {
	pub pois: Vec<PoiEntry>,
	_source: PhantomData<fn() -> S>,
}

impl<S: PoiOrigin> hcsg::GenerationScheme for PoiSet<S> {
	const INDEX_SCALE: DVec3 = S::INDEX_SCALE;
	const RETENTION_MARGIN: DVec3 = S::INDEX_SCALE;

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		S::original_ids_for(cx, region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let (pois, bounds) = S::pois(cx, id)?;
		Some((Self { pois, _source: PhantomData }, bounds))
	}
}

/// Resolve POIs for xz tile `tile` through [`PoiSet<S>`].
///
/// Discovery is `S::original_ids_for` on the tile column — O(tile), not a
/// walk of every generated or authored POI in the world.
pub fn collect_matching_in_xz_tile<S: PoiOrigin>(
	cx: &mut GenerationContext,
	tile: IVec2,
	tile_size: f32,
	center_y: f32,
	interests: &PoiInterests,
	out: &mut Vec<PoiEntry>,
) {
	out.clear();
	if interests.is_empty() {
		return;
	}
	let Some(region) = xz_tile_region(tile, tile_size, center_y) else {
		return;
	};
	let size = normalized_tile_size(tile_size);
	for set in cx.get_or_generate_in::<PoiSet<S>>(region) {
		for poi in &set.pois {
			if interests.contains(poi.kind) && xz_tile(poi.position, size) == tile {
				out.push(*poi);
			}
		}
	}
	out.sort_unstable_by_key(|poi| poi.id);
}

/// Resolve POIs inside [`NearbyQuery`] through [`PoiSet<S>`].
///
/// Discovery is `S::original_ids_for` on the bounded region — O(origins in
/// the window), not a walk of every generated POI. Distance uses 3D overlap
/// with arrival radius; the inner hole is XZ, matching [`crate::PoiRegistry`].
pub fn collect_matching_nearby<S: PoiOrigin>(
	cx: &mut GenerationContext,
	center: Vec3,
	query: NearbyQuery,
	interests: &PoiInterests,
	excluded: &[PoiId],
	out: &mut Vec<PoiEntry>,
) {
	out.clear();
	if interests.is_empty() || !center.is_finite() || !query.radius.is_finite() {
		return;
	}
	let Some(region) = nearby_region(center, query.radius) else {
		return;
	};
	let radius = query.radius.clamp(0.0, MAX_NEARBY_QUERY_RADIUS);
	let min_radius = query.min_radius.max(0.0);
	for set in cx.get_or_generate_in::<PoiSet<S>>(region) {
		for poi in &set.pois {
			if nearby_candidate(center, radius, min_radius, interests, poi) {
				out.push(*poi);
			}
		}
	}
	out.sort_unstable_by_key(|poi| poi.id);
	drop_excluded_entries(out, excluded);
}

/// Choose one generated POI inside [`NearbyQuery`] using the same policies as
/// [`crate::PoiRegistry::choose_in`].
pub fn choose_matching_nearby<S: PoiOrigin>(
	cx: &mut GenerationContext,
	center: Vec3,
	query: NearbyQuery,
	interests: &PoiInterests,
	excluded: &[PoiId],
	seed: u64,
) -> Option<PoiEntry> {
	let mut candidates = Vec::new();
	collect_matching_nearby::<S>(cx, center, query, interests, excluded, &mut candidates);
	choose_nearby_entry(center, query, interests, &candidates, seed)
}

/// Authored / test POIs seeded at [`Id::Universal`].
#[derive(Clone, Debug, Default)]
pub struct PoiCatalog {
	pub entries: Vec<PoiEntry>,
}

lod::seeded_root!(PoiCatalog);

/// [`PoiOrigin`] over a seeded [`PoiCatalog`].
///
/// The catalog is a session root (like other authored HCSG seeds), not a
/// runtime side index. World tiles use [`PoiOrigin`] impls that delegate to
/// grove / urban / place grids instead.
pub struct Catalog;

impl PoiOrigin for Catalog {
	const INDEX_SCALE: DVec3 = DVec3::splat(256.0);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		let Some(catalog) = cx.get::<PoiCatalog>(Id::Universal) else {
			return Vec::new();
		};
		catalog
			.entries
			.iter()
			.filter(|entry| xz_point_in_region(entry.position, region))
			.map(|entry| OriginalId(catalog_id(entry.id)))
			.collect()
	}

	fn pois(cx: &mut GenerationContext, id: Id) -> Option<(Vec<PoiEntry>, Aabb3d)> {
		let catalog = cx.get::<PoiCatalog>(Id::Universal)?;
		let entry = *catalog.entries.iter().find(|entry| catalog_id(entry.id) == id)?;
		Some((vec![entry], pin_bounds(entry.position, entry.arrival_radius)))
	}
}

/// One origin id per xz tile overlapping `region`, one POI at the tile center.
///
/// Used to measure tile lookup as O(tiles in the query), not O(world POIs).
pub struct GridTiles;

impl PoiOrigin for GridTiles {
	const INDEX_SCALE: DVec3 = DVec3::splat(256.0);

	fn original_ids_for(_cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		grid_tile_ids(region, 256.0)
	}

	fn pois(_cx: &mut GenerationContext, id: Id) -> Option<(Vec<PoiEntry>, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let center = (Vec3::from(bounds.min) + Vec3::from(bounds.max)) * 0.5;
		let tile = xz_tile(center, 256.0);
		Some((
			vec![PoiEntry {
				id: PoiId(grid_poi_id(tile)),
				kind: GRID_KIND,
				position: center,
				arrival_radius: 1.0,
				salience: 1.0,
				local: true,
				global: true,
			}],
			bounds,
		))
	}
}

pub const GRID_KIND: PoiKind = PoiKind::new("test/grid-tile");

/// Frame / test wiring: collect a tile or nearby window through [`HcsgStorage`].
pub trait DistantTilePois: Send + Sync + 'static {
	fn collect_matching_in_xz_tile(
		&self,
		storage: &HcsgStorage,
		tile: IVec2,
		tile_size: f32,
		center_y: f32,
		interests: &PoiInterests,
		out: &mut Vec<PoiEntry>,
	);

	fn collect_matching_nearby(
		&self,
		storage: &HcsgStorage,
		center: Vec3,
		query: NearbyQuery,
		interests: &PoiInterests,
		excluded: &[PoiId],
		out: &mut Vec<PoiEntry>,
	);
}

impl DistantTilePois for Catalog {
	fn collect_matching_in_xz_tile(
		&self,
		storage: &HcsgStorage,
		tile: IVec2,
		tile_size: f32,
		center_y: f32,
		interests: &PoiInterests,
		out: &mut Vec<PoiEntry>,
	) {
		let mut cx = GenerationContext::new(storage);
		collect_matching_in_xz_tile::<Catalog>(&mut cx, tile, tile_size, center_y, interests, out);
	}

	fn collect_matching_nearby(
		&self,
		storage: &HcsgStorage,
		center: Vec3,
		query: NearbyQuery,
		interests: &PoiInterests,
		excluded: &[PoiId],
		out: &mut Vec<PoiEntry>,
	) {
		let mut cx = GenerationContext::new(storage);
		collect_matching_nearby::<Catalog>(&mut cx, center, query, interests, excluded, out);
	}
}

/// Bevy handle to generated-world tile / nearby lookup. World replaces the default [`Catalog`].
#[derive(Resource, Clone)]
pub struct DistantTilePoiLookup(pub Arc<dyn DistantTilePois>);

impl DistantTilePoiLookup {
	pub fn new(lookup: impl DistantTilePois) -> Self {
		Self(Arc::new(lookup))
	}
}

impl Default for DistantTilePoiLookup {
	fn default() -> Self {
		Self::new(Catalog)
	}
}

/// Storage plus the installed generated-world lookup.
#[derive(Clone, Copy)]
pub struct GeneratedNearby<'a> {
	pub storage: &'a HcsgStorage,
	pub lookup: &'a DistantTilePoiLookup,
}

impl<'a> GeneratedNearby<'a> {
	pub fn new(storage: &'a HcsgStorage, lookup: &'a DistantTilePoiLookup) -> Self {
		Self { storage, lookup }
	}

	pub fn collect_matching_nearby(
		self,
		center: Vec3,
		query: NearbyQuery,
		interests: &PoiInterests,
		excluded: &[PoiId],
		out: &mut Vec<PoiEntry>,
	) {
		self.lookup.0.collect_matching_nearby(
			self.storage,
			center,
			query,
			interests,
			excluded,
			out,
		);
	}

	pub fn choose_matching_nearby(
		self,
		center: Vec3,
		query: NearbyQuery,
		interests: &PoiInterests,
		excluded: &[PoiId],
		seed: u64,
	) -> Option<PoiEntry> {
		let mut candidates = Vec::new();
		self.collect_matching_nearby(center, query, interests, excluded, &mut candidates);
		choose_nearby_entry(center, query, interests, &candidates, seed)
	}
}

/// Bounded 3D region for a nearby scheme query, padded by the max arrival disk.
pub fn nearby_region(center: Vec3, radius: f32) -> Option<Aabb3d> {
	if !center.is_finite() || !radius.is_finite() {
		return None;
	}
	let radius = radius.clamp(0.0, MAX_NEARBY_QUERY_RADIUS) + MAX_POI_ARRIVAL_RADIUS;
	let extent = Vec3::splat(radius);
	Some(Aabb3d::from_min_max(center - extent, center + extent))
}

/// XZ column for tile `tile`. `center_y` is accepted for call-site compatibility
/// and must be finite, but it does not clip the column.
pub fn xz_tile_region(tile: IVec2, tile_size: f32, center_y: f32) -> Option<Aabb3d> {
	if !tile_size.is_finite() || !center_y.is_finite() {
		return None;
	}
	let size = normalized_tile_size(tile_size);
	let min_xz = tile.as_vec2() * size;
	let max_xz = min_xz + Vec2::splat(size);
	Some(Aabb3d::from_min_max(
		Vec3::new(min_xz.x, -XZ_COLUMN_Y, min_xz.y),
		Vec3::new(max_xz.x, XZ_COLUMN_Y, max_xz.y),
	))
}

pub fn xz_tile(position: Vec3, tile_size: f32) -> IVec2 {
	(position.xz() / normalized_tile_size(tile_size)).floor().as_ivec2()
}

fn normalized_tile_size(tile_size: f32) -> f32 {
	if tile_size.is_finite() {
		tile_size.clamp(1.0, MAX_TILE_QUERY)
	} else {
		256.0
	}
}

fn xz_point_in_region(position: Vec3, region: Aabb3d) -> bool {
	position.x >= region.min.x
		&& position.x <= region.max.x
		&& position.z >= region.min.z
		&& position.z <= region.max.z
}

fn catalog_id(id: PoiId) -> Id {
	let mut bytes = [0u8; 32];
	bytes[..8].copy_from_slice(&id.0.to_le_bytes());
	Id::Bytes(lod::gen::Bytes(bytes))
}

fn pin_bounds(position: Vec3, arrival_radius: f32) -> Aabb3d {
	let extent = Vec3::splat(arrival_radius.max(0.05));
	Aabb3d::from_min_max(position - extent, position + extent)
}

fn grid_tile_ids(region: Aabb3d, tile_size: f32) -> Vec<OriginalId> {
	let size = normalized_tile_size(tile_size);
	let min =
		IVec2::new((region.min.x / size).floor() as i32, (region.min.z / size).floor() as i32);
	// `xz_tile_region` uses an exclusive max on XZ (the next tile's origin).
	let max = IVec2::new(
		((region.max.x / size).ceil() as i32) - 1,
		((region.max.z / size).ceil() as i32) - 1,
	);
	if max.x < min.x || max.y < min.y {
		return Vec::new();
	}
	let mut ids = Vec::new();
	for z in min.y..=max.y {
		for x in min.x..=max.x {
			ids.push(OriginalId(grid_tile_id(IVec2::new(x, z), size)));
		}
	}
	ids
}

fn grid_tile_id(tile: IVec2, tile_size: f32) -> Id {
	Id::from_cell(xz_tile_cell(tile, tile_size).unwrap_or_else(hcsg::universal_bounds))
}

/// Compact origin-cell footprint for [`GridTiles`] ids. Query regions use the
/// tall [`xz_tile_region`] column; cell ids stay a 1 m slab so `from_cell`
/// does not encode the probe Y.
fn xz_tile_cell(tile: IVec2, tile_size: f32) -> Option<Aabb3d> {
	if !tile_size.is_finite() {
		return None;
	}
	let size = normalized_tile_size(tile_size);
	let min_xz = tile.as_vec2() * size;
	let max_xz = min_xz + Vec2::splat(size);
	Some(Aabb3d::from_min_max(
		Vec3::new(min_xz.x, -0.5, min_xz.y),
		Vec3::new(max_xz.x, 0.5, max_xz.y),
	))
}

fn nearby_candidate(
	center: Vec3,
	radius: f32,
	min_radius: f32,
	interests: &PoiInterests,
	poi: &PoiEntry,
) -> bool {
	interests.contains(poi.kind)
		&& center.distance(poi.position) <= radius + poi.arrival_radius
		&& xz_distance(center, poi.position) >= min_radius
}

fn drop_excluded_entries(candidates: &mut Vec<PoiEntry>, excluded: &[PoiId]) {
	for id in excluded {
		if candidates.len() <= 1 {
			return;
		}
		candidates.retain(|candidate| candidate.id != *id);
	}
}

pub(crate) fn choose_nearby_entry(
	center: Vec3,
	query: NearbyQuery,
	interests: &PoiInterests,
	candidates: &[PoiEntry],
	seed: u64,
) -> Option<PoiEntry> {
	let radius = query.radius.clamp(0.0, MAX_NEARBY_QUERY_RADIUS);
	match query.choice {
		NearbyChoice::Nearest => candidates
			.iter()
			.min_by(|a, b| {
				xz_distance(center, a.position)
					.total_cmp(&xz_distance(center, b.position))
					.then_with(|| a.id.cmp(&b.id))
			})
			.copied(),
		NearbyChoice::Weighted => {
			choose_weighted_entry(center, radius, interests, candidates, seed)
		}
	}
}

fn choose_weighted_entry(
	center: Vec3,
	radius: f32,
	interests: &PoiInterests,
	candidates: &[PoiEntry],
	seed: u64,
) -> Option<PoiEntry> {
	let weight = |entry: &PoiEntry| -> f32 {
		let interest = interests.weight(entry.kind).unwrap_or(0.0);
		let proximity = 1.0 / (1.0 + center.distance(entry.position) / radius.max(1.0));
		interest * entry.salience.max(0.1) * proximity
	};
	let total: f32 = candidates.iter().map(weight).sum();
	if total <= 0.0 {
		return None;
	}
	let mut draw = unit_f32(seed) * total;
	let mut fallback = None;
	for entry in candidates {
		fallback = Some(*entry);
		draw -= weight(entry);
		if draw <= 0.0 {
			return Some(*entry);
		}
	}
	fallback
}

fn xz_distance(a: Vec3, b: Vec3) -> f32 {
	(a.xz() - b.xz()).length()
}

fn grid_poi_id(tile: IVec2) -> u64 {
	((tile.x as u64) << 32) ^ (tile.y as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15)
}
