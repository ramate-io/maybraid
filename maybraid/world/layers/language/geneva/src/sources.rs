//! Consumer-owned reads of groves, urbanization, geography, and POIs.

use std::collections::HashSet;

use bevy::ecs::system::{ReadOnlySystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::{GlobalTransform, Query, Res};
use chico::{Chico, ChicoForest, ChicoGrove, ForestIndex};
use durham::TerrainEntryStore;
use lod::gen::{SpatialIndex, TrackedId};
use procedural_common::Bounds2;
use richmond::{DiscoverablePlace, Richmond};
use urbanization_cells::{SelectedUrbanization, UrbanizationIndex};
use urbanization_layer_model::Urbanization;
use vegetation_layer_model::Vegetation;

use maybraid_language_core::lexicalizer::mix;

use crate::english::{
	named_forest_english, named_geographic_english, named_grove_english, named_place_english,
	named_urban_english,
};
use crate::index::{name_key_salt, LanguageIndex, NameKey};
use crate::name::terms_fingerprint;

/// Quantize POI XZ so parent-transform jitter does not dirty keep deps.
const PLACE_QUANT_M: f32 = 8.0;

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

/// New or restamped features plus the live keep-set for one source class.
#[derive(Clone, Debug, Default)]
pub struct FeatureSnapshot {
	pub active: HashSet<NameKey>,
	pub work: Vec<NamedFeature>,
}

/// New or restamped places plus the live keep-set.
#[derive(Clone, Debug, Default)]
pub struct PlaceSnapshot {
	pub active: HashSet<NameKey>,
	pub work: Vec<NamedPlace>,
}

fn features_to_snapshot(features: Vec<NamedFeature>, index: &LanguageIndex) -> FeatureSnapshot {
	let active = features.iter().map(|feature| feature.key).collect();
	let work = features
		.into_iter()
		.filter(|feature| !index.is_current(feature.key, feature.revision, feature.fingerprint))
		.collect();
	FeatureSnapshot { active, work }
}

fn places_to_snapshot(places: Vec<NamedPlace>, index: &LanguageIndex) -> PlaceSnapshot {
	let active = places.iter().map(|place| place.key).collect();
	let work = places
		.into_iter()
		.filter(|place| {
			place.inherit_host_language
				|| !index.is_current(place.key, place.revision, place.fingerprint)
		})
		.collect();
	PlaceSnapshot { active, work }
}

/// What Geneva reads from the vegetated world under it.
///
/// Vegetation, urbanization, and POI contracts do not grow a naming method.
pub trait NamedWorld: Send + Sync + 'static {
	type Read: ReadOnlySystemParam + 'static;

	fn source_revisions(read: &SystemParamItem<'_, '_, Self::Read>) -> SourceRevisions;

	fn groves_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<NamedFeature>;

	fn geography_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<NamedFeature>;

	fn urban_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<NamedFeature>;

	fn places_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<NamedPlace>;

	fn groves_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
		index: &LanguageIndex,
	) -> FeatureSnapshot {
		features_to_snapshot(Self::groves_overlapping(read, region), index)
	}

	fn geography_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
		index: &LanguageIndex,
	) -> FeatureSnapshot {
		features_to_snapshot(Self::geography_overlapping(read, region), index)
	}

	fn urban_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
		index: &LanguageIndex,
	) -> FeatureSnapshot {
		features_to_snapshot(Self::urban_overlapping(read, region), index)
	}

	fn places_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
		index: &LanguageIndex,
	) -> PlaceSnapshot {
		places_to_snapshot(Self::places_overlapping(read, region), index)
	}
}

type WorldRead = (
	Res<'static, ForestIndex>,
	Res<'static, UrbanizationIndex>,
	Option<Res<'static, TerrainEntryStore>>,
	Query<'static, 'static, (&'static DiscoverablePlace, &'static GlobalTransform)>,
);

impl<T: 'static> NamedWorld for Vegetation<Chico<Urbanization<Richmond<T>>>> {
	type Read = WorldRead;

	fn source_revisions(read: &SystemParamItem<'_, '_, Self::Read>) -> SourceRevisions {
		let (forests, urban, terrain, places) = read;
		SourceRevisions {
			forest: forests.membership_revision(),
			urban: SpatialIndex::<SelectedUrbanization>::membership_revision(&**urban),
			terrain: terrain.as_ref().map(|store| store.membership_revision()).unwrap_or(0),
			places: places_signature(places),
		}
	}

	fn groves_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<NamedFeature> {
		let (forests, _, _, _) = read;
		grove_features(forests, region)
	}

	fn geography_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<NamedFeature> {
		let (_, _, Some(store), _) = read else {
			return Vec::new();
		};
		geography_features(store, region)
	}

	fn urban_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<NamedFeature> {
		let (_, urban, _, _) = read;
		urban_features(urban, region)
	}

	fn places_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<NamedPlace> {
		let (_, _, _, places) = read;
		place_features(places, region)
	}

	fn groves_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
		index: &LanguageIndex,
	) -> FeatureSnapshot {
		let (forests, _, _, _) = read;
		grove_snapshot(forests, region, index)
	}

	fn geography_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
		index: &LanguageIndex,
	) -> FeatureSnapshot {
		let (_, _, Some(store), _) = read else {
			return FeatureSnapshot::default();
		};
		geography_snapshot(store, region, index)
	}

	fn urban_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
		index: &LanguageIndex,
	) -> FeatureSnapshot {
		let (_, urban, _, _) = read;
		urban_snapshot(urban, region, index)
	}

	fn places_snapshot(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
		index: &LanguageIndex,
	) -> PlaceSnapshot {
		let (_, _, _, places) = read;
		place_snapshot(places, region, index)
	}
}

fn grove_features(index: &ForestIndex, region: Aabb3d) -> Vec<NamedFeature> {
	grove_snapshot(index, region, &LanguageIndex::default()).work
}

fn grove_snapshot(
	forests: &ForestIndex,
	region: Aabb3d,
	language: &LanguageIndex,
) -> FeatureSnapshot {
	let mut active = HashSet::new();
	let mut work = Vec::new();
	for TrackedId(id) in SpatialIndex::<ChicoForest>::tracked_ids_for(forests, region) {
		let key = NameKey::Forest(id);
		active.insert(key);
		let revision = SpatialIndex::<ChicoForest>::version(forests, id).map(|v| v.0).unwrap_or(0);
		if language.is_current_revision(key, revision) {
			continue;
		}
		let Some(forest) = SpatialIndex::<ChicoForest>::get(forests, id) else {
			continue;
		};
		let Some(bounds) = SpatialIndex::<ChicoForest>::get_bounds(forests, id) else {
			continue;
		};
		let kinds = [
			forest.layers.tufts,
			forest.layers.understory,
			forest.layers.lower_canopy,
			forest.layers.upper_canopy,
		]
		.into_iter()
		.flatten();
		work.push(NamedFeature::new(
			key,
			bounds,
			named_forest_english(forest.layers.layering, kinds, name_key_salt(key)),
			revision,
		));
	}
	for TrackedId(id) in SpatialIndex::<ChicoGrove>::tracked_ids_for(forests, region) {
		let key = NameKey::Grove(id);
		active.insert(key);
		let revision = SpatialIndex::<ChicoGrove>::version(forests, id).map(|v| v.0).unwrap_or(0);
		if language.is_current_revision(key, revision) {
			continue;
		}
		let Some(grove) = SpatialIndex::<ChicoGrove>::get(forests, id) else {
			continue;
		};
		let Some(bounds) = SpatialIndex::<ChicoGrove>::get_bounds(forests, id) else {
			continue;
		};
		work.push(NamedFeature::new(
			key,
			bounds,
			named_grove_english(grove.recipes.iter().map(|recipe| recipe.kind), name_key_salt(key)),
			revision,
		));
	}
	FeatureSnapshot { active, work }
}

fn geography_features(store: &TerrainEntryStore, region: Aabb3d) -> Vec<NamedFeature> {
	geography_snapshot(store, region, &LanguageIndex::default()).work
}

fn geography_snapshot(
	store: &TerrainEntryStore,
	region: Aabb3d,
	language: &LanguageIndex,
) -> FeatureSnapshot {
	let query = Bounds2::from_xz(region.min.x, region.min.z, region.max.x, region.max.z);
	let mut active = HashSet::new();
	let mut work = Vec::new();
	for feature in store.geographic_features_overlapping(query) {
		let key = NameKey::Geographic(feature.id);
		active.insert(key);
		if language.is_current_revision(key, feature.revision.0) {
			continue;
		}
		let bounds = Aabb3d::from_min_max(
			bevy::math::Vec3::new(feature.bounds.min.x, -1.0, feature.bounds.min.y),
			bevy::math::Vec3::new(feature.bounds.max.x, 1.0, feature.bounds.max.y),
		);
		work.push(NamedFeature::new(
			key,
			bounds,
			named_geographic_english(feature.kind, name_key_salt(key)),
			feature.revision.0,
		));
	}
	FeatureSnapshot { active, work }
}

fn urban_features(index: &UrbanizationIndex, region: Aabb3d) -> Vec<NamedFeature> {
	urban_snapshot(index, region, &LanguageIndex::default()).work
}

fn urban_snapshot(
	index: &UrbanizationIndex,
	region: Aabb3d,
	language: &LanguageIndex,
) -> FeatureSnapshot {
	let mut active = HashSet::new();
	let mut work = Vec::new();
	for TrackedId(id) in SpatialIndex::<SelectedUrbanization>::tracked_ids_for(index, region) {
		let Some(cell) = SpatialIndex::<SelectedUrbanization>::get(index, id) else {
			continue;
		};
		let Some(bounds) = SpatialIndex::<SelectedUrbanization>::get_bounds(index, id) else {
			continue;
		};
		let revision = SpatialIndex::<SelectedUrbanization>::version(index, id)
			.map(|v| v.0)
			.unwrap_or(0);
		let cell_key = NameKey::Urban(id);
		active.insert(cell_key);
		if !language.is_current_revision(cell_key, revision) {
			let english = named_urban_english(cell.kind, None, name_key_salt(cell_key));
			work.push(NamedFeature::new(cell_key, bounds, english, revision));
		}
		for leaf in &cell.leaves {
			let leaf_key = NameKey::UrbanLeaf(leaf.id());
			active.insert(leaf_key);
			if language.is_current_revision(leaf_key, revision) {
				continue;
			}
			work.push(NamedFeature::new(
				leaf_key,
				leaf.bounds,
				named_urban_english(cell.kind, Some(leaf.kind), name_key_salt(leaf_key)),
				revision,
			));
		}
	}
	FeatureSnapshot { active, work }
}

fn place_features(
	places: &Query<'_, '_, (&DiscoverablePlace, &GlobalTransform)>,
	region: Aabb3d,
) -> Vec<NamedPlace> {
	place_snapshot(places, region, &LanguageIndex::default()).work
}

fn place_snapshot(
	places: &Query<'_, '_, (&DiscoverablePlace, &GlobalTransform)>,
	region: Aabb3d,
	language: &LanguageIndex,
) -> PlaceSnapshot {
	let mut active = HashSet::new();
	let mut work = Vec::new();
	for (place, transform) in places.iter() {
		let world = transform.translation();
		if !xz_contains(region, world.x, world.z) {
			continue;
		}
		let xz = Vec2::new(world.x, world.z);
		let (key, provisional, inherit_host_language) = place_key(place, xz);
		active.insert(key);
		if !inherit_host_language && language.is_current_revision(key, 1) {
			continue;
		}
		let english = named_place_english(
			place.label,
			place_identity_bits(place) ^ u64::from(xz.x.to_bits()) ^ u64::from(xz.y.to_bits()),
		);
		let fingerprint = terms_fingerprint(&english);
		work.push(NamedPlace {
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
	PlaceSnapshot { active, work }
}

/// World XZ only. Elevated POIs stay eligible when they sit over the region.
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

fn quantize_place_axis(value: f32) -> i32 {
	(value / PLACE_QUANT_M).round() as i32
}

fn place_dependency_item(place: &DiscoverablePlace, xz: Vec2) -> u64 {
	let identity = place_identity_bits(place);
	let persistent = u64::from(place.persistent);
	let label = place.label.salt();
	let mut item = mix(identity) ^ mix(persistent) ^ mix(label);
	if place.host.is_none() {
		item ^= mix(quantize_place_axis(xz.x) as u64) ^ mix(quantize_place_axis(xz.y) as u64);
	}
	item
}

fn places_signature(places: &Query<'_, '_, (&DiscoverablePlace, &GlobalTransform)>) -> u64 {
	let mut count = 0u64;
	let mut acc = 0u64;
	for (place, transform) in places.iter() {
		count += 1;
		let translation = transform.translation();
		acc =
			acc.wrapping_add(place_dependency_item(place, Vec2::new(translation.x, translation.z)));
	}
	mix(count) ^ acc
}

#[cfg(test)]
pub(crate) fn places_signature_from_world_xz(
	places: impl IntoIterator<Item = (DiscoverablePlace, bevy::math::Vec3)>,
) -> u64 {
	let mut count = 0u64;
	let mut acc = 0u64;
	for (place, world) in places {
		count += 1;
		acc = acc.wrapping_add(place_dependency_item(&place, Vec2::new(world.x, world.z)));
	}
	mix(count) ^ acc
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
		let english = named_place_english(
			place.label,
			place_identity_bits(&place) ^ u64::from(xz.x.to_bits()) ^ u64::from(xz.y.to_bits()),
		);
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
