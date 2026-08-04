//! Rectangular ring storey shell: outer + inner wall loops + frame slabs.
//!
//! Outer walls walk CCW and inner walls walk CW so both face the gallery between
//! them. There is no separate omit-interval API — author courtyard breaks and
//! broad wall omissions with [`Openings`] (wide `Passage` / `Aperture` AABBs on
//! the hit outer or inner side). Openings fit to authored AABB positions on the
//! hit side / frame band — not centered approximations.
//!
//! **Walls:** each connectable opening maps to **exactly one** outer or inner
//! side (largest projected along-span wins, then nearest mid — so a corner
//! depth-nibble cannot steal a true leaf). Slight span shortfall truncates the
//! leaf (up to ~0.4 m); larger overruns leave the opening unmapped. A single
//! AABB that spans half the ring does **not** clear every wall on that half —
//! open a U / half-ring by authoring one opening per side you want removed. A
//! wide passage spanning most of one side is the intended way to open that run.
//! **Slabs:** only [`OpeningLabel::cuts_slab`] labels cut Solid floor / ceiling.

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
use crate::paneling::fitted_rectangle::{ClippedFittedRectangle, FittedRectangle};
use crate::paneling::panel_complex::DEFAULT_PANEL_THICKNESS;

use crate::shells::ortho::WallEdge;

pub use openings::{RectRingFloorSide, OPENING_SPAN_TRUNCATE_MAX};

/// Horizontal storey slab presentation for towering ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RectRingFloorSlab {
	None,
	Solid,
}

impl Default for RectRingFloorSlab {
	fn default() -> Self {
		Self::None
	}
}

/// Authored parameters / builder for a [`RectRingFloor`] shell.
#[derive(Debug, Clone, PartialEq)]
pub struct RectRingFloorParams {
	/// Storey plan center; `y` is the floor elevation.
	pub center_xz: Vec3,
	/// Full outer width (X) / depth (Z).
	pub outer: Vec2,
	/// Full inner courtyard width (X) / depth (Z). Must be smaller than
	/// [`Self::outer`] on both axes.
	pub inner: Vec2,
	pub storey_height: f32,
	/// Wall / slab voids. Prefer wide connectable openings to author broad
	/// omissions along outer or inner sides of the ring.
	///
	/// One opening → one wall side. To remove several sides (e.g. a U / half
	/// ring), supply one opening per side.
	pub openings: Openings,
	pub floor: RectRingFloorSlab,
	pub ceiling: RectRingFloorSlab,
	pub style: PanelStyle,
	pub joint_thickness: f32,
}

impl Default for RectRingFloorParams {
	fn default() -> Self {
		Self {
			center_xz: Vec3::ZERO,
			outer: Vec2::new(8.0, 6.0),
			inner: Vec2::new(4.0, 3.0),
			storey_height: 3.0,
			openings: Openings::new(),
			floor: RectRingFloorSlab::None,
			ceiling: RectRingFloorSlab::None,
			style: PanelStyle::RoughStonework,
			joint_thickness: DEFAULT_PANEL_THICKNESS,
		}
	}
}

impl RectRingFloorParams {
	pub fn new(center_xz: Vec3, outer: Vec2, inner: Vec2, storey_height: f32) -> Self {
		Self {
			center_xz,
			outer,
			inner,
			storey_height,
			..Self::default()
		}
	}

	pub fn floor(mut self, floor: RectRingFloorSlab) -> Self {
		self.floor = floor;
		self
	}

	pub fn ceiling(mut self, ceiling: RectRingFloorSlab) -> Self {
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

	pub fn joint_thickness(mut self, thickness: f32) -> Self {
		self.joint_thickness = thickness;
		self
	}

	pub fn build(self) -> RectRingFloor {
		RectRingFloor::from_params(self)
	}
}

/// One rectangular-ring storey: outer/inner wall strips + optional frame slabs.
#[derive(Debug, Clone, PartialEq)]
pub struct RectRingFloor {
	params: RectRingFloorParams,
	walls: Vec<ClippedRectangularStrip>,
	edges: Vec<WallEdge>,
	floor_pieces: Vec<RingSlabPiece>,
	ceiling_pieces: Vec<RingSlabPiece>,
	openings: Openings,
	mapped: MappedOpenings,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RingSlabPiece {
	Solid(FittedRectangle),
	Clipped(ClippedFittedRectangle),
}

impl RingSlabPiece {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		match self {
			Self::Solid(r) => r.panel_nodes_for_level(level),
			Self::Clipped(r) => r.panel_nodes_for_level(level),
		}
	}
}

impl RectRingFloor {
	pub fn new(params: RectRingFloorParams) -> Self {
		Self::from_params(params)
	}

	fn from_params(params: RectRingFloorParams) -> Self {
		let (outer, inner) = sanitize_pair(params.outer, params.inner);
		let storey_height = params.storey_height.max(1e-4);
		let center_xz = Vec3::new(params.center_xz.x, params.center_xz.y, params.center_xz.z);
		let params = RectRingFloorParams {
			center_xz,
			outer,
			inner,
			storey_height,
			..params
		};

		let geom = params.resolve_geometry();
		let (walls, openings, mapped) = params.resolve_walls(&geom.edges);
		let floor_pieces = params.resolve_slab_pieces(params.floor, &geom.slab_rects, geom.y0);
		let ceiling_pieces =
			params.resolve_slab_pieces(params.ceiling, &geom.slab_rects, geom.y0 + storey_height);

		Self {
			params,
			walls,
			edges: geom.edges,
			floor_pieces,
			ceiling_pieces,
			openings,
			mapped,
		}
	}

	pub fn params(&self) -> &RectRingFloorParams {
		&self.params
	}

	pub fn walls(&self) -> &[ClippedRectangularStrip] {
		&self.walls
	}

	pub fn edges(&self) -> &[WallEdge] {
		&self.edges
	}

	pub fn wall_count(&self) -> usize {
		self.walls.len()
	}

	pub fn has_floor(&self) -> bool {
		!self.floor_pieces.is_empty()
	}

	/// Number of frame floor band pieces (after shaft / cut subdivision).
	pub fn floor_band_count(&self) -> usize {
		self.floor_pieces.len()
	}

	pub fn has_ceiling(&self) -> bool {
		!self.ceiling_pieces.is_empty()
	}
}

impl BuildingComponents for RectRingFloor {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for w in &self.walls {
			out.extend(w.panel_nodes_for_level(level));
		}
		for p in &self.floor_pieces {
			out.extend(p.panel_nodes_for_level(level));
		}
		for p in &self.ceiling_pieces {
			out.extend(p.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for w in &self.walls {
			out.extend(w.joint_nodes_for_level(level));
		}
		out
	}
}

fn sanitize_pair(outer: Vec2, inner: Vec2) -> (Vec2, Vec2) {
	let outer = Vec2::new(outer.x.max(1e-3), outer.y.max(1e-3));
	let mut inner = Vec2::new(inner.x.max(1e-3), inner.y.max(1e-3));
	// Keep a thin gallery on each axis.
	const MIN_BAND: f32 = 0.25;
	if inner.x >= outer.x - MIN_BAND {
		inner.x = (outer.x - MIN_BAND).max(1e-3);
	}
	if inner.y >= outer.y - MIN_BAND {
		inner.y = (outer.y - MIN_BAND).max(1e-3);
	}
	(outer, inner)
}
