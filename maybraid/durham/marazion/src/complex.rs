//! Graph of watershed depressions with a **shared** outer apron.
//!
//! [RFC-127 §3.1.3.4](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#3134-pocket-complex)
//! anticipates multi-part pocket complexes; v1 realizes standalone stream / lake
//! leaves as single-edge / single-node graphs compiled through this type.

use crate::depression::WatershedDepression;
use crate::fill::WaterFill;
use bevy_math::Vec2;
use jersey_terrain_stamps::{
	JerseyModulation, Region2D, RegionAffineModulation, RegionNoise,
	RegionPolylineGradingModulation,
};
use procedural_common::Bounds2;

/// Stable index into [`WatershedDepressionComplex::nodes`].
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

/// Shared outer shelf applied once around the complex (behavior-parity modes).
#[derive(Debug, Clone)]
pub enum WatershedApronShelf {
	/// Lake plateau: flatten to `rim_level` with apron fade past the plateau region.
	LakeFlatten {
		region: Region2D,
		rim_level: f32,
		outer_radius: f32,
		apron_noise: RegionNoise,
		rim_height: RegionNoise,
	},
	/// Stream skirt: raise-only grade to bank levels across the apron band.
	StreamRaiseOnly {
		region: Region2D,
		path: Vec<Vec2>,
		bank_levels: Vec<f32>,
		node_blend: f32,
		fade: f32,
		apron_noise: RegionNoise,
		rim_height: RegionNoise,
	},
}

impl WatershedApronShelf {
	fn into_modulation(self) -> JerseyModulation {
		match self {
			Self::LakeFlatten {
				region,
				rim_level,
				outer_radius,
				apron_noise,
				rim_height,
			} => JerseyModulation::Affine(
				RegionAffineModulation::new(region, 0.0, rim_level, 0.0, outer_radius)
					.with_noise(apron_noise)
					.with_height_noise_add_only(rim_height),
			),
			Self::StreamRaiseOnly {
				region,
				path,
				bank_levels,
				node_blend,
				fade,
				apron_noise,
				rim_height,
			} => JerseyModulation::PolylineGrading(
				RegionPolylineGradingModulation::new(region, path, bank_levels, 0.0, fade)
					.with_node_blend(node_blend)
					.with_noise(apron_noise)
					.with_height_noise_add_only(rim_height)
					.raise_only(),
			),
		}
	}
}

/// Compiled stamp products from a watershed depression complex.
#[derive(Debug, Clone)]
pub struct CompiledWatershed {
	pub bounds: Bounds2,
	pub seed: u32,
	pub modulations: Vec<JerseyModulation>,
	pub fills: Vec<WaterFill>,
	/// Union of all wet-core footprints (for future multi-part aprons).
	pub wet_union: Option<Region2D>,
}

impl CompiledWatershed {
	pub fn is_empty(&self) -> bool {
		self.modulations.is_empty()
	}
}

/// Graph of depressions with one shared apron shelf.
#[derive(Debug, Clone)]
pub struct WatershedDepressionComplex {
	pub bounds: Bounds2,
	pub seed: u32,
	pub nodes: Vec<WatershedNode>,
	pub edges: Vec<WatershedEdge>,
	pub apron: Option<WatershedApronShelf>,
}

impl WatershedDepressionComplex {
	pub fn new(bounds: Bounds2, seed: u32) -> Self {
		Self {
			bounds,
			seed,
			nodes: Vec::new(),
			edges: Vec::new(),
			apron: None,
		}
	}

	pub fn with_apron(mut self, apron: WatershedApronShelf) -> Self {
		self.apron = Some(apron);
		self
	}

	pub fn push_node(&mut self, node: WatershedNode) -> WatershedNodeId {
		let id = self.nodes.len();
		self.nodes.push(node);
		id
	}

	pub fn push_edge(&mut self, edge: WatershedEdge) -> WatershedEdgeId {
		let id = self.edges.len();
		self.edges.push(edge);
		id
	}

	/// Single-node lake complex (no edges).
	pub fn from_lake_node(
		bounds: Bounds2,
		seed: u32,
		depression: WatershedDepression,
		apron: WatershedApronShelf,
	) -> Self {
		let mut complex = Self::new(bounds, seed);
		complex.push_node(WatershedNode::with_depression(depression));
		complex.apron = Some(apron);
		complex
	}

	/// Single-edge stream complex with empty endpoint nodes.
	pub fn from_stream_edge(
		bounds: Bounds2,
		seed: u32,
		depression: WatershedDepression,
		apron: WatershedApronShelf,
	) -> Self {
		let mut complex = Self::new(bounds, seed);
		let from = complex.push_node(WatershedNode::empty());
		let to = complex.push_node(WatershedNode::empty());
		complex.push_edge(WatershedEdge {
			from,
			to,
			depression,
		});
		complex.apron = Some(apron);
		complex
	}

	/// Compile shared apron + wet-core carves/fills.
	///
	/// Order: apron shelf first, then node depressions, then edge depressions
	/// (matches lake plateau→bowl and stream skirt→channel→cuts).
	pub fn compile(self) -> CompiledWatershed {
		let mut wet_cores: Vec<Region2D> = Vec::new();
		let mut carve: Vec<JerseyModulation> = Vec::new();
		let mut fills: Vec<WaterFill> = Vec::new();

		for node in &self.nodes {
			if let Some(dep) = &node.depression {
				wet_cores.push(dep.wet_core.clone());
				carve.extend(dep.carve_modulations.iter().cloned());
				if let Some(fill) = &dep.fill {
					fills.push(fill.clone());
				}
			}
		}
		for edge in &self.edges {
			wet_cores.push(edge.depression.wet_core.clone());
			carve.extend(edge.depression.carve_modulations.iter().cloned());
			if let Some(fill) = &edge.depression.fill {
				fills.push(fill.clone());
			}
		}

		let mut modulations = Vec::new();
		if let Some(apron) = self.apron {
			modulations.push(apron.into_modulation());
		}
		modulations.extend(carve);

		let wet_union = match wet_cores.len() {
			0 => None,
			1 => wet_cores.pop(),
			_ => Some(Region2D::union(wet_cores)),
		};

		CompiledWatershed {
			bounds: self.bounds,
			seed: self.seed,
			modulations,
			fills,
			wet_union,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::depression::{WatershedDepression, WatershedDepressionKind};
	use jersey_terrain_stamps::{CircleRegion, Region2D};

	#[test]
	fn empty_complex_compiles_empty() -> anyhow::Result<()> {
		let c = WatershedDepressionComplex::new(Bounds2::from_xz(0.0, 0.0, 10.0, 10.0), 1);
		let out = c.compile();
		assert!(out.is_empty());
		assert!(out.fills.is_empty());
		assert!(out.wet_union.is_none());
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
			vec![],
			None,
		)));
		complex.push_node(WatershedNode::with_depression(WatershedDepression::new(
			WatershedDepressionKind::LakeBowl,
			b.clone(),
			vec![],
			None,
		)));
		let out = complex.compile();
		let u = out.wet_union.expect("union");
		let p = Vec2::new(-10.0, 0.0);
		assert!((u.sdf(p) - a.sdf(p).min(b.sdf(p))).abs() < 1e-5);
		Ok(())
	}
}
