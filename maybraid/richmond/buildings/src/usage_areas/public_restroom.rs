//! Public restroom: ToiletStalls + sinks Labels.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::label_util::label_filling_aabb;
use crate::usage_areas::stall_layout::{facade_band, primary_facade, StallSide};

#[derive(Debug, Clone, PartialEq)]
pub struct PublicRestroom {
	pub toilet_stalls: LabelNode,
	pub public_restroom_sinks: LabelNode,
}

impl Fit for PublicRestroom {
	fn fit_to_confines(
		confines: &Confines,
		_noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let (side, _) = primary_facade(confines);
		let sinks = facade_band(&confines.bounds, side, 1.0, 0.55);
		let toilets = toilet_band(&confines.bounds, side);
		Ok((
			Self {
				toilet_stalls: label_filling_aabb(
					LabelStyle::Gray,
					"ToiletStalls",
					&toilets,
					confines.roll,
				),
				public_restroom_sinks: label_filling_aabb(
					LabelStyle::Cyan,
					"PublicRestroomSinks",
					&sinks,
					confines.roll,
				),
			},
			FillableRegions::empty(),
		))
	}
}

fn toilet_band(bounds: &Aabb3d, entry: StallSide) -> Aabb3d {
	let min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	// Toilets along a side wall adjacent to the entry façade.
	match entry {
		StallSide::South | StallSide::North => Aabb3d::from_min_max(
			Vec3::new(min.x, min.y, min.z + (max.z - min.z) * 0.25),
			Vec3::new(min.x + (max.x - min.x) * 0.4, max.y, max.z - (max.z - min.z) * 0.15),
		),
		StallSide::East | StallSide::West => Aabb3d::from_min_max(
			Vec3::new(min.x + (max.x - min.x) * 0.2, min.y, min.z),
			Vec3::new(max.x - (max.x - min.x) * 0.15, max.y, min.z + (max.z - min.z) * 0.4),
		),
	}
}

impl BuildingComponents for PublicRestroom {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		Layers::from_free(vec![
			self.toilet_stalls.clone(),
			self.public_restroom_sinks.clone(),
		])
	}
}
