//! Indexed hydrology complex: member nodes + sample-time correction.
//!
//! Authored leaves emit [`crate::node::HydrologyNode`]s. [`HydrologyComplex`]
//! indexes them and applies carve → rim → apron. Optional backfills stay a
//! post-depression jersey layer via [`Self::compile`].

use crate::backfill::WatershedBackfill;
use crate::depression::WatershedDepression;
use crate::fill::{WaterFill, WaterSurface};
use crate::hydro::{
	smoothmin_fold, ComplexApronParams, CorrectionStage, FootprintIndex, HydroFold,
	HydroPrimitive, SURFACE_SMOOTHMIN_K,
};
use crate::node::{HydrologyNode, HydroParameters};
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
		Self {
			depression: Some(depression),
		}
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
			shore_fade: apron.shore_fade,
			fill_undercut: apron.fill_undercut,
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

	/// Fold \(\phi\), bed, \(W\), and blended rim/apron policy.
	pub fn fold_fields(&self, p: Vec2, use_index: bool) -> Option<HydroFold> {
		let ids: Vec<u16> = if use_index {
			self.candidate_ids(p)
		} else {
			self.index.all_ids(self.hydrology.len())
		};
		if ids.is_empty() {
			return None;
		}
		let mut phi = f32::INFINITY;
		let mut bed = f32::INFINITY;
		let mut surfaces: Vec<f32> = Vec::new();
		let mut rim_w = 0.25_f32;
		let mut apron_w = 0.25_f32;
		let mut bank = f32::NEG_INFINITY;
		let mut shore_fade = self.shore_fade;
		for &id in &ids {
			let Some(node) = self.hydrology.get(id as usize) else {
				continue;
			};
			let pad = node.index_pad();
			let d = node.primitive.phi(p);
			if d > pad {
				continue;
			}
			phi = phi.min(d);
			let (w, b) = node.primitive.surface_and_bed(p);
			rim_w = rim_w.max(node.parameters.rim_width.max(0.25));
			apron_w = apron_w.max(node.parameters.apron_width.max(0.25));
			shore_fade = shore_fade.max(node.parameters.shore_fade.max(0.25));
			let node_bank = node.parameters.bank_target(w, p);
			if d <= 0.0 {
				bed = bed.min(b);
				surfaces.push(w);
				bank = bank.max(node_bank);
			} else if d < (rim_w + apron_w).max(1.0) {
				surfaces.push(w);
				bank = bank.max(node_bank);
			}
		}
		if !phi.is_finite() {
			return None;
		}
		let water = if surfaces.is_empty() {
			bed
		} else {
			smoothmin_fold(&surfaces, SURFACE_SMOOTHMIN_K)
		};
		let bed = if bed.is_finite() { bed } else { water };
		let bank = if bank.is_finite() {
			bank
		} else {
			water + 1.1
		};
		Some(HydroFold {
			phi,
			bed,
			water,
			rim_width: rim_w,
			apron_width: apron_w,
			bank,
			shore_fade,
		})
	}

	pub fn carve_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let p = Vec2::new(x, z);
		if !self.bounds.contains(p) {
			return elevation;
		}
		let Some(fold) = self.fold_fields(p, true) else {
			return elevation;
		};
		if fold.phi <= 0.0 {
			elevation.min(fold.bed)
		} else {
			elevation
		}
	}

	pub fn rim_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let p = Vec2::new(x, z);
		if !self.bounds.contains(p) {
			return elevation;
		}
		let Some(fold) = self.fold_fields(p, true) else {
			return elevation;
		};
		if fold.phi <= 0.0 || fold.phi >= fold.rim_width {
			return elevation;
		}
		let t = (1.0 - fold.phi / fold.rim_width).clamp(0.0, 1.0);
		let toward = elevation * (1.0 - t) + fold.bank * t;
		toward.max(elevation)
	}

	pub fn apron_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let p = Vec2::new(x, z);
		if !self.bounds.contains(p) {
			return elevation;
		}
		let Some(fold) = self.fold_fields(p, true) else {
			return elevation;
		};
		let rim_w = fold.rim_width;
		let apron_w = fold.apron_width;
		if fold.phi < rim_w || fold.phi >= rim_w + apron_w {
			return elevation;
		}
		let u = ((fold.phi - rim_w) / apron_w).clamp(0.0, 1.0);
		let fade = u * u * (3.0 - 2.0 * u);
		let toward = elevation * fade + fold.bank * (1.0 - fade);
		toward.max(elevation)
	}

	pub fn apply_stage(&self, stage: CorrectionStage, elevation: f32, x: f32, z: f32) -> f32 {
		match stage {
			CorrectionStage::Carve => self.carve_elevation(elevation, x, z),
			CorrectionStage::Rim => self.rim_elevation(elevation, x, z),
			CorrectionStage::Apron => self.apron_elevation(elevation, x, z),
		}
	}

	/// Full carve → rim → apron.
	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let h = self.carve_elevation(elevation, x, z);
		let h = self.rim_elevation(h, x, z);
		self.apron_elevation(h, x, z)
	}

	pub fn surface_at(&self, x: f32, z: f32) -> Option<f32> {
		self.fold_fields(Vec2::new(x, z), true).map(|f| f.water)
	}

	pub fn occupancy_at(&self, x: f32, z: f32) -> Option<f32> {
		self.fold_fields(Vec2::new(x, z), true).map(|f| f.phi)
	}

	pub fn fill_softmask_at(&self, x: f32, z: f32) -> f32 {
		let fade = self.shore_fade.max(0.25);
		match self.occupancy_at(x, z) {
			None => 1.0,
			Some(phi) if phi <= 0.0 => 0.0,
			Some(phi) if phi >= fade => 1.0,
			Some(phi) => {
				let t = (phi / fade).clamp(0.0, 1.0);
				t * t * (3.0 - 2.0 * t)
			}
		}
	}

	/// Build a [`WaterFill`] that samples \(W\) / softmask from this complex.
	pub fn water_fill(&self) -> WaterFill {
		let center = self.bounds.center();
		let radius = self.bounds.extent().max_element() * 0.75;
		WaterFill {
			region: Region2D::Circle(CircleRegion { center, radius }),
			inner_radius: 0.0,
			outer_radius: self.shore_fade.max(0.25),
			noise: None,
			surface: WaterSurface::Hydro {
				complex: self.clone(),
			},
			terrain_undercut: self.fill_undercut.max(0.0),
		}
	}

	pub fn compile(&self) -> CompiledWatershed {
		let wet_union = self.wet_union_from_graph();
		let modulations: Vec<_> = self
			.backfills
			.iter()
			.cloned()
			.map(|b| b.into_modulation())
			.collect();
		let fills = if self.is_empty() {
			Vec::new()
		} else {
			vec![self.water_fill()]
		};
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
	use crate::node::{HydrologyNode, HydroParameters};
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
		let core = Region2D::Circle(CircleRegion {
			center: Vec2::ZERO,
			radius: 10.0,
		});
		let node = HydrologyNode::new(
			HydroPrimitive {
				footprint: HydroFootprint::Ellipse {
					center: Vec2::ZERO,
					radii: Vec2::splat(8.0),
					rotation: 0.0,
				},
				elevation: HydroElevation::RadialBowl {
					surface: 40.0,
					center_depth: 3.0,
				},
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
		let a = Region2D::Circle(CircleRegion {
			center: Vec2::new(-10.0, 0.0),
			radius: 4.0,
		});
		let b = Region2D::Circle(CircleRegion {
			center: Vec2::new(10.0, 0.0),
			radius: 4.0,
		});
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
}
