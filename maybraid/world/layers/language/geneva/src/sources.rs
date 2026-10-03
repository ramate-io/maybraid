//! Consumer-owned reads of groves, urbanization, geography, and POIs.

use bevy::ecs::system::{ReadOnlySystemParam, SystemParamItem};
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::Vec2;
use bevy::prelude::{Query, Res, Transform};
use chico::{Chico, ChicoForest, ChicoGrove, ForestIndex, ForestGroveKind};
use lod::gen::{SpatialIndex, TrackedId};
use richmond::{DiscoverablePlace, DiscoverablePlaceLabel, Richmond};
use urbanization_cells::{SelectedUrbanization, UrbanizationIndex};
use urbanization_layer_model::Urbanization;
use vegetation_layer_model::Vegetation;

use crate::english::english_words;
use crate::index::NameKey;

/// A generated cell the language layer may name.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedFeature {
	pub key: NameKey,
	pub bounds: Aabb3d,
	pub english: Vec<String>,
}

/// A POI the language layer may name.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedPlace {
	pub key: NameKey,
	pub xz: Vec2,
	pub english: Vec<String>,
	pub persistent: bool,
}

/// What Geneva reads from the vegetated world under it.
///
/// Vegetation, urbanization, and POI contracts do not grow a naming method.
pub trait NamedWorld: Send + Sync + 'static {
	type Read: ReadOnlySystemParam + 'static;

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
	Res<'static, UrbanizationIndex>,
	Query<'static, 'static, (&'static DiscoverablePlace, &'static Transform)>,
);

impl<T: 'static> NamedWorld for Vegetation<Chico<Urbanization<Richmond<T>>>> {
	type Read = WorldRead;

	fn groves_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<NamedFeature> {
		let (forests, _, _) = read;
		grove_features(forests, region)
	}

	fn geography_overlapping(
		_read: &SystemParamItem<'_, '_, Self::Read>,
		_region: Aabb3d,
	) -> Vec<NamedFeature> {
		Vec::new()
	}

	fn urban_overlapping(
		read: &SystemParamItem<'_, '_, Self::Read>,
		region: Aabb3d,
	) -> Vec<NamedFeature> {
		let (_, urban, _) = read;
		urban_features(urban, region)
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
		let mut english = english_words(&format!("{:?}", forest.layers.layering));
		for kind in [
			forest.layers.tufts,
			forest.layers.understory,
			forest.layers.lower_canopy,
			forest.layers.upper_canopy,
		]
		.into_iter()
		.flatten()
		{
			english.extend(grove_kind_english(kind));
		}
		out.push(NamedFeature { key: NameKey::Forest(id), bounds, english });
	}
	for TrackedId(id) in SpatialIndex::<ChicoGrove>::tracked_ids_for(index, region) {
		let Some(grove) = SpatialIndex::<ChicoGrove>::get(index, id) else {
			continue;
		};
		let Some(bounds) = SpatialIndex::<ChicoGrove>::get_bounds(index, id) else {
			continue;
		};
		let mut english = Vec::new();
		for recipe in &grove.recipes {
			english.extend(grove_kind_english(recipe.kind));
		}
		out.push(NamedFeature { key: NameKey::Grove(id), bounds, english });
	}
	out
}

fn urban_features(index: &UrbanizationIndex, region: Aabb3d) -> Vec<NamedFeature> {
	let mut out = Vec::new();
	for TrackedId(id) in SpatialIndex::<SelectedUrbanization>::tracked_ids_for(index, region) {
		let Some(cell) = SpatialIndex::<SelectedUrbanization>::get(index, id) else {
			continue;
		};
		let Some(bounds) = SpatialIndex::<SelectedUrbanization>::get_bounds(index, id) else {
			continue;
		};
		let english = english_words(&format!("{:?}", cell.kind));
		out.push(NamedFeature { key: NameKey::Urban(id), bounds, english: english.clone() });
		for leaf in &cell.leaves {
			let mut leaf_english = english.clone();
			leaf_english.extend(english_words(&format!("{:?}", leaf.kind)));
			out.push(NamedFeature {
				key: NameKey::UrbanLeaf(leaf.id()),
				bounds: leaf.bounds,
				english: leaf_english,
			});
		}
	}
	out
}

fn place_features(
	places: &Query<'_, '_, (&DiscoverablePlace, &Transform)>,
	region: Aabb3d,
) -> Vec<NamedPlace> {
	let mut out = Vec::new();
	for (place, transform) in places.iter() {
		let xz = Vec2::new(transform.translation.x, transform.translation.z);
		let point = Aabb3d::from_min_max(
			transform.translation - bevy::math::Vec3::splat(0.5),
			transform.translation + bevy::math::Vec3::splat(0.5),
		);
		if !region.intersects(&point) {
			continue;
		}
		out.push(NamedPlace {
			key: NameKey::Place {
				qx: xz.x.round() as i32,
				qz: xz.y.round() as i32,
				label: place_label_tag(place.label),
			},
			xz,
			english: english_words(&format!("{:?}", place.label)),
			persistent: place.persistent,
		});
	}
	out
}

fn grove_kind_english(kind: ForestGroveKind) -> Vec<String> {
	english_words(&format!("{kind:?}"))
}

fn place_label_tag(label: DiscoverablePlaceLabel) -> u32 {
	label.salt() as u32
}
