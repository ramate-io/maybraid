//! Graph of watershed depressions with shared hydro composition.
//!
//! Authored leaves emit [`crate::node::HydrologyNode`]s. [`Self::compile`]
//! prepares a [`crate::hydro::PreparedHydroComplex`] (member refs / owned nodes)
//! for staged carve → rim → apron correction. Optional backfills stay a
//! post-depression jersey layer.

use crate::backfill::WatershedBackfill;
use crate::depression::WatershedDepression;
use crate::fill::WaterFill;
use crate::hydro::{water_fill_from_prepared, PreparedHydroComplex};
use crate::node::HydrologyNode;
use jersey_terrain_stamps::{JerseyModulation, Region2D};
use procedural_common::Bounds2;

/// Stable index into [`WatershedDepressionComplex::graph_nodes`].
pub type WatershedNodeId = usize;
/// Stable index into [`WatershedDepressionComplex::edges`].
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

/// Compiled stamp products from a watershed depression complex.
#[derive(Debug, Clone)]
pub struct CompiledWatershed {
	pub bounds: Bounds2,
	pub seed: u32,
	/// Post-depression jersey ops (backfills).
	pub modulations: Vec<JerseyModulation>,
	/// Prepared hydro complex (member nodes + staged correction).
	pub hydro: Option<PreparedHydroComplex>,
	pub fills: Vec<WaterFill>,
	pub wet_union: Option<Region2D>,
}

impl CompiledWatershed {
	pub fn is_empty(&self) -> bool {
		self.modulations.is_empty()
			&& self
				.hydro
				.as_ref()
				.map_or(true, |h| h.is_empty())
	}

	pub fn modify_elevation(&self, elevation: f32, x: f32, z: f32) -> f32 {
		let mut h = elevation;
		if let Some(hydro) = &self.hydro {
			h = hydro.modify_elevation(h, x, z);
		}
		for m in &self.modulations {
			h = m.modify_elevation(h, x, z);
		}
		h
	}
}

/// Assembled watershed: graph bookkeeping + member [`HydrologyNode`]s.
#[derive(Debug, Clone)]
pub struct WatershedDepressionComplex {
	pub bounds: Bounds2,
	pub seed: u32,
	pub graph_nodes: Vec<WatershedNode>,
	pub edges: Vec<WatershedEdge>,
	pub backfills: Vec<WatershedBackfill>,
	/// Member hydrology nodes (source of truth for correction).
	pub hydrology: Vec<HydrologyNode>,
}

impl WatershedDepressionComplex {
	pub fn new(bounds: Bounds2, seed: u32) -> Self {
		Self {
			bounds,
			seed,
			graph_nodes: Vec::new(),
			edges: Vec::new(),
			backfills: Vec::new(),
			hydrology: Vec::new(),
		}
	}

	pub fn with_hydrology(mut self, nodes: Vec<HydrologyNode>) -> Self {
		self.hydrology = nodes;
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
		self.hydrology.is_empty() && self.backfills.is_empty()
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

	pub fn compile(&self) -> CompiledWatershed {
		let wet_union = self.wet_union_from_graph();
		let modulations: Vec<_> = self
			.backfills
			.iter()
			.cloned()
			.map(|b| b.into_modulation())
			.collect();

		if self.hydrology.is_empty() {
			return CompiledWatershed {
				bounds: self.bounds,
				seed: self.seed,
				modulations,
				hydro: None,
				fills: Vec::new(),
				wet_union,
			};
		}

		let prepared = PreparedHydroComplex::prepare(
			self.bounds,
			self.seed,
			self.hydrology.clone(),
		);
		let fill = water_fill_from_prepared(prepared.clone());
		CompiledWatershed {
			bounds: self.bounds,
			seed: self.seed,
			modulations,
			hydro: Some(prepared),
			fills: vec![fill],
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
	use bevy_math::Vec2;
	use jersey_terrain_stamps::{CircleRegion, Region2D, RegionNoise};

	#[test]
	fn empty_complex_compiles_empty() -> anyhow::Result<()> {
		let c = WatershedDepressionComplex::new(Bounds2::from_xz(0.0, 0.0, 10.0, 10.0), 1);
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
		let mut complex =
			WatershedDepressionComplex::new(Bounds2::from_xz(-40.0, -40.0, 40.0, 40.0), 3);
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
		assert!(out.hydro.is_some());
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
		let mut complex =
			WatershedDepressionComplex::new(Bounds2::from_xz(-40.0, -40.0, 40.0, 40.0), 2);
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
