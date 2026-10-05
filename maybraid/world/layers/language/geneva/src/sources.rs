//! Consumer-owned reads of groves, urbanization, geography, and POIs.

use std::collections::HashSet;

use bevy::ecs::system::{ReadOnlySystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::{Rect, Vec2, Vec3};
use bevy::prelude::Res;
use chico::{Chico, ChicoForest, ChicoGrove, ForestIndex};
use durham::TerrainEntryStore;
use lod::gen::{SpatialIndex, TrackedId};
use procedural_common::Bounds2;
use richmond::{DiscoverablePlace, DiscoverablePlaceIndex, Richmond};
use urbanization_cells::{SelectedUrbanization, UrbanizationIndex};
use urbanization_layer_model::Urbanization;
use vegetation_layer_model::Vegetation;

use crate::english::{
	named_forest_english, named_geographic_english, named_grove_english, named_place_english,
	named_urban_english,
};
use crate::index::{name_key_salt, LanguageIndex, NameKey};
use crate::name::terms_fingerprint;

/// Quantize the naming origin so small camera moves do not rescan.
pub const NAME_WINDOW_QUANT_M: f32 = 32.0;

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
			Vec3::new(self.origin.x - radius, -1.0, self.origin.y - radius),
			Vec3::new(self.origin.x + radius, 1.0, self.origin.y + radius),
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

fn places_to_snapshot(
	places: Vec<NamedPlace>,
	index: &LanguageIndex,
	region: NamingRegion,
) -> PlaceSnapshot {
	let mut snapshot = PlaceSnapshot::default();
	for place in places {
		if region.retains_xz(place.xz) {
			snapshot.active.insert(place.key);
		}
		if !region.collects_xz(place.xz) {
			continue;
		}
		if place.inherit_host_language
			|| !index.is_current(place.key, place.revision, place.fingerprint)
		{
			snapshot.work.push(place);
			continue;
		}
		push_place_move(&mut snapshot.moved, index, &place);
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

fn place_extent(xz: Vec2) -> Rect {
	Rect::from_center_size(xz, Vec2::splat(12.0))
}

/// What Geneva reads from the vegetated world under it.
///
/// Vegetation, urbanization, and POI contracts do not grow a naming method.
pub trait NamedWorld: Send + Sync + 'static {
	type Read: ReadOnlySystemParam + 'static;

	fn source_revisions(read: &SystemParamItem<'_, '_, Self::Read>) -> SourceRevisions;

	fn groves_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
	) -> Vec<NamedFeature>;

	fn geography_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
	) -> Vec<NamedFeature>;

	fn urban_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
	) -> Vec<NamedFeature>;

	fn places_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
	) -> Vec<NamedPlace>;

	fn groves_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
		index: &LanguageIndex,
	) -> FeatureSnapshot {
		features_to_snapshot(Self::groves_overlapping(read, region), index, region)
	}

	fn geography_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
		index: &LanguageIndex,
	) -> FeatureSnapshot {
		features_to_snapshot(Self::geography_overlapping(read, region), index, region)
	}

	fn urban_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
		index: &LanguageIndex,
	) -> FeatureSnapshot {
		features_to_snapshot(Self::urban_overlapping(read, region), index, region)
	}

	fn places_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
		index: &LanguageIndex,
	) -> PlaceSnapshot {
		places_to_snapshot(Self::places_overlapping(read, region), index, region)
	}
}

type WorldRead = (
	Res<'static, ForestIndex>,
	Res<'static, UrbanizationIndex>,
	Option<Res<'static, TerrainEntryStore>>,
	Option<Res<'static, DiscoverablePlaceIndex>>,
);

impl<T: 'static> NamedWorld for Vegetation<Chico<Urbanization<Richmond<T>>>> {
	type Read = WorldRead;

	fn source_revisions(read: &SystemParamItem<'_, '_, Self::Read>) -> SourceRevisions {
		let (forests, urban, terrain, places) = read;
		SourceRevisions {
			forest: forests.membership_revision(),
			urban: SpatialIndex::<SelectedUrbanization>::membership_revision(&**urban),
			terrain: terrain.as_ref().map(|store| store.membership_revision()).unwrap_or(0),
			places: places.as_ref().map(|index| index.membership_revision()).unwrap_or(0),
		}
	}

	fn groves_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
	) -> Vec<NamedFeature> {
		let (forests, _, _, _) = read;
		grove_features(forests, region)
	}

	fn geography_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
	) -> Vec<NamedFeature> {
		let (_, _, Some(store), _) = read else {
			return Vec::new();
		};
		geography_features(store, region)
	}

	fn urban_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
	) -> Vec<NamedFeature> {
		let (_, urban, _, _) = read;
		urban_features(urban, region)
	}

	fn places_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
	) -> Vec<NamedPlace> {
		let (_, _, _, Some(places)) = read else {
			return Vec::new();
		};
		place_features(places, region)
	}

	fn groves_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
		index: &LanguageIndex,
	) -> FeatureSnapshot {
		let (forests, _, _, _) = read;
		grove_snapshot(forests, region, index)
	}

	fn geography_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
		index: &LanguageIndex,
	) -> FeatureSnapshot {
		let (_, _, Some(store), _) = read else {
			return FeatureSnapshot::default();
		};
		geography_snapshot(store, region, index)
	}

	fn urban_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
		index: &LanguageIndex,
	) -> FeatureSnapshot {
		let (_, urban, _, _) = read;
		urban_snapshot(urban, region, index)
	}

	fn places_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: NamingRegion,
		index: &LanguageIndex,
	) -> PlaceSnapshot {
		let (_, _, _, Some(places)) = read else {
			return PlaceSnapshot::default();
		};
		place_snapshot(places, region, index)
	}
}

fn grove_features(index: &ForestIndex, region: NamingRegion) -> Vec<NamedFeature> {
	grove_snapshot(index, region, &LanguageIndex::default()).work
}

fn grove_snapshot(
	forests: &ForestIndex,
	region: NamingRegion,
	language: &LanguageIndex,
) -> FeatureSnapshot {
	let query = region.query_aabb();
	let mut snapshot = FeatureSnapshot::default();
	for TrackedId(id) in SpatialIndex::<ChicoForest>::tracked_ids_for(forests, query) {
		let Some(bounds) = SpatialIndex::<ChicoForest>::get_bounds(forests, id) else {
			continue;
		};
		consider_feature(
			&mut snapshot,
			language,
			region,
			NameKey::Forest(id),
			bounds,
			SpatialIndex::<ChicoForest>::version(forests, id).map(|v| v.0).unwrap_or(0),
			|| {
				let forest = SpatialIndex::<ChicoForest>::get(forests, id)?;
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
	for TrackedId(id) in SpatialIndex::<ChicoGrove>::tracked_ids_for(forests, query) {
		let Some(bounds) = SpatialIndex::<ChicoGrove>::get_bounds(forests, id) else {
			continue;
		};
		consider_feature(
			&mut snapshot,
			language,
			region,
			NameKey::Grove(id),
			bounds,
			SpatialIndex::<ChicoGrove>::version(forests, id).map(|v| v.0).unwrap_or(0),
			|| {
				let grove = SpatialIndex::<ChicoGrove>::get(forests, id)?;
				Some(named_grove_english(
					grove.recipes.iter().map(|recipe| recipe.kind),
					name_key_salt(NameKey::Grove(id)),
				))
			},
		);
	}
	snapshot
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

fn geography_features(store: &TerrainEntryStore, region: NamingRegion) -> Vec<NamedFeature> {
	geography_snapshot(store, region, &LanguageIndex::default()).work
}

fn geography_snapshot(
	store: &TerrainEntryStore,
	region: NamingRegion,
	language: &LanguageIndex,
) -> FeatureSnapshot {
	let aabb = region.query_aabb();
	let query = Bounds2::from_xz(aabb.min.x, aabb.min.z, aabb.max.x, aabb.max.z);
	let mut snapshot = FeatureSnapshot::default();
	for feature in store.geographic_features_overlapping(query) {
		let key = NameKey::Geographic(feature.id);
		let bounds = Aabb3d::from_min_max(
			Vec3::new(feature.bounds.min.x, -1.0, feature.bounds.min.y),
			Vec3::new(feature.bounds.max.x, 1.0, feature.bounds.max.y),
		);
		consider_feature(&mut snapshot, language, region, key, bounds, feature.revision.0, || {
			Some(named_geographic_english(feature.kind, name_key_salt(key)))
		});
	}
	snapshot
}

fn urban_features(index: &UrbanizationIndex, region: NamingRegion) -> Vec<NamedFeature> {
	urban_snapshot(index, region, &LanguageIndex::default()).work
}

fn urban_snapshot(
	index: &UrbanizationIndex,
	region: NamingRegion,
	language: &LanguageIndex,
) -> FeatureSnapshot {
	let query = region.query_aabb();
	let mut snapshot = FeatureSnapshot::default();
	for TrackedId(id) in SpatialIndex::<SelectedUrbanization>::tracked_ids_for(index, query) {
		let Some(cell) = SpatialIndex::<SelectedUrbanization>::get(index, id) else {
			continue;
		};
		let Some(bounds) = SpatialIndex::<SelectedUrbanization>::get_bounds(index, id) else {
			continue;
		};
		let revision = SpatialIndex::<SelectedUrbanization>::version(index, id)
			.map(|v| v.0)
			.unwrap_or(0);
		consider_feature(
			&mut snapshot,
			language,
			region,
			NameKey::Urban(id),
			bounds,
			revision,
			|| Some(named_urban_english(cell.kind, None, name_key_salt(NameKey::Urban(id)))),
		);
		for leaf in &cell.leaves {
			consider_feature(
				&mut snapshot,
				language,
				region,
				NameKey::UrbanLeaf(leaf.id()),
				leaf.bounds,
				revision,
				|| {
					Some(named_urban_english(
						cell.kind,
						Some(leaf.kind),
						name_key_salt(NameKey::UrbanLeaf(leaf.id())),
					))
				},
			);
		}
	}
	snapshot
}

fn place_features(places: &DiscoverablePlaceIndex, region: NamingRegion) -> Vec<NamedPlace> {
	place_snapshot(places, region, &LanguageIndex::default()).work
}

fn place_snapshot(
	places: &DiscoverablePlaceIndex,
	region: NamingRegion,
	language: &LanguageIndex,
) -> PlaceSnapshot {
	let mut snapshot = PlaceSnapshot::default();
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

fn snapshot_place(
	snapshot: &mut PlaceSnapshot,
	language: &LanguageIndex,
	place: &DiscoverablePlace,
	xz: Vec2,
) {
	let (key, provisional, inherit_host_language) = place_key(place, xz);
	let english = named_place_english(place.label, place_identity_bits(place));
	let fingerprint = terms_fingerprint(&english);
	let named = NamedPlace {
		key,
		xz,
		english,
		persistent: place.persistent,
		revision: 1,
		fingerprint,
		provisional,
		host: place.host,
		inherit_host_language,
	};
	if inherit_host_language || !language.is_current(key, 1, fingerprint) {
		snapshot.work.push(named);
		return;
	}
	push_place_move(&mut snapshot.moved, language, &named);
}

/// World XZ only. Elevated POIs stay eligible when they sit over the region.
#[cfg(test)]
fn xz_contains(region: Aabb3d, x: f32, z: f32) -> bool {
	x >= region.min.x && x < region.max.x && z >= region.min.z && z < region.max.z
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
		Some(host) => crate::index::name_key_salt(NameKey::Place { host, local: place.local }),
		None => u64::from(place.label.salt() as u32),
	}
}

#[cfg(test)]
pub(crate) fn places_from_world_xz(
	places: impl IntoIterator<Item = (DiscoverablePlace, bevy::math::Vec3)>,
	region: Aabb3d,
) -> Vec<NamedPlace> {
	let mut out = Vec::new();
	for (place, world) in places {
		if !xz_contains(region, world.x, world.z) {
			continue;
		}
		let xz = Vec2::new(world.x, world.z);
		let (key, provisional, inherit_host_language) = place_key(&place, xz);
		let english = named_place_english(place.label, place_identity_bits(&place));
		let fingerprint = terms_fingerprint(&english);
		out.push(NamedPlace {
			key,
			xz,
			english,
			persistent: place.persistent,
			revision: 1,
			fingerprint,
			provisional,
			host: place.host,
			inherit_host_language,
		});
	}
	out
}
