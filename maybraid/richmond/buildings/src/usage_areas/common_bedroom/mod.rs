//! Common bedroom usage area: beds / nightstands + walled closet / ensuite rooms.
//!
//! **Semantically:** a residential cell with passage keep-outs, sleep furniture,
//! and private partitions (closet, ensuite) that open onto the bedroom floor.
//!
//! **Programmatically:**
//! 1. [`CommonBedroomParameterized::sample`] (or [`CommonBedroomParameterized::with_fill`])
//! 2. [`CommonBedroomPlan::from_parameterized`] packs clearances → beds →
//!    nightstands → [`crate::usage_areas::enclosed_room`] closets / ensuites
//! 3. [`CommonBedroom`] presents furniture + partition panels; residual
//!    [`FillableRegions::within`] carry closet / ensuite confines with doors

mod layout;
mod parameterized;

pub use parameterized::{CommonBedroomParameterized, CommonBedroomPlan, SCOPE};

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::furniture::FurnitureNode;
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::bedroom::placement_filling_aabb;
use crate::fit::{
	Confines, FillRegion, FillableRegions, Fit, FitError, SpaceKind,
};
use crate::openings::{Opening, OpeningId, Openings};
use crate::paneling::Rectangle;
use crate::usage_areas::label_util::label_filling_aabb;

use layout::BedroomPartition;

/// Presentable common bedroom: furniture + closet / ensuite enclosure panels.
#[derive(Debug, Clone, PartialEq)]
pub struct CommonBedroom {
	pub room_type: LabelNode,
	pub beds: Vec<FurnitureNode>,
	pub nightstands: Vec<FurnitureNode>,
	pub closet_walls: Vec<Rectangle>,
	pub ensuite_walls: Vec<Rectangle>,
	pub closets: Vec<ClosetFill>,
	pub ensuites: Vec<EnsuiteFill>,
}

/// Closet residual + wardrobe placeholder.
#[derive(Debug, Clone, PartialEq)]
pub struct ClosetFill {
	pub bounds: Aabb3d,
	pub label: LabelNode,
	pub wardrobe: FurnitureNode,
	pub door_id: OpeningId,
	pub door: Opening,
}

/// Ensuite residual + vanity / toilet placeholders.
#[derive(Debug, Clone, PartialEq)]
pub struct EnsuiteFill {
	pub bounds: Aabb3d,
	pub label: LabelNode,
	pub vanity: FurnitureNode,
	pub toilet: FurnitureNode,
	pub door_id: OpeningId,
	pub door: Opening,
}

impl CommonBedroom {
	pub fn from_plan(plan: CommonBedroomPlan, confines: &Confines) -> Self {
		let style = plan.parameterized.style;
		let beds = plan
			.packed
			.beds
			.iter()
			.map(|aabb| FurnitureNode::bed(placement_filling_aabb(aabb)))
			.collect();
		let nightstands = plan
			.packed
			.nightstands
			.iter()
			.map(|aabb| FurnitureNode::nightstand(placement_filling_aabb(aabb)))
			.collect();

		let mut closet_walls = Vec::new();
		let closets = plan
			.packed
			.closets
			.iter()
			.map(|p| {
				closet_walls.extend(p.walls.iter().cloned());
				closet_fill(p, style, confines.roll)
			})
			.collect();

		let mut ensuite_walls = Vec::new();
		let ensuites = plan
			.packed
			.ensuites
			.iter()
			.map(|p| {
				ensuite_walls.extend(p.walls.iter().cloned());
				ensuite_fill(p, style, confines.roll)
			})
			.collect();

		Self {
			room_type: label_filling_aabb(
				LabelStyle::Blue,
				"CommonBedroom",
				&confines.bounds,
				confines.roll,
			),
			beds,
			nightstands,
			closet_walls,
			ensuite_walls,
			closets,
			ensuites,
		}
	}

	fn partition_regions(&self, roll: f32) -> Vec<FillRegion> {
		let mut out = Vec::new();
		for c in &self.closets {
			let mut openings = Openings::new();
			openings.insert(c.door_id.clone(), c.door.clone());
			out.push(FillRegion::new(
				SpaceKind::InternalSpace,
				Confines::new(c.bounds, roll, openings),
			));
		}
		for e in &self.ensuites {
			let mut openings = Openings::new();
			openings.insert(e.door_id.clone(), e.door.clone());
			out.push(FillRegion::new(
				SpaceKind::InternalSpace,
				Confines::new(e.bounds, roll, openings),
			));
		}
		out
	}
}

fn closet_fill(part: &BedroomPartition, style: LabelStyle, roll: f32) -> ClosetFill {
	ClosetFill {
		bounds: part.bounds,
		label: label_filling_aabb(style, "Closet", &part.bounds, roll),
		wardrobe: FurnitureNode::wardrobe(placement_filling_aabb(&part.bounds)),
		door_id: part.door_id.clone(),
		door: part.door.clone(),
	}
}

fn ensuite_fill(part: &BedroomPartition, style: LabelStyle, roll: f32) -> EnsuiteFill {
	let aabb = &part.bounds;
	let size = aabb.max - aabb.min;
	let vanity_aabb = Aabb3d::from_min_max(
		Vec3::new(aabb.min.x + 0.1, aabb.min.y, aabb.min.z + 0.15),
		Vec3::new(
			aabb.min.x + size.x * 0.55,
			aabb.min.y + 0.85,
			aabb.min.z + 0.55,
		),
	);
	let toilet_aabb = Aabb3d::from_min_max(
		Vec3::new(aabb.max.x - 0.55, aabb.min.y, aabb.max.z - 0.7),
		Vec3::new(aabb.max.x - 0.1, aabb.min.y + 0.75, aabb.max.z - 0.15),
	);
	EnsuiteFill {
		bounds: part.bounds,
		label: label_filling_aabb(style, "Ensuite", &part.bounds, roll),
		vanity: FurnitureNode::vanity(placement_filling_aabb(&vanity_aabb)),
		toilet: FurnitureNode::toilet(placement_filling_aabb(&toilet_aabb)),
		door_id: part.door_id.clone(),
		door: part.door.clone(),
	}
}

impl Fit for CommonBedroom {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = CommonBedroomParameterized::sample(confines, noise)?;
		let plan = CommonBedroomPlan::from_parameterized(params, confines, noise)?;
		let room = Self::from_plan(plan, confines);
		let regions = FillableRegions {
			within: room.partition_regions(confines.roll),
			atop: Vec::new(),
		};
		Ok((room, regions))
	}
}

impl CommonBedroom {
	/// Fit with explicit spaciousness / occupancy (playground `/show bedroom`).
	pub fn fit_with_fill(
		confines: &Confines,
		noise: NoiseParams,
		spaciousness: f32,
		occupancy: f32,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = CommonBedroomParameterized::with_fill(spaciousness, occupancy);
		let plan = CommonBedroomPlan::from_parameterized(params, confines, noise)?;
		let room = Self::from_plan(plan, confines);
		let regions = FillableRegions {
			within: room.partition_regions(confines.roll),
			atop: Vec::new(),
		};
		Ok((room, regions))
	}
}

impl BuildingComponents for CommonBedroom {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for wall in &self.closet_walls {
			out.extend(wall.panel_nodes_for_level(level));
		}
		for wall in &self.ensuite_walls {
			out.extend(wall.panel_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut labels = vec![self.room_type.clone()];
		labels.extend(self.closets.iter().map(|c| c.label.clone()));
		labels.extend(self.ensuites.iter().map(|e| e.label.clone()));
		Layers::from_free(labels)
	}

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		let mut out = Layers::new();
		out.extend(Layers::from_free(self.beds.clone()));
		out.extend(Layers::from_free(self.nightstands.clone()));
		for c in &self.closets {
			out.extend(Layers::from_free(vec![c.wardrobe.clone()]));
		}
		for e in &self.ensuites {
			out.extend(Layers::from_free(vec![e.vanity.clone(), e.toilet.clone()]));
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;
	use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
	use crate::usage_areas::clearance::PASSAGE_CLEARANCE;
	use procedural_common::{aabb3_to_plan, PlanAxes};

	fn roomy_south() -> Confines {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door_a"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(1.5, 0.0, -0.2),
				Vec3::new(2.5, 2.2, 0.2),
			)),
		);
		Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(6.0, 3.0, 6.0)),
			0.0,
			openings,
		)
	}

	#[test]
	fn common_bedroom_places_bed_and_tracks_partition_doors() {
		let confines = roomy_south();
		let (room, regions) = CommonBedroom::fit_with_fill(
			&confines,
			NoiseParams {
				seed: 7,
				..NoiseParams::default()
			},
			1.0,
			0.7,
		)
		.unwrap();
		assert_eq!(room.room_type.text.as_str(), "CommonBedroom");
		assert!(!room.beds.is_empty());
		assert_eq!(regions.within.len(), room.closets.len() + room.ensuites.len());
		for c in &room.closets {
			assert!(matches!(c.door.label, OpeningLabel::Passage));
			assert!(c.door_id.0.contains("closet_door"));
		}
		for e in &room.ensuites {
			assert!(matches!(e.door.label, OpeningLabel::Passage));
			assert!(e.door_id.0.contains("ensuite_door"));
		}
		if !room.closets.is_empty() {
			assert!(!room.closet_walls.is_empty());
		}
		if !room.ensuites.is_empty() {
			assert!(!room.ensuite_walls.is_empty());
		}
	}

	#[test]
	fn common_bedroom_avoids_passage_clearance() {
		let confines = roomy_south();
		let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
		let faces = crate::usage_areas::PassageClearance::collect_faces(&confines, host);
		let bands = crate::usage_areas::PassageClearance::bands_std(host, &faces);
		assert!(!bands.is_empty());
		let params = CommonBedroomParameterized::with_fill(1.0, 0.55);
		let plan = CommonBedroomPlan::from_parameterized(
			params,
			&confines,
			NoiseParams {
				seed: 42,
				..NoiseParams::default()
			},
		)
		.unwrap();
		assert!(!plan.packed.beds.is_empty());
		for bed in &plan.packed.beds {
			let p = aabb3_to_plan(bed, PlanAxes::XZ);
			for band in &bands {
				assert!(
					!procedural_common::intersects_aabb2(p, *band),
					"bed intersects passage clearance (depth ~{PASSAGE_CLEARANCE})"
				);
			}
		}
	}

	#[test]
	fn common_bedroom_soft_fails_tiny_cell() {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(1.5, 2.5, 1.5),
		));
		assert!(matches!(
			CommonBedroom::fit_to_confines(&confines, NoiseParams::default()),
			Err(FitError::TooSmall { .. })
		));
	}
}
