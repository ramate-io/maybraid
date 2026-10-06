//! Consumer-owned reads of groves, urbanization, geography, and POIs.

use bevy::ecs::system::{ReadOnlySystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::{GlobalTransform, Query, Res};
use chico::{Chico, ChicoForest, ChicoGrove, ForestIndex};
use durham::{HcsgStorage, TerrainStorage};
use lod::gen::{SpatialIndex, TrackedId};
use procedural_common::Bounds2;
use richmond::{DiscoverablePlace, Richmond};
use urbanization_cells::UrbanizationStorage;
use urbanization_layer_model::Urbanization;
use vegetation_layer_model::Vegetation;

use maybraid_language_core::lexicalizer::mix;

use crate::english::{
	named_forest_english, named_geographic_english, named_grove_english, named_place_english,
	named_urban_english,
};
use crate::index::{name_key_salt, NameKey};
use crate::name::terms_fingerprint;

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
}

type WorldRead = (
	Res<'static, ForestIndex>,
	Option<Res<'static, HcsgStorage>>,
	Query<'static, 'static, (&'static DiscoverablePlace, &'static GlobalTransform)>,
);

impl<T: 'static> NamedWorld for Vegetation<Chico<Urbanization<Richmond<T>>>> {
	type Read = WorldRead;

	fn source_revisions(read: &SystemParamItem<'_, '_, Self::Read>) -> SourceRevisions {
		let (forests, storage, places) = read;
		SourceRevisions {
			forest: forests.membership_revision(),
			urban: storage.as_ref().map(|store| store.urbanization_revision()).unwrap_or(0),
			terrain: storage.as_ref().map(|store| store.geography_revision()).unwrap_or(0),
			places: places_signature(places),
		}
	}

	fn groves_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<NamedFeature> {
		let (forests, _, _) = read;
		grove_features(forests, region)
	}

	fn geography_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<NamedFeature> {
		let (_, Some(store), _) = read else {
			return Vec::new();
		};
		geography_features(store, region)
	}

	fn urban_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<NamedFeature> {
		let (_, Some(store), _) = read else {
			return Vec::new();
		};
		urban_features(store, region)
	}

	fn places_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<NamedPlace> {
		let (_, _, places) = read;
		place_features(places, region)
	}
}

fn grove_features(index: &ForestIndex, region: Aabb3d) -> Vec<NamedFeature> {
	let mut out = Vec::new();
	for TrackedId(id) in SpatialIndex::<ChicoForest>::tracked_ids_for(index, region) {
		let Some(forest) = SpatialIndex::<ChicoForest>::get(index, id) else {
			continue;
		};
		let Some(bounds) = SpatialIndex::<ChicoForest>::get_bounds(index, id) else {
			continue;
		};
		let revision = SpatialIndex::<ChicoForest>::version(index, id).map(|v| v.0).unwrap_or(0);
		let kinds = [
			forest.layers.tufts,
			forest.layers.understory,
			forest.layers.lower_canopy,
			forest.layers.upper_canopy,
		]
		.into_iter()
		.flatten();
		let key = NameKey::Forest(id);
		out.push(NamedFeature::new(
			key,
			bounds,
			named_forest_english(forest.layers.layering, kinds, name_key_salt(key)),
			revision,
		));
	}
	for TrackedId(id) in SpatialIndex::<ChicoGrove>::tracked_ids_for(index, region) {
		let Some(grove) = SpatialIndex::<ChicoGrove>::get(index, id) else {
			continue;
		};
		let Some(bounds) = SpatialIndex::<ChicoGrove>::get_bounds(index, id) else {
			continue;
		};
		let revision = SpatialIndex::<ChicoGrove>::version(index, id).map(|v| v.0).unwrap_or(0);
		let key = NameKey::Grove(id);
		out.push(NamedFeature::new(
			key,
			bounds,
			named_grove_english(grove.recipes.iter().map(|recipe| recipe.kind), name_key_salt(key)),
			revision,
		));
	}
	out
}

fn geography_features(store: &HcsgStorage, region: Aabb3d) -> Vec<NamedFeature> {
	let query = Bounds2::from_xz(region.min.x, region.min.z, region.max.x, region.max.z);
	store
		.geographic_features_overlapping(query)
		.map(|feature| {
			let bounds = Aabb3d::from_min_max(
				bevy::math::Vec3::new(feature.bounds.min.x, -1.0, feature.bounds.min.y),
				bevy::math::Vec3::new(feature.bounds.max.x, 1.0, feature.bounds.max.y),
			);
			NamedFeature::new(
				NameKey::Geographic(feature.id),
				bounds,
				named_geographic_english(
					feature.kind,
					name_key_salt(NameKey::Geographic(feature.id)),
				),
				feature.revision.0,
			)
		})
		.collect()
}

fn urban_features(store: &HcsgStorage, region: Aabb3d) -> Vec<NamedFeature> {
	let mut out = Vec::new();
	for (id, entry) in store.selected_overlapping(region) {
		let (cell, bounds, revision) = (&entry.value, entry.bounds, entry.version.0);
		let cell_key = NameKey::Urban(id);
		let english = named_urban_english(cell.kind, None, name_key_salt(cell_key));
		out.push(NamedFeature::new(cell_key, bounds, english, revision));
		for leaf in &cell.leaves {
			let leaf_key = NameKey::UrbanLeaf(leaf.id());
			out.push(NamedFeature::new(
				leaf_key,
				leaf.bounds,
				named_urban_english(cell.kind, Some(leaf.kind), name_key_salt(leaf_key)),
				revision,
			));
		}
	}
	out
}

fn place_features(
	places: &Query<'_, '_, (&DiscoverablePlace, &GlobalTransform)>,
	region: Aabb3d,
) -> Vec<NamedPlace> {
	let mut out = Vec::new();
	for (place, transform) in places.iter() {
		let world = transform.translation();
		if !xz_contains(region, world.x, world.z) {
			continue;
		}
		let xz = Vec2::new(world.x, world.z);
		let (key, provisional, inherit_host_language) = place_key(place, xz);
		let english = named_place_english(
			place.label,
			place_identity_bits(place) ^ u64::from(xz.x.to_bits()) ^ u64::from(xz.y.to_bits()),
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

fn places_signature(places: &Query<'_, '_, (&DiscoverablePlace, &GlobalTransform)>) -> u64 {
	let mut items: Vec<_> = places
		.iter()
		.map(|(place, transform)| {
			let t = transform.translation();
			(
				place_identity_bits(place),
				place.persistent,
				place.label.salt(),
				t.x.to_bits(),
				t.z.to_bits(),
			)
		})
		.collect();
	items.sort_unstable();
	let mut sig = mix(items.len() as u64);
	for (identity, persistent, label, x, z) in items {
		sig = mix(sig
			^ mix(identity)
			^ mix(u64::from(persistent))
			^ mix(label)
			^ mix(u64::from(x))
			^ mix(u64::from(z)));
	}
	sig
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
