//! Indexed hydrology complex: member nodes + sample-time correction.
//!
//! Authored leaves emit [`crate::node::HydrologyNode`]s. [`HydrologyComplex`]
//! indexes them and blends terrain via
//! [`HydrologyNode::blend_terrain_elevation`]. Optional backfills stay a
//! post-depression jersey layer via [`Self::compile`].

use crate::backfill::WatershedBackfill;
use crate::depression::WatershedDepression;
use crate::fill::{WaterFill, WaterSurface};
use crate::hydro::{
	smoothmin_fold, ComplexApronParams, CorrectionStage, FootprintIndex, HydroPrimitive,
	SURFACE_SMOOTHMIN_K,
};
use crate::node::{HydroParameters, HydrologyNode};
use bevy_math::Vec2;
use jersey_terrain_stamps::{CircleRegion, JerseyModulation, Region2D};
use procedural_common::Bounds2;

/// Stable index into [`HydrologyComplex::graph_nodes`].
pub type WatershedNodeId = usize;
/// Stable index into [`HydrologyComplex::edges`].
pub type WatershedEdgeId = usize;

/// Graph node: empty hub, joint polyline, or lake bowl depression.
#[derive(Debug, Clone)]
pub struct WatershedNode {
	pub depression: Option<WatershedDepression>,
}

impl WatershedNode {
	pub fn empty() -> Self {
		Self { depression: None }
	}

	pub fn with_depression(depression: WatershedDepression) -> Self {
		Self { depression: Some(depression) }
	}
}

/// Graph edge: typically a stream-corridor depression between two nodes.
#[derive(Debug, Clone)]
pub struct WatershedEdge {
	pub from: WatershedNodeId,
	pub to: WatershedNodeId,
	pub depression: WatershedDepression,
}

/// Stamp products derived from a [`HydrologyComplex`] (fills + optional backfills).
#[derive(Debug, Clone)]
pub struct CompiledWatershed {
	pub bounds: Bounds2,
	pub seed: u32,
	/// Indexed complex used for elevation / fill sampling.
	pub complex: HydrologyComplex,
	/// Post-depression jersey ops (backfills).
	pub modulations: Vec<JerseyModulation>,
	pub fills: Vec<WaterFill>,
	pub wet_union: Option<Region2D>,
}

impl CompiledWatershed {
	pub fn is_empty(&self) -> bool {
		self.complex.is_empty() && self.modulations.is_empty()
	}

	pub fn has_hydro(&self) -> bool {
		!self.complex.is_empty()
	}

	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let mut h = self.complex.modify_elevation(elevation, x, z);
		for m in &self.modulations {
			h = m.modify_elevation(h, x, z);
		}
		h
	}
}

/// Captures and indexes hydrology nodes; owns carve → rim → apron modulation.
#[derive(Debug, Clone)]
pub struct HydrologyComplex {
	pub bounds: Bounds2,
	pub seed: u32,
	pub graph_nodes: Vec<WatershedNode>,
	pub edges: Vec<WatershedEdge>,
	pub backfills: Vec<WatershedBackfill>,
	/// Member hydrology nodes (source of truth for correction).
	pub hydrology: Vec<HydrologyNode>,
	/// Broadphase over [`Self::hydrology`] (rebuilt when nodes change).
	pub index: FootprintIndex,
	pub shore_fade: f32,
	pub fill_undercut: f32,
	/// Max liberal fill overhang beyond \(\phi = 0\) (from member nodes).
	pub fill_support_pad: f32,
}

impl HydrologyComplex {
	pub fn new(bounds: Bounds2, seed: u32) -> Self {
		let mut complex = Self {
			bounds,
			seed,
			graph_nodes: Vec::new(),
			edges: Vec::new(),
			backfills: Vec::new(),
			hydrology: Vec::new(),
			index: FootprintIndex::empty(),
			shore_fade: 2.5,
			fill_undercut: 0.0,
			fill_support_pad: 0.0,
		};
		complex.reindex();
		complex
	}

	/// Test helper: wrap bare primitives with shared apron parameters.
	pub fn from_primitives(
		bounds: Bounds2,
		seed: u32,
		primitives: Vec<HydroPrimitive>,
		apron: ComplexApronParams,
	) -> Self {
		let params = HydroParameters {
			shelf_anchor: None,
			rim_lift: apron.rim_lift,
			rim_width: apron.rim_width,
			apron_width: apron.apron_width,
			rim_height: apron.rim_height,
			rim_uplift_cap: apron.rim_uplift_cap,
			boundary_noise: None,
			shore_fade: apron.shore_fade,
			fill_undercut: apron.fill_undercut,
			fill_support_pad: 0.0,
		};
		let extent = params.correction_pad();
		let hydrology = primitives
			.into_iter()
			.map(|primitive| HydrologyNode::new(primitive, params.clone(), extent))
			.collect();
		Self::new(bounds, seed).with_hydrology(hydrology)
	}

	pub fn with_hydrology(mut self, nodes: Vec<HydrologyNode>) -> Self {
		self.hydrology = nodes;
		self.reindex();
		self
	}

	pub fn with_backfill(mut self, backfill: WatershedBackfill) -> Self {
		self.backfills.push(backfill);
		self
	}

	pub fn push_node(&mut self, node: WatershedNode) -> WatershedNodeId {
		let id = self.graph_nodes.len();
		self.graph_nodes.push(node);
		id
	}

	pub fn push_edge(&mut self, edge: WatershedEdge) -> WatershedEdgeId {
		let id = self.edges.len();
		self.edges.push(edge);
		id
	}

	pub fn is_empty(&self) -> bool {
		self.hydrology.is_empty()
	}

	fn reindex(&mut self) {
		let short = self.bounds.extent().min_element().max(1.0);
		let cell = (short * 0.08).clamp(8.0, 64.0);
		self.index = FootprintIndex::build_nodes(self.bounds, &self.hydrology, cell);
		self.shore_fade = self
			.hydrology
			.iter()
			.map(|m| m.parameters.shore_fade)
			.fold(2.5_f32, f32::max)
			.max(0.25);
		self.fill_undercut = self
			.hydrology
			.iter()
			.map(|m| m.parameters.fill_undercut)
			.fold(0.0_f32, f32::max);
		self.fill_support_pad = self
			.hydrology
			.iter()
			.map(|m| m.parameters.fill_support_pad)
			.fold(0.0_f32, f32::max);
	}

	fn wet_union_from_graph(&self) -> Option<Region2D> {
		let mut wet_cores: Vec<Region2D> = Vec::new();
		for node in &self.graph_nodes {
			if let Some(dep) = &node.depression {
				wet_cores.push(dep.wet_core.clone());
			}
		}
		for edge in &self.edges {
			wet_cores.push(edge.depression.wet_core.clone());
		}
		match wet_cores.len() {
			0 => None,
			1 => wet_cores.pop(),
			_ => Some(Region2D::union(wet_cores)),
		}
	}

	pub fn candidate_ids(&self, p: Vec2) -> Vec<u16> {
		let raw = self.index.candidates(p);
		if raw.is_empty() {
			return Vec::new();
		}
		let mut ids = raw.to_vec();
		ids.sort_unstable();
		ids.dedup();
		ids
	}

	/// Nodes whose correction support contains `p` (index pad + SDF filter).
	pub fn nodes_intersecting(&self, p: Vec2) -> Vec<&HydrologyNode> {
		let mut out = Vec::new();
		for id in self.candidate_ids(p) {
			let Some(node) = self.hydrology.get(id as usize) else {
				continue;
			};
			if node.phi(p) > node.index_pad() {
				continue;
			}
			out.push(node);
		}
		out
	}

	/// Class-priority blend over intersecting nodes (carve → rim → apron).
	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let p = Vec2::new(x, z);
		if !self.bounds.contains(p) {
			return elevation;
		}
		let nodes = self.nodes_intersecting(p);
		HydrologyNode::blend_terrain_elevation(&nodes, elevation, p)
	}

	/// Soft-min free surface for the wet-column gate.
	///
	/// Contributors match terrain carve ownership: only **Carve** nodes (plus the
	/// liberal [`Self::fill_support_pad`] band when no carve is present). Neighbor
	/// **Rim** surfaces are excluded — otherwise a lower pitched reach can pull
	/// \(W\) below the local carved bed and dry the column.
	pub fn surface_at(&self, x: f32, z: f32) -> Option<f32> {
		let p = Vec2::new(x, z);
		let fill_core = self.fill_support_pad.max(0.0);
		let mut carve_surfaces = Vec::new();
		let mut support_surfaces = Vec::new();
		for node in self.nodes_intersecting(p) {
			let d = node.phi(p);
			if matches!(
				node.point_classification(p),
				Some(CorrectionStage::Carve)
			) {
				carve_surfaces.push(node.surface_level(p));
			} else if d <= fill_core {
				support_surfaces.push(node.surface_level(p));
			}
		}
		let surfaces = if !carve_surfaces.is_empty() {
			carve_surfaces
		} else {
			support_surfaces
		};
		if surfaces.is_empty() {
			None
		} else {
			Some(smoothmin_fold(&surfaces, SURFACE_SMOOTHMIN_K))
		}
	}

	/// Min occupancy \(\phi\) over intersecting nodes (for fill softmask).
	pub fn occupancy_at(&self, x: f32, z: f32) -> Option<f32> {
		let p = Vec2::new(x, z);
		let mut phi = f32::INFINITY;
		for node in self.nodes_intersecting(p) {
			phi = phi.min(node.phi(p));
		}
		phi.is_finite().then_some(phi)
	}

	/// Softmask with liberal overhang: wet while \(\phi \le\) [`Self::fill_support_pad`],
	/// then fade over [`Self::shore_fade`].
	pub fn fill_softmask_at(&self, x: f32, z: f32) -> f32 {
		let pad = self.fill_support_pad.max(0.0);
		let fade = self.shore_fade.max(0.25);
		match self.occupancy_at(x, z) {
			None => 1.0,
			Some(phi) if phi <= pad => 0.0,
			Some(phi) if phi >= pad + fade => 1.0,
			Some(phi) => {
				let t = ((phi - pad) / fade).clamp(0.0, 1.0);
				t * t * (3.0 - 2.0 * t)
			}
		}
	}

	/// Build a [`WaterFill`] that samples \(W\) / softmask from this complex.
	pub fn water_fill(&self) -> WaterFill {
		// Prefer authored wet union for probes; fall back to a cell-covering circle.
		let region = self.wet_union_from_graph().unwrap_or_else(|| {
			let center = self.bounds.center();
			let radius = self.bounds.extent().max_element() * 0.75;
			Region2D::Circle(CircleRegion { center, radius })
		});
		WaterFill {
			region,
			inner_radius: 0.0,
			outer_radius: (self.fill_support_pad + self.shore_fade).max(0.25),
			noise: None,
			surface: WaterSurface::Hydro {
				complex: self.clone(),
			},
			terrain_undercut: self.fill_undercut.max(0.0),
		}
	}

	pub fn compile(&self) -> CompiledWatershed {
		let wet_union = self.wet_union_from_graph();
		let modulations: Vec<_> =
			self.backfills.iter().cloned().map(|b| b.into_modulation()).collect();
		let fills = if self.is_empty() { Vec::new() } else { vec![self.water_fill()] };
		CompiledWatershed {
			bounds: self.bounds,
			seed: self.seed,
			complex: self.clone(),
			modulations,
			fills,
			wet_union,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::backfill::WatershedBackfill;
	use crate::depression::{WatershedDepression, WatershedDepressionKind};
	use crate::hydro::{HydroElevation, HydroFootprint, HydroPrimitive};
	use crate::node::{HydroParameters, HydrologyNode};
	use jersey_terrain_stamps::{CircleRegion, Region2D, RegionNoise};

	#[test]
	fn empty_complex_compiles_empty() -> anyhow::Result<()> {
		let c = HydrologyComplex::new(Bounds2::from_xz(0.0, 0.0, 10.0, 10.0), 1);
		let out = c.compile();
		assert!(out.is_empty());
		assert!(out.fills.is_empty());
		Ok(())
	}

	#[test]
	fn backfill_appends_after_hydro() -> anyhow::Result<()> {
		let core = Region2D::Circle(CircleRegion { center: Vec2::ZERO, radius: 10.0 });
		let node = HydrologyNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Ellipse {
					center: Vec2::ZERO,
					radii: Vec2::splat(8.0),
					rotation: 0.0,
				},
				elevation: HydroElevation::RadialBowl { surface: 40.0, center_depth: 3.0 },
				influence_pad: 4.0,
			},
			HydroParameters::default(),
			12.0,
		);
		let mut complex = HydrologyComplex::new(Bounds2::from_xz(-40.0, -40.0, 40.0, 40.0), 3);
		complex.push_node(WatershedNode::with_depression(WatershedDepression::new(
			WatershedDepressionKind::LakeBowl,
			core.clone(),
		)));
		let out = complex
			.with_hydrology(vec![node])
			.with_backfill(WatershedBackfill::basin(
				core,
				RegionNoise::from_seed(1, 0.05, 4.0),
				2.0,
			))
			.compile();
		assert!(out.has_hydro());
		assert_eq!(out.modulations.len(), 1);
		Ok(())
	}

	#[test]
	fn wet_union_of_two_cores_is_min_sdf() -> anyhow::Result<()> {
		let a = Region2D::Circle(CircleRegion { center: Vec2::new(-10.0, 0.0), radius: 4.0 });
		let b = Region2D::Circle(CircleRegion { center: Vec2::new(10.0, 0.0), radius: 4.0 });
		let mut complex = HydrologyComplex::new(Bounds2::from_xz(-40.0, -40.0, 40.0, 40.0), 2);
		complex.push_node(WatershedNode::with_depression(WatershedDepression::new(
			WatershedDepressionKind::LakeBowl,
			a.clone(),
		)));
		complex.push_node(WatershedNode::with_depression(WatershedDepression::new(
			WatershedDepressionKind::LakeBowl,
			b.clone(),
		)));
		let out = complex.compile();
		let u = out.wet_union.expect("union");
		let p = Vec2::new(-10.0, 0.0);
		assert!((u.sdf(p) - a.sdf(p).min(b.sdf(p))).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn pitched_neighbor_rim_does_not_dry_upstream_carve() -> anyhow::Result<()> {
		// Colinear reaches with a steep drop. Mid of the upstream segment sits in
		// the downstream rim band; fill W must not soft-min that lower surface.
		let half_w = 8.0;
		let rim_w = 25.0;
		let depth = 4.0;
		let params = HydroParameters {
			rim_width: rim_w,
			apron_width: 8.0,
			fill_support_pad: 0.0,
			shore_fade: 2.0,
			fill_undercut: 2.0,
			..HydroParameters::default()
		};
		let a = Vec2::new(0.0, 0.0);
		let b = Vec2::new(40.0, 0.0);
		let c = Vec2::new(80.0, 0.0);
		let upstream = HydrologyNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::ReachSegment {
					a,
					b,
					half_width: half_w,
				},
				elevation: HydroElevation::ReachProfile {
					surface_a: 100.0,
					surface_b: 80.0,
					center_depth: depth,
				},
				influence_pad: rim_w + 8.0,
			},
			params.clone(),
			rim_w + 8.0,
		);
		let downstream = HydrologyNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::ReachSegment {
					a: b,
					b: c,
					half_width: half_w,
				},
				elevation: HydroElevation::ReachProfile {
					surface_a: 80.0,
					surface_b: 60.0,
					center_depth: depth,
				},
				influence_pad: rim_w + 8.0,
			},
			params,
			rim_w + 8.0,
		);
		let complex = HydrologyComplex::new(Bounds2::from_xz(-20.0, -40.0, 100.0, 40.0), 1)
			.with_hydrology(vec![upstream, downstream]);
		let mid = Vec2::new(20.0, 0.0);
		let w = complex.surface_at(mid.x, mid.y).expect("surface");
		// Local upstream mid grade is 90; a rim soft-min would drag toward ~80.
		assert!(
			w > 85.0,
			"fill W should track upstream carve grade, got {w}"
		);
		let h = complex.modify_elevation(100.0, mid.x, mid.y);
		let fill = complex.water_fill();
		assert!(
			fill.wet_y_span_at(mid.x, mid.y, h).is_some(),
			"pitched mid-channel should stay wet: W={w} h={h}"
		);
		Ok(())
	}

	#[test]
	fn fill_softmask_honors_support_pad() -> anyhow::Result<()> {
		let mut params = HydroParameters::default();
		params.fill_support_pad = 6.0;
		params.shore_fade = 2.0;
		let node = HydrologyNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Ellipse {
					center: Vec2::ZERO,
					radii: Vec2::splat(10.0),
					rotation: 0.0,
				},
				elevation: HydroElevation::RadialBowl {
					surface: 40.0,
					center_depth: 3.0,
				},
				influence_pad: 20.0,
			},
			params,
			20.0,
		);
		let mut complex = HydrologyComplex::new(Bounds2::from_xz(-40.0, -40.0, 40.0, 40.0), 4);
		complex = complex.with_hydrology(vec![node]);
		// Just outside the bowl but inside the liberal pad → fully wet softmask.
		assert!(complex.fill_softmask_at(12.0, 0.0) < 0.05);
		// Outside pad + fade → dry.
		assert!(complex.fill_softmask_at(20.0, 0.0) > 0.95);
		// Surface remains defined in the pad band.
		assert!(complex.surface_at(12.0, 0.0).is_some());
		Ok(())
	}

	#[test]
	fn water_fill_probes_node_interior() -> anyhow::Result<()> {
		let node = HydrologyNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Ellipse {
					center: Vec2::new(30.0, -25.0),
					radii: Vec2::splat(8.0),
					rotation: 0.0,
				},
				elevation: HydroElevation::RadialBowl {
					surface: 40.0,
					center_depth: 3.0,
				},
				influence_pad: 12.0,
			},
			HydroParameters::default(),
			12.0,
		);
		let mut complex = HydrologyComplex::new(Bounds2::from_xz(-40.0, -40.0, 40.0, 40.0), 5);
		complex.push_node(WatershedNode::with_depression(WatershedDepression::new(
			WatershedDepressionKind::LakeBowl,
			Region2D::Circle(CircleRegion {
				center: Vec2::new(30.0, -25.0),
				radius: 8.0,
			}),
		)));
		let fill = complex.with_hydrology(vec![node]).water_fill();
		let probes = fill.wet_volume_probe_points();
		assert!(
			probes.iter().any(|p| (p.x - 30.0).abs() < 1e-3 && (p.y + 25.0).abs() < 1e-3),
			"expected lake-center probe, got {probes:?}"
		);
		Ok(())
	}
}
