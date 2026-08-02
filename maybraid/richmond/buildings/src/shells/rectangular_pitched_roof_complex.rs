//! Orthogonal AABB roof complex: pitched roofs with true valleys at L/T junctions.
//!
//! Each [`Aabb3d`] is a roof massing box (top midline = ridge, bottom long edges =
//! wall plates). Side overhang expands eaves; free ends take [`EndCap`] hip/gable.
//! Concave plan corners form valleys via facing pitch-plane intersection, and
//! neighboring [`PitchedRoof`]s are truncated to meet on those valleys.

mod geometry;
mod topology;
mod valleys;

#[cfg(test)]
mod tests;

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use richmond_building_components::joints::JointNode;
use richmond_building_components::panels::{PanelNode, PanelStyle};
use richmond_building_components::{BuildingComponents, Layers};

use crate::paneling::panel_complex::DEFAULT_PANEL_THICKNESS;
use crate::shells::pitched_rectangular_roof::{PitchedRoof, PitchedRoofParams, RoofHalf};

use geometry::VolumeCandidate;
use topology::resolve_junctions;
pub use valleys::ValleySegment;
use valleys::apply_valleys;

/// Side-eave overhang policy (also used for gable end projections).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Overhang {
	/// Absolute meters.
	Fixed(f32),
	/// Fraction of the eave-to-eave (short-axis) span.
	Ratio(f32),
}

impl Default for Overhang {
	fn default() -> Self {
		Self::Fixed(0.3)
	}
}

/// Free-end treatment for massing boxes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EndCap {
	/// Banked hips on free ends.
	Hip,
	/// Gable ends; ridge / eave may project past the massing by the given overhangs.
	Gable {
		ridge: Overhang,
		eave: Overhang,
	},
}

impl Default for EndCap {
	fn default() -> Self {
		Self::Hip
	}
}

/// Authored parameters for a [`RectangularPitchedRoofComplex`].
#[derive(Debug, Clone, PartialEq)]
pub struct RectangularPitchedRoofComplexParams {
	pub volumes: Vec<Aabb3d>,
	pub overhang: Overhang,
	pub end_cap: EndCap,
	pub style: PanelStyle,
	pub joint_thickness: f32,
}

impl Default for RectangularPitchedRoofComplexParams {
	fn default() -> Self {
		Self::l_shape()
	}
}

impl RectangularPitchedRoofComplexParams {
	pub fn new(volumes: Vec<Aabb3d>) -> Self {
		Self {
			volumes,
			overhang: Overhang::default(),
			end_cap: EndCap::default(),
			style: PanelStyle::ShepherdsThatch,
			joint_thickness: DEFAULT_PANEL_THICKNESS,
		}
	}

	/// Single massing box centered on X, long along X.
	pub fn single(extent_x: f32, extent_z: f32, y0: f32, y1: f32) -> Self {
		let hx = extent_x * 0.5;
		let hz = extent_z * 0.5;
		Self::new(vec![Aabb3d::from_min_max(
			Vec3::new(-hx, y0, -hz),
			Vec3::new(hx, y1, hz),
		)])
	}

	/// L: long-X bar + long-Z stem sharing the −X/−Z corner.
	pub fn l_shape() -> Self {
		// A: x[-2, 8] × z[-2, 2]  (long X)
		// B: x[-2, 2] × z[-2, 8]  (long Z)
		Self::new(vec![
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -2.0), Vec3::new(8.0, 4.5, 2.0)),
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -2.0), Vec3::new(2.0, 4.5, 8.0)),
		])
	}

	/// T: long-X bar + long-Z stem centered on the bar.
	pub fn t_shape() -> Self {
		Self::new(vec![
			Aabb3d::from_min_max(Vec3::new(-8.0, 2.5, -2.0), Vec3::new(8.0, 4.5, 2.0)),
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -2.0), Vec3::new(2.0, 4.5, 8.0)),
		])
	}

	pub fn overhang(mut self, overhang: Overhang) -> Self {
		self.overhang = overhang;
		self
	}

	pub fn end_cap(mut self, end_cap: EndCap) -> Self {
		self.end_cap = end_cap;
		self
	}

	pub fn style(mut self, style: PanelStyle) -> Self {
		self.style = style;
		self
	}

	pub fn build(self) -> RectangularPitchedRoofComplex {
		RectangularPitchedRoofComplex::new(self)
	}
}

/// Resolved orthogonal roof complex.
#[derive(Debug, Clone, PartialEq)]
pub struct RectangularPitchedRoofComplex {
	params: RectangularPitchedRoofComplexParams,
	roofs: Vec<PitchedRoof>,
	valleys: Vec<ValleySegment>,
}

impl RectangularPitchedRoofComplex {
	pub fn new(params: RectangularPitchedRoofComplexParams) -> Self {
		let (roofs, valleys) = resolve(&params);
		Self {
			params,
			roofs,
			valleys,
		}
	}

	pub fn params(&self) -> &RectangularPitchedRoofComplexParams {
		&self.params
	}

	pub fn roofs(&self) -> &[PitchedRoof] {
		&self.roofs
	}

	pub fn valleys(&self) -> &[ValleySegment] {
		&self.valleys
	}
}

impl BuildingComponents for RectangularPitchedRoofComplex {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for roof in &self.roofs {
			out.extend(roof.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		for roof in &self.roofs {
			out.extend(roof.joint_nodes_for_level(level));
		}
		out
	}
}

fn resolve(params: &RectangularPitchedRoofComplexParams) -> (Vec<PitchedRoof>, Vec<ValleySegment>) {
	if params.volumes.is_empty() {
		return (Vec::new(), Vec::new());
	}

	let mut volumes: Vec<VolumeCandidate> = params
		.volumes
		.iter()
		.copied()
		.map(|aabb| VolumeCandidate::from_aabb(aabb, params.overhang))
		.collect();

	let corners = resolve_junctions(&mut volumes);
	let valleys = apply_valleys(&mut volumes, &corners);

	for vol in &mut volumes {
		vol.apply_end_caps(params.end_cap);
	}

	let roofs = volumes
		.iter()
		.map(|vol| emit_roof(vol, params))
		.collect();

	(roofs, valleys)
}

fn emit_roof(vol: &VolumeCandidate, params: &RectangularPitchedRoofComplexParams) -> PitchedRoof {
	let hip = matches!(params.end_cap, EndCap::Hip);
	let gable = matches!(params.end_cap, EndCap::Gable { .. });
	let free = vol.end_free;

	let halves = [0usize, 1].map(|side| {
		let mut half = RoofHalf::new(
			vol.ridge.as_tuple(),
			vol.eave[side].as_tuple(),
			vol.wall[side].as_tuple(),
		)
		.draw_in_wall_line(true);
		if hip {
			half = half.draw_in_half_hip((free[0], free[1]));
		}
		if gable {
			half = half.draw_in_half_gable_end((free[0], free[1]));
		}
		half
	});

	PitchedRoofParams::new(halves)
		.style(params.style)
		.joint_thickness(params.joint_thickness)
		.build()
}
