//! **Palm Crown** — stacked frond rings as [`VegetationComponents`].
//!
//! High / Medium emit [`FrondCollection`] rachis runs (straight frond segments along the
//! droop/arch spine). Low / UltraLow drop the fronds and keep a single layered-ball proxy
//! fit to the High crown AABB.
//!
//! Legacy stacked [`FrondCrown`](chico_ball_components::FrondCrown) mesh spawn remains in
//! [`spawn`] for date / Waialea / bush trees still on RenderItem.

pub mod spawn;

pub use spawn::spawn_stacked_frond_crowns;

use bevy::prelude::*;
use chico_ball_components::frond::FrondCrownShape;
use chico_vegetation_components::{
	FoliageNode, FrondCollection, FrondRun, Layers, Placement, StickNode, VegetationComponents,
	VegetationStructuralLodProbe,
};
use clap::Args;
use lod::gen::LodSceneLevel;

/// Per-ring seed salt (shared with [`spawn::FROND_RING_SEED_SALT`]).
pub use spawn::FROND_RING_SEED_SALT;

/// Authoring / CLI parameters for a palm crown (standalone or stacked rings).
#[derive(Component, Clone, Args, Debug, PartialEq)]
#[command(rename_all = "kebab-case")]
pub struct PalmCrownParams {
	/// Number of stacked frond rings along +Y.
	#[arg(long, default_value_t = 3)]
	pub ring_count: u32,

	/// Vertical spacing (m) between successive ring anchors.
	#[arg(long, default_value_t = 0.14)]
	pub ring_spacing: f32,

	#[command(flatten, next_help_heading = "Frond Crown")]
	pub shape: FrondCrownShape,
}

impl Default for PalmCrownParams {
	fn default() -> Self {
		Self {
			ring_count: 3,
			ring_spacing: 0.14,
			shape: FrondCrownShape {
				// Slightly palmier than the single-ring mesh default.
				frond_count: 11,
				length: 1.6,
				width: 0.16,
				droop: 0.55,
				arch_lift: 0.28,
				twist: 0.55,
				leaflet_count: 16,
				spine_segments: 8,
				downward_tilt_radians: 0.55,
				outward_spread_radians: 1.4,
				emission_lift_radians: 0.32,
				..FrondCrownShape::default()
			},
		}
	}
}

impl PalmCrownParams {
	pub fn new(ring_count: u32, ring_spacing: f32, shape: FrondCrownShape) -> Self {
		Self { ring_count, ring_spacing, shape }
	}

	/// Tree-local ring anchors stacked along +Y from the origin.
	pub fn ring_anchors(&self) -> Vec<Vec3> {
		let n = self.ring_count.max(1);
		(0..n)
			.map(|i| Vec3::new(0.0, i as f32 * self.ring_spacing.max(0.0), 0.0))
			.collect()
	}

	/// Shape for ring `index`, seed-salted so stacked rings do not clone.
	pub fn ring_shape(&self, index: u32) -> FrondCrownShape {
		FrondCrownShape {
			seed: self.shape.seed.wrapping_add(index as i32 * FROND_RING_SEED_SALT),
			..self.shape.clone()
		}
	}

	/// Grow ring anchors once for presentation / LOD emission.
	pub fn build(&self) -> PalmCrown {
		PalmCrown::from_params(self)
	}
}

/// Built palm crown: params plus resolved ring anchors.
#[derive(Clone, Debug, PartialEq)]
pub struct PalmCrown {
	pub ring_count: u32,
	pub ring_spacing: f32,
	pub shape: FrondCrownShape,
	pub anchors: Vec<Vec3>,
}

impl PalmCrown {
	pub fn from_params(params: &PalmCrownParams) -> Self {
		Self {
			ring_count: params.ring_count,
			ring_spacing: params.ring_spacing,
			shape: params.shape.clone(),
			anchors: params.ring_anchors(),
		}
	}

	fn ring_shape(&self, index: u32) -> FrondCrownShape {
		FrondCrownShape {
			seed: self.shape.seed.wrapping_add(index as i32 * FROND_RING_SEED_SALT),
			..self.shape.clone()
		}
	}

	fn ring_node(&self, index: usize, anchor: Vec3) -> Option<FoliageNode> {
		let shape = self.ring_shape(index as u32);
		let runs: Vec<FrondRun> = shape
			.frond_runs_at(anchor)
			.into_iter()
			.filter_map(|run| {
				let placements: Vec<Placement> = run
					.into_iter()
					.filter_map(|seg| {
						Placement::frond_segment(seg.start, seg.direction, seg.length, seg.width)
					})
					.collect();
				(!placements.is_empty()).then(|| FrondRun::from_placements(placements))
			})
			.collect();
		if runs.is_empty() {
			return None;
		}
		Some(FoliageNode::frond_collection(
			FrondCollection::new(runs),
			Placement::IDENTITY,
		))
	}

	fn frond_nodes(&self) -> Vec<FoliageNode> {
		self.anchors
			.iter()
			.enumerate()
			.filter_map(|(index, anchor)| self.ring_node(index, *anchor))
			.collect()
	}

	/// AABB of High rachis polylines (origin + droop extents).
	fn crown_aabb(&self) -> (Vec3, Vec3) {
		let mut min = Vec3::splat(f32::INFINITY);
		let mut max = Vec3::splat(f32::NEG_INFINITY);
		let mut any = false;
		for (index, anchor) in self.anchors.iter().enumerate() {
			let shape = self.ring_shape(index as u32);
			for run in shape.frond_runs_at(*anchor) {
				for seg in run {
					let tip = seg.start + seg.direction * seg.length;
					let half_w = seg.width * 0.5;
					for p in [seg.start, tip] {
						min = min.min(p - Vec3::splat(half_w));
						max = max.max(p + Vec3::splat(half_w));
						any = true;
					}
				}
			}
		}
		if !any {
			let r = self.shape.length.max(0.5);
			return (Vec3::splat(-r), Vec3::new(r, r, r));
		}
		(min, max)
	}

	fn layered_proxy_ball(&self) -> FoliageNode {
		let (min, max) = self.crown_aabb();
		let center = (min + max) * 0.5;
		let half_extents = ((max - min) * 0.5).max(Vec3::splat(1e-4));
		FoliageNode::layered_ball(Placement::new(center, 0.0).with_scale(half_extents))
	}

	fn crown_center(&self) -> Vec3 {
		let (min, max) = self.crown_aabb();
		(min + max) * 0.5
	}

	fn structural_radius(&self) -> f32 {
		let (min, max) = self.crown_aabb();
		let half = (max - min) * 0.5;
		half.x.max(half.y).max(half.z).max(1e-3)
	}
}

impl VegetationComponents for PalmCrown {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::new()
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		match level {
			LodSceneLevel::High | LodSceneLevel::Medium => {
				Layers::from_free(self.frond_nodes())
			}
			// Structural UltraLow collapses to Low content; both drop fronds for a proxy ball.
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => {
				Layers::from_free(vec![self.layered_proxy_ball()])
			}
		}
	}

	fn structural_lod_probe(&self) -> Option<VegetationStructuralLodProbe> {
		Some(VegetationStructuralLodProbe::new(
			self.crown_center(),
			self.structural_radius(),
		))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::gen::LodSceneLevel;

	fn crown(seed: i32) -> PalmCrownParams {
		PalmCrownParams {
			shape: FrondCrownShape {
				seed,
				frond_count: 6,
				spine_segments: 4,
				..PalmCrownParams::default().shape
			},
			..PalmCrownParams::default()
		}
	}

	#[test]
	fn ring_anchors_stack_along_y() -> Result<()> {
		let params = PalmCrownParams {
			ring_count: 3,
			ring_spacing: 0.2,
			..crown(1)
		};
		let anchors = params.ring_anchors();
		assert_eq!(anchors.len(), 3);
		assert_eq!(anchors[0], Vec3::ZERO);
		assert_eq!(anchors[1], Vec3::new(0.0, 0.2, 0.0));
		assert_eq!(anchors[2], Vec3::new(0.0, 0.4, 0.0));
		Ok(())
	}

	#[test]
	fn high_emits_one_collection_per_ring() -> Result<()> {
		let built = crown(3).build();
		let nodes = built.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(nodes.len(), built.anchors.len());
		let collection = nodes[0].geometry.as_frond_collection().expect("collection");
		assert_eq!(collection.runs.len(), 6);
		assert_eq!(collection.runs[0].segments.len(), 4);
		Ok(())
	}

	#[test]
	fn medium_keeps_fronds_low_is_layered_ball_only() -> Result<()> {
		let built = crown(5).build();
		let medium = built.foliage_nodes_for_level(LodSceneLevel::Medium).flatten();
		assert!(!medium.is_empty());
		assert!(medium[0].geometry.as_frond_collection().is_some());

		let low = built.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
		assert_eq!(low.len(), 1);
		assert!(low[0].geometry.is_layered_ball());

		let ultra = built.foliage_nodes_for_level(LodSceneLevel::UltraLow).flatten();
		assert_eq!(ultra.len(), 1);
		assert!(ultra[0].geometry.is_layered_ball());
		Ok(())
	}

	#[test]
	fn structural_probe_is_present() -> Result<()> {
		let built = crown(0).build();
		assert!(built.structural_lod_probe().is_some());
		Ok(())
	}
}
