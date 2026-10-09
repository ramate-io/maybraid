//! Consumer-owned reads of groves, urbanization, geography, and POIs.

use std::collections::HashSet;
use std::marker::PhantomData;
use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::math::{Rect, Vec2, Vec3};
use chico::{ChicoForest, ChicoGrove, ForestGround, GrownGrove};
use durham::terrain::stamps::StampLeaf;
use durham::terrain::watersheds::{PocketWater, PocketWatersHighPass, PocketWatersLowPass};
use durham::terrain::{
	CanyonHighPassStampCell, CanyonLowPassStampCell, MassifHighPassStampCell,
	MassifLowPassStampCell, PlateauHighPassStampCell, PlateauLowPassStampCell,
	PocketWaterHighPassStampCell, PocketWaterLowPassStampCell, RollingHighPassStampCell,
	RollingLowPassStampCell, ValleyHighPassStampCell, ValleyLowPassStampCell,
};
use durham::{GeographicBand, GeographicFamily, GeographicFeatureId, GeographicFeatureKind};
use lod::hcsg::shared::{Busy, HcsgStorage, HcsgValue};
use procedural_common::Bounds2;
use richmond::{DiscoverablePlace, DiscoverablePlaceIndex};
use urbanization_cells::SelectedUrbanization;

use crate::english::{
	named_forest_english, named_geographic_english, named_grove_english, named_place_english,
	named_urban_english,
};
use crate::index::{large_tiles_overlapping, name_key_salt, LanguageIndex, NameKey};
use crate::name::terms_fingerprint;
use crate::tiles::LargeTile;

/// Quantize the naming origin so small camera moves do not rescan.
pub const NAME_WINDOW_QUANT_M: f32 = 32.0;

/// Half-height of a naming query: every height a named source stands at.
const NAMING_COLUMN_Y: f32 = 10_000.0;

/// Nearby naming query. Tiles keep a separate 40 km generate window.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NamingRegion {
	pub origin: Vec2,
	pub collect_radius: f32,
	pub retain_radius: f32,
}

impl NamingRegion {
	pub fn around(origin: Vec2, collect_radius: f32, retain_radius: f32) -> Self {
		Self { origin, collect_radius, retain_radius }
	}

	pub fn query_aabb(self) -> Aabb3d {
		let radius = self.retain_radius.max(self.collect_radius);
		Aabb3d::from_min_max(
			Vec3::new(self.origin.x - radius, -NAMING_COLUMN_Y, self.origin.y - radius),
			Vec3::new(self.origin.x + radius, NAMING_COLUMN_Y, self.origin.y + radius),
		)
	}

	pub fn quantized_origin(self) -> (i32, i32) {
		(quantize_axis(self.origin.x), quantize_axis(self.origin.y))
	}

	pub fn collects_bounds(self, bounds: Aabb3d) -> bool {
		xz_distance_to_aabb(self.origin, bounds) <= self.collect_radius
	}

	pub fn retains_bounds(self, bounds: Aabb3d) -> bool {
		xz_distance_to_aabb(self.origin, bounds) <= self.retain_radius
	}

	pub fn collects_xz(self, xz: Vec2) -> bool {
		self.origin.distance(xz) <= self.collect_radius
	}

	pub fn retains_xz(self, xz: Vec2) -> bool {
		self.origin.distance(xz) <= self.retain_radius
	}
}

fn quantize_axis(value: f32) -> i32 {
	(value / NAME_WINDOW_QUANT_M).round() as i32
}

fn xz_distance_to_aabb(origin: Vec2, bounds: Aabb3d) -> f32 {
	let closest = Vec2::new(
		origin.x.clamp(bounds.min.x, bounds.max.x),
		origin.y.clamp(bounds.min.z, bounds.max.z),
	);
	origin.distance(closest)
}

/// Individual source revisions. Do not XOR these together for invalidation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SourceRevisions {
	pub forest: u64,
	pub urban: u64,
	pub terrain: u64,
	pub places: u64,
	pub tiles: u64,
}

/// A generated cell the language layer may name.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedFeature {
	pub key: NameKey,
	pub bounds: Aabb3d,
	pub english: Vec<String>,
	pub revision: u64,
	pub fingerprint: u64,
	pub provisional: bool,
}

impl NamedFeature {
	pub fn new(key: NameKey, bounds: Aabb3d, english: Vec<String>, revision: u64) -> Self {
		let fingerprint = terms_fingerprint(&english);
		Self { key, bounds, english, revision, fingerprint, provisional: false }
	}
}

/// A POI the language layer may name.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedPlace {
	pub key: NameKey,
	pub xz: Vec2,
	pub english: Vec<String>,
	pub persistent: bool,
	pub revision: u64,
	pub fingerprint: u64,
	pub provisional: bool,
	pub host: Option<lod::gen::Id>,
	pub inherit_host_language: bool,
}

/// Pose update that must not force retranslation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoseUpdate {
	pub key: NameKey,
	pub center: Vec2,
	pub extent: Rect,
}

/// New or restamped features plus the live keep-set for one source class.
#[derive(Clone, Debug, Default)]
pub struct FeatureSnapshot {
	pub active: HashSet<NameKey>,
	pub work: Vec<NamedFeature>,
	pub moved: Vec<PoseUpdate>,
}

/// New or restamped places plus the live keep-set.
#[derive(Clone, Debug, Default)]
pub struct PlaceSnapshot {
	pub active: HashSet<NameKey>,
	pub work: Vec<NamedPlace>,
	pub moved: Vec<PoseUpdate>,
}

/// What Geneva reads from the world under it, without waiting on a lock:
/// forests and groves grown on ground `W`, urbanization cells and Durham's
/// authored geography from the shared storage, and Richmond's places.
///
/// Only what the world has already published is named; Geneva generates none
/// of it. [`Busy`] means try again next frame.
pub struct NameSources<'a, W> {
	storage: &'a HcsgStorage,
	places: Option<&'a DiscoverablePlaceIndex>,
	_ground: PhantomData<fn() -> W>,
}

impl<'a, W: ForestGround> NameSources<'a, W> {
	pub fn new(storage: &'a HcsgStorage, places: Option<&'a DiscoverablePlaceIndex>) -> Self {
		Self { storage, places, _ground: PhantomData }
	}

	pub fn revisions(&self) -> Result<SourceRevisions, Busy> {
		let storage = self.storage;
		Ok(SourceRevisions {
			forest: storage
				.try_membership_revision::<ChicoForest>()?
				.max(storage.try_membership_revision::<GrownGrove<W>>()?),
			urban: storage.try_membership_revision::<SelectedUrbanization>()?,
			terrain: geography_revision(storage)?,
			places: self.places.map_or(0, DiscoverablePlaceIndex::membership_revision),
			tiles: storage.try_membership_revision::<LargeTile>()?,
		})
	}

	/// Published tiles over `window` that `language` has not admitted yet.
	pub fn tiles(
		&self,
		window: &[(i32, i32)],
		language: &LanguageIndex,
	) -> Result<Vec<Arc<LargeTile>>, Busy> {
		let mut tiles = Vec::new();
		for &(ix, iz) in window {
			if language.large_tile(ix, iz).is_some() {
				continue;
			}
			if let Some(entry) = self.storage.try_entry::<LargeTile>(LargeTile::id(ix, iz))? {
				tiles.push(entry.value);
			}
		}
		Ok(tiles)
	}

	/// Forests, and the groves that grew on `W`.
	pub fn groves(
		&self,
		region: NamingRegion,
		language: &LanguageIndex,
	) -> Result<FeatureSnapshot, Busy> {
		let query = region.query_aabb();
		let mut snapshot = FeatureSnapshot::default();
		for id in self.storage.try_overlapping::<ChicoForest>(query)? {
			let Some(entry) = self.storage.try_entry::<ChicoForest>(id)? else {
				continue;
			};
			let forest = &entry.value;
			consider_feature(
				&mut snapshot,
				language,
				region,
				NameKey::Forest(id),
				entry.bounds,
				entry.version.0,
				|| {
					let kinds = [
						forest.layers.tufts,
						forest.layers.understory,
						forest.layers.lower_canopy,
						forest.layers.upper_canopy,
					]
					.into_iter()
					.flatten();
					Some(named_forest_english(
						forest.layers.layering,
						kinds,
						name_key_salt(NameKey::Forest(id)),
					))
				},
			);
		}
		for id in self.storage.try_overlapping::<GrownGrove<W>>(query)? {
			let Some(entry) = self.storage.try_entry::<ChicoGrove>(id)? else {
				continue;
			};
			let grove = &entry.value;
			consider_feature(
				&mut snapshot,
				language,
				region,
				NameKey::Grove(id),
				entry.bounds,
				entry.version.0,
				|| {
					Some(named_grove_english(
						grove.recipes.iter().map(|recipe| recipe.kind),
						name_key_salt(NameKey::Grove(id)),
					))
				},
			);
		}
		Ok(snapshot)
	}

	/// Durham's occupied stamps and authored pocket waters, one name per source.
	pub fn geography(
		&self,
		region: NamingRegion,
		language: &LanguageIndex,
	) -> Result<FeatureSnapshot, Busy> {
		use GeographicBand::{HighPass, LowPass};
		use GeographicFamily as Family;
		use GeographicFeatureKind as Kind;
		let mut out = FeatureSnapshot::default();
		let mut read = Geography { storage: self.storage, language, region, out: &mut out };
		read.stamps::<MassifHighPassStampCell>(Family::Massif, HighPass, Kind::Massif)?;
		read.stamps::<MassifLowPassStampCell>(Family::Massif, LowPass, Kind::Massif)?;
		read.stamps::<PlateauHighPassStampCell>(Family::Plateau, HighPass, Kind::Plateau)?;
		read.stamps::<PlateauLowPassStampCell>(Family::Plateau, LowPass, Kind::Plateau)?;
		read.stamps::<CanyonHighPassStampCell>(Family::Canyon, HighPass, Kind::Canyon)?;
		read.stamps::<CanyonLowPassStampCell>(Family::Canyon, LowPass, Kind::Canyon)?;
		read.stamps::<RollingHighPassStampCell>(Family::Rolling, HighPass, Kind::Rolling)?;
		read.stamps::<RollingLowPassStampCell>(Family::Rolling, LowPass, Kind::Rolling)?;
		read.stamps::<ValleyHighPassStampCell>(Family::Valley, HighPass, Kind::Valley)?;
		read.stamps::<ValleyLowPassStampCell>(Family::Valley, LowPass, Kind::Valley)?;
		let pocket = Family::PocketWaterStamp;
		read.stamps::<PocketWaterHighPassStampCell>(pocket, HighPass, Kind::PocketWater)?;
		read.stamps::<PocketWaterLowPassStampCell>(pocket, LowPass, Kind::PocketWater)?;
		read.waters::<PocketWatersHighPass>(HighPass, |waters| &waters.authored)?;
		read.waters::<PocketWatersLowPass>(LowPass, |waters| &waters.authored)?;
		Ok(out)
	}

	/// Urbanization cells and their development leaves.
	pub fn urban(
		&self,
		region: NamingRegion,
		language: &LanguageIndex,
	) -> Result<FeatureSnapshot, Busy> {
		let mut snapshot = FeatureSnapshot::default();
		for id in self.storage.try_overlapping::<SelectedUrbanization>(region.query_aabb())? {
			let Some(entry) = self.storage.try_entry::<SelectedUrbanization>(id)? else {
				continue;
			};
			let (cell, revision) = (&entry.value, entry.version.0);
			consider_feature(
				&mut snapshot,
				language,
				region,
				NameKey::Urban(id),
				entry.bounds,
				revision,
				|| Some(named_urban_english(cell.kind, None, name_key_salt(NameKey::Urban(id)))),
			);
			for leaf in &cell.leaves {
				let key = NameKey::UrbanLeaf(leaf.id());
				consider_feature(&mut snapshot, language, region, key, leaf.bounds, revision, || {
					Some(named_urban_english(cell.kind, Some(leaf.kind), name_key_salt(key)))
				});
			}
		}
		Ok(snapshot)
	}

	/// Richmond's places, from its [`DiscoverablePlaceIndex`] when present.
	pub fn places(&self, region: NamingRegion, language: &LanguageIndex) -> PlaceSnapshot {
		let mut snapshot = PlaceSnapshot::default();
		let Some(places) = self.places else {
			return snapshot;
		};
		for record in places.overlapping(region.query_aabb()) {
			let xz = record.xz;
			if region.retains_xz(xz) {
				let (key, _, _) = place_key(&record.place, xz);
				snapshot.active.insert(key);
			}
			if !region.collects_xz(xz) {
				continue;
			}
			snapshot_place(&mut snapshot, language, &record.place, xz);
		}
		snapshot
	}
}

/// One geography snapshot being filled.
struct Geography<'a> {
	storage: &'a HcsgStorage,
	language: &'a LanguageIndex,
	region: NamingRegion,
	out: &'a mut FeatureSnapshot,
}

impl Geography<'_> {
	/// Stamp leaves with modulations; an empty leaf is not a source.
	fn stamps<T: StampLeaf + HcsgValue>(
		&mut self,
		family: GeographicFamily,
		band: GeographicBand,
		kind: GeographicFeatureKind,
	) -> Result<(), Busy> {
		for id in self.storage.try_overlapping::<T>(self.region.query_aabb())? {
			let Some(entry) = self.storage.try_entry::<T>(id)? else {
				continue;
			};
			if entry.value.modulations().is_empty() {
				continue;
			}
			let cell = entry.value.cell();
			let bounds = Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z);
			self.push(GeographicFeatureId { family, band, source: id }, kind, bounds, entry.version.0);
		}
		Ok(())
	}

	/// Authored lakes, bogs and streams, named once across the cells they feed.
	fn waters<T: HcsgValue>(
		&mut self,
		band: GeographicBand,
		authored: impl Fn(&T) -> &PocketWater,
	) -> Result<(), Busy> {
		for id in self.storage.try_overlapping::<T>(self.region.query_aabb())? {
			let Some(entry) = self.storage.try_entry::<T>(id)? else {
				continue;
			};
			let (kind, bounds) = match authored(&entry.value) {
				PocketWater::Empty => continue,
				PocketWater::Lake(lake) => (GeographicFeatureKind::Lake, lake.bounds),
				PocketWater::Bog(bog) => (GeographicFeatureKind::Bog, bog.bounds),
				PocketWater::Stream(stream) => (GeographicFeatureKind::Stream, stream.bounds),
				PocketWater::StreamsGraph(graph) => {
					(GeographicFeatureKind::StreamsGraph, graph.bounds)
				}
			};
			let family = GeographicFamily::Watershed;
			self.push(GeographicFeatureId { family, band, source: id }, kind, bounds, entry.version.0);
		}
		Ok(())
	}

	fn push(
		&mut self,
		id: GeographicFeatureId,
		kind: GeographicFeatureKind,
		bounds: Bounds2,
		revision: u64,
	) {
		let key = NameKey::Geographic(id);
		let bounds = Aabb3d::from_min_max(
			Vec3::new(bounds.min.x, -1.0, bounds.min.y),
			Vec3::new(bounds.max.x, 1.0, bounds.max.y),
		);
		consider_feature(self.out, self.language, self.region, key, bounds, revision, || {
			Some(named_geographic_english(kind, name_key_salt(key)))
		});
	}
}

/// Latest membership change among the stores [`NameSources::geography`] reads.
fn geography_revision(storage: &HcsgStorage) -> Result<u64, Busy> {
	Ok([
		storage.try_membership_revision::<MassifHighPassStampCell>()?,
		storage.try_membership_revision::<MassifLowPassStampCell>()?,
		storage.try_membership_revision::<PlateauHighPassStampCell>()?,
		storage.try_membership_revision::<PlateauLowPassStampCell>()?,
		storage.try_membership_revision::<CanyonHighPassStampCell>()?,
		storage.try_membership_revision::<CanyonLowPassStampCell>()?,
		storage.try_membership_revision::<RollingHighPassStampCell>()?,
		storage.try_membership_revision::<RollingLowPassStampCell>()?,
		storage.try_membership_revision::<ValleyHighPassStampCell>()?,
		storage.try_membership_revision::<ValleyLowPassStampCell>()?,
		storage.try_membership_revision::<PocketWaterHighPassStampCell>()?,
		storage.try_membership_revision::<PocketWaterLowPassStampCell>()?,
		storage.try_membership_revision::<PocketWatersHighPass>()?,
		storage.try_membership_revision::<PocketWatersLowPass>()?,
	]
	.into_iter()
	.max()
	.unwrap_or(0))
}

/// The tiles over every box of a window, each once.
pub(crate) fn window_tiles(boxes: &[Aabb3d]) -> Vec<(i32, i32)> {
	let mut tiles: Vec<_> = boxes.iter().flat_map(|region| large_tiles_overlapping(*region)).collect();
	tiles.sort_unstable();
	tiles.dedup();
	tiles
}

pub(crate) fn features_to_snapshot(
	features: Vec<NamedFeature>,
	index: &LanguageIndex,
	region: NamingRegion,
) -> FeatureSnapshot {
	let mut snapshot = FeatureSnapshot::default();
	for feature in features {
		if region.retains_bounds(feature.bounds) {
			snapshot.active.insert(feature.key);
		}
		if !region.collects_bounds(feature.bounds) {
			continue;
		}
		if index.is_current(feature.key, feature.revision, feature.fingerprint) {
			push_feature_move(&mut snapshot.moved, index, &feature);
			continue;
		}
		snapshot.work.push(feature);
	}
	snapshot
}

fn push_feature_move(moved: &mut Vec<PoseUpdate>, index: &LanguageIndex, feature: &NamedFeature) {
	let center = feature_center(feature.bounds);
	let extent = xz_extent(feature.bounds);
	if index.pose_matches(feature.key, center, extent) {
		return;
	}
	moved.push(PoseUpdate { key: feature.key, center, extent });
}

fn push_place_move(moved: &mut Vec<PoseUpdate>, index: &LanguageIndex, place: &NamedPlace) {
	let extent = place_extent(place.xz);
	if index.pose_matches(place.key, place.xz, extent) {
		return;
	}
	moved.push(PoseUpdate { key: place.key, center: place.xz, extent });
}

pub(crate) fn feature_center(bounds: Aabb3d) -> Vec2 {
	Vec2::new((bounds.min.x + bounds.max.x) * 0.5, (bounds.min.z + bounds.max.z) * 0.5)
}

pub(crate) fn xz_extent(bounds: Aabb3d) -> Rect {
	Rect::from_corners(Vec2::new(bounds.min.x, bounds.min.z), Vec2::new(bounds.max.x, bounds.max.z))
}

pub(crate) fn place_extent(xz: Vec2) -> Rect {
	Rect::from_center_size(xz, Vec2::splat(12.0))
}

fn consider_feature(
	snapshot: &mut FeatureSnapshot,
	language: &LanguageIndex,
	region: NamingRegion,
	key: NameKey,
	bounds: Aabb3d,
	revision: u64,
	english: impl FnOnce() -> Option<Vec<String>>,
) {
	if region.retains_bounds(bounds) {
		snapshot.active.insert(key);
	}
	if !region.collects_bounds(bounds) {
		return;
	}
	if language.is_current_revision(key, revision) {
		let feature = NamedFeature {
			key,
			bounds,
			english: Vec::new(),
			revision,
			fingerprint: 0,
			provisional: false,
		};
		push_feature_move(&mut snapshot.moved, language, &feature);
		return;
	}
	let Some(english) = english() else {
		return;
	};
	snapshot.work.push(NamedFeature::new(key, bounds, english, revision));
}

fn snapshot_place(
	snapshot: &mut PlaceSnapshot,
	language: &LanguageIndex,
	place: &DiscoverablePlace,
	xz: Vec2,
) {
	let named = named_place(place, xz);
	if named.inherit_host_language || !language.is_current(named.key, 1, named.fingerprint) {
		snapshot.work.push(named);
		return;
	}
	push_place_move(&mut snapshot.moved, language, &named);
}

/// The place's name inputs. English follows identity, not pose, so a moved
/// place keeps its name.
fn named_place(place: &DiscoverablePlace, xz: Vec2) -> NamedPlace {
	let (key, provisional, inherit_host_language) = place_key(place, xz);
	let english = named_place_english(place.label, place_identity_bits(place));
	let fingerprint = terms_fingerprint(&english);
	NamedPlace {
		key,
		xz,
		english,
		persistent: place.persistent,
		revision: 1,
		fingerprint,
		provisional,
		host: place.host,
		inherit_host_language,
	}
}

fn place_key(place: &DiscoverablePlace, xz: Vec2) -> (NameKey, bool, bool) {
	if let Some(host) = place.host {
		(NameKey::Place { host, local: place.local }, false, !place.persistent)
	} else {
		(
			NameKey::ProvisionalPlace {
				qx: xz.x.round() as i32,
				qz: xz.y.round() as i32,
				label: place.label.salt() as u32,
			},
			true,
			false,
		)
	}
}

fn place_identity_bits(place: &DiscoverablePlace) -> u64 {
	match place.host {
		Some(host) => name_key_salt(NameKey::Place { host, local: place.local }),
		None => u64::from(place.label.salt() as u32),
	}
}

/// World XZ only. Elevated POIs stay eligible when they sit over the region.
#[cfg(test)]
pub(crate) fn places_from_world_xz(
	places: impl IntoIterator<Item = (DiscoverablePlace, Vec3)>,
	region: Aabb3d,
) -> Vec<NamedPlace> {
	places
		.into_iter()
		.filter(|(_, world)| {
			world.x >= region.min.x
				&& world.x < region.max.x
				&& world.z >= region.min.z
				&& world.z < region.max.z
		})
		.map(|(place, world)| named_place(&place, Vec2::new(world.x, world.z)))
		.collect()
}
