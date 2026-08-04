//! Single rectangular livable apartment region (program fill deferred).

use bevy_math::bounding::BoundingVolume;
use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::joints::JointNode;
use richmond_building_components::labels::{LabelNode, LabelStyle};
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::shells::{RectFloor, RectFloorParams, RectFloorSlab};

/// One primary rectangular livable region allocated from an I-frame.
#[derive(Debug, Clone, PartialEq)]
pub struct LivableApartment {
	pub region_id: u32,
	pub confines: Confines,
	/// Optional envelope shell for presentation; interiors unfilled for now.
	pub shell: Option<RectFloor>,
}

impl LivableApartment {
	pub fn from_confines(
		region_id: u32,
		confines: &Confines,
	) -> Result<(Self, FillableRegions), FitError> {
		let min = Vec3::from(confines.bounds.min);
		let max = Vec3::from(confines.bounds.max);
		let footprint = Vec2::new((max.x - min.x).max(0.0), (max.z - min.z).max(0.0));
		let height = (max.y - min.y).max(0.0);
		if footprint.x < 2.0 || footprint.y < 2.0 {
			return Err(FitError::TooSmall {
				reason: "livable_footprint",
			});
		}
		if height < 2.0 {
			return Err(FitError::TooSmall {
				reason: "livable_height",
			});
		}
		let shell = try_shell(confines);
		Ok((
			Self {
				region_id,
				confines: confines.clone(),
				shell,
			},
			FillableRegions {
				within: vec![crate::fit::FillRegion::new(
					crate::fit::SpaceKind::InternalSpace,
					confines.clone(),
				)],
				atop: Vec::new(),
			},
		))
	}
}

fn try_shell(confines: &Confines) -> Option<RectFloor> {
	let min = Vec3::from(confines.bounds.min);
	let max = Vec3::from(confines.bounds.max);
	let footprint = Vec2::new((max.x - min.x).max(0.0), (max.z - min.z).max(0.0));
	let height = (max.y - min.y).max(0.0);
	if footprint.x < 1.5 || footprint.y < 1.5 || height < 2.0 {
		return None;
	}
	let center_xz = Vec3::new(0.5 * (min.x + max.x), min.y, 0.5 * (min.z + max.z));
	Some(RectFloor::new(RectFloorParams {
		center_xz,
		footprint,
		storey_height: height,
		openings: confines.openings.clone(),
		floor: RectFloorSlab::Solid,
		ceiling: RectFloorSlab::None,
		..RectFloorParams::default()
	}))
}

impl Fit for LivableApartment {
	fn fit_to_confines(
		confines: &Confines,
		_noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_confines(0, confines)
	}
}

impl BuildingComponents for LivableApartment {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		if let Some(shell) = &self.shell {
			out.extend(shell.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		if let Some(shell) = &self.shell {
			out.extend(shell.joint_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		let center = Vec3::from(self.confines.bounds.center());
		let extents =
			Vec3::from(self.confines.bounds.max - self.confines.bounds.min).max(Vec3::splat(1e-4));
		out.push_free(LabelNode::rectangle(
			LabelStyle::Blue,
			&format!("Livable {}", self.region_id + 1),
			center,
			extents,
			self.confines.roll,
		));
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use crate::openings::Openings;

	#[test]
	fn fits_rectangle() {
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(8.0, 3.0, 6.0)),
			0.0,
			Openings::new(),
		);
		let (apt, regions) = LivableApartment::from_confines(0, &confines).unwrap();
		assert!(apt.shell.is_some());
		assert_eq!(regions.within.len(), 1);
	}
}
