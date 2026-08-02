//! Orthonormal rounded-rectangle storey: straight rectangle walls + ruled corners.
//!
//! Straight runs use [`ClippedRectangularStrip`] (ordinary rectangle kits). Corner
//! quarter-cylinders use [`ClippedRuledStrip`]. Openings fit to authored AABB
//! positions on the hit span / slab piece.

mod geometry;
mod openings;
mod slabs;

#[cfg(test)]
mod tests;

use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::openings::{MappedOpenings, Openings};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::paneling::clipped_ruled_strip::ClippedRuledStrip;
use crate::paneling::fitted_rectangle::{ClippedFittedRectangle, FittedRectangle};
use crate::paneling::panel_complex::{PanelComplex, DEFAULT_PANEL_THICKNESS};

use crate::shells::ortho::PlanRect;

pub use geometry::RoundedRectCorner;
pub use openings::RoundedRectFloorSide;

/// Horizontal storey slab presentation for towering ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundedRectFloorSlab {
	None,
	Solid,
}

impl Default for RoundedRectFloorSlab {
	fn default() -> Self {
		Self::None
	}
}

/// Authored parameters / builder for a [`RoundedRectFloor`] shell.
#[derive(Debug, Clone, PartialEq)]
pub struct RoundedRectFloorParams {
	pub center_xz: Vec3,
	pub footprint: Vec2,
	pub storey_height: f32,
	/// Corner radius; clamped so each straight run stays ≥ ε.
	pub corner_radius: f32,
	/// Samples along each quarter-cylinder (excluding endpoints already on straights).
	pub corner_segments: u32,
	pub openings: Openings,
	pub floor: RoundedRectFloorSlab,
	pub ceiling: RoundedRectFloorSlab,
	pub style: PanelStyle,
	pub joint_thickness: f32,
}

impl Default for RoundedRectFloorParams {
	fn default() -> Self {
		Self {
			center_xz: Vec3::ZERO,
			footprint: Vec2::new(8.0, 6.0),
			storey_height: 3.0,
			corner_radius: 1.0,
			corner_segments: 4,
			openings: Openings::new(),
			floor: RoundedRectFloorSlab::None,
			ceiling: RoundedRectFloorSlab::None,
			style: PanelStyle::RoughStonework,
			joint_thickness: DEFAULT_PANEL_THICKNESS,
		}
	}
}

impl RoundedRectFloorParams {
	pub fn new(center_xz: Vec3, footprint: Vec2, storey_height: f32, corner_radius: f32) -> Self {
		Self {
			center_xz,
			footprint,
			storey_height,
			corner_radius,
			..Self::default()
		}
	}

	pub fn floor(mut self, floor: RoundedRectFloorSlab) -> Self {
		self.floor = floor;
		self
	}

	pub fn ceiling(mut self, ceiling: RoundedRectFloorSlab) -> Self {
		self.ceiling = ceiling;
		self
	}

	pub fn style(mut self, style: PanelStyle) -> Self {
		self.style = style;
		self
	}

	pub fn openings(mut self, openings: Openings) -> Self {
		self.openings = openings;
		self
	}

	pub fn corner_segments(mut self, segments: u32) -> Self {
		self.corner_segments = segments;
		self
	}

	pub fn joint_thickness(mut self, thickness: f32) -> Self {
		self.joint_thickness = thickness;
		self
	}

	pub fn build(self) -> RoundedRectFloor {
		RoundedRectFloor::from_params(self)
	}

	pub(crate) fn plan(&self) -> PlanRect {
		PlanRect::new(self.center_xz, self.footprint.x, self.footprint.y)
	}

	pub(crate) fn clamped_radius(&self) -> f32 {
		let plan = self.plan();
		let max_r = (plan.half_x.min(plan.half_z) - 1e-3).max(0.0);
		self.corner_radius.clamp(0.0, max_r)
	}
}

/// One rounded-rectangle storey.
#[derive(Debug, Clone, PartialEq)]
pub struct RoundedRectFloor {
	params: RoundedRectFloorParams,
	straights: [ClippedRectangularStrip; 4],
	corners: [ClippedRuledStrip; 4],
	floor_core: Option<RoundedSlabPiece>,
	floor_quarters: Vec<PanelComplex>,
	ceiling_core: Option<RoundedSlabPiece>,
	ceiling_quarters: Vec<PanelComplex>,
	openings: Openings,
	mapped: MappedOpenings,
	radius: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RoundedSlabPiece {
	Solid(FittedRectangle),
	Clipped(ClippedFittedRectangle),
}

impl RoundedSlabPiece {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		match self {
			Self::Solid(r) => r.panel_nodes_for_level(level),
			Self::Clipped(r) => r.panel_nodes_for_level(level),
		}
	}
}

impl RoundedRectFloor {
	pub fn new(params: RoundedRectFloorParams) -> Self {
		Self::from_params(params)
	}

	fn from_params(params: RoundedRectFloorParams) -> Self {
		let footprint = Vec2::new(params.footprint.x.max(1e-4), params.footprint.y.max(1e-4));
		let storey_height = params.storey_height.max(1e-4);
		let center_xz = Vec3::new(params.center_xz.x, params.center_xz.y, params.center_xz.z);
		let corner_segments = params.corner_segments.max(1);
		let params = RoundedRectFloorParams {
			center_xz,
			footprint,
			storey_height,
			corner_segments,
			..params
		};
		let radius = params.clamped_radius();
		let plan = params.plan();
		let geom = params.resolve_geometry(plan, radius);

		let (straights, corners, openings, mapped) = params.resolve_walls(&geom);

		let y1 = plan.y + storey_height;
		let (floor_core, floor_quarters) = params.resolve_slab_parts(params.floor, plan, radius);
		let ceil_plan = PlanRect::new(
			Vec3::new(plan.center.x, y1, plan.center.z),
			plan.full_x(),
			plan.full_z(),
		);
		let (ceiling_core, ceiling_quarters) =
			params.resolve_slab_parts(params.ceiling, ceil_plan, radius);

		Self {
			params,
			straights,
			corners,
			floor_core,
			floor_quarters,
			ceiling_core,
			ceiling_quarters,
			openings,
			mapped,
			radius,
		}
	}

	pub fn params(&self) -> &RoundedRectFloorParams {
		&self.params
	}

	pub fn corner_radius(&self) -> f32 {
		self.radius
	}

	pub fn straights(&self) -> &[ClippedRectangularStrip; 4] {
		&self.straights
	}

	pub fn corners(&self) -> &[ClippedRuledStrip; 4] {
		&self.corners
	}

	pub fn has_floor(&self) -> bool {
		self.floor_core.is_some() || !self.floor_quarters.is_empty()
	}

	pub fn has_ceiling(&self) -> bool {
		self.ceiling_core.is_some() || !self.ceiling_quarters.is_empty()
	}
}

impl BuildingComponents for RoundedRectFloor {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for s in &self.straights {
			out.extend(s.panel_nodes_for_level(level));
		}
		for c in &self.corners {
			out.extend(c.panel_nodes_for_level(level));
		}
		if let Some(core) = &self.floor_core {
			out.extend(core.panel_nodes_for_level(level));
		}
		for q in &self.floor_quarters {
			out.extend(q.panel_nodes_for_level(level));
		}
		if let Some(core) = &self.ceiling_core {
			out.extend(core.panel_nodes_for_level(level));
		}
		for q in &self.ceiling_quarters {
			out.extend(q.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for s in &self.straights {
			out.extend(s.joint_nodes_for_level(level));
		}
		for c in &self.corners {
			out.extend(c.joint_nodes_for_level(level));
		}
		out
	}
}
