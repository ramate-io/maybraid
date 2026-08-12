//! **Palm Crown** — stacked frond rings as [`VegetationComponents`].
//!
//! High / Medium emit one [`FrondCollection`] per frond (rachis as a short straight-segment
//! run). Per-frond collections keep merge LOD on rachis-scale extents. Low / UltraLow drop
//! the fronds and keep two rotated layered-ball proxies fit to the High crown AABB.
//!
//! Legacy stacked [`FrondCrown`](chico_ball_components::FrondCrown) mesh spawn remains in
//! [`spawn`] for date / Waialea / bush trees still on RenderItem.

pub mod spawn;

pub use spawn::spawn_stacked_frond_crowns;

use bevy::prelude::*;
use chico_ball_components::frond::FrondCrownShape;
use chico_vegetation_components::{
	chico_leaf_material_ref, FoliageNode, FrondCollection, FrondRun, Layers, Placement, StickNode,
	VegetationComponents, StructuralLod, STRUCTURAL_HIGH_FACTOR, STRUCTURAL_LOW_FACTOR,
	STRUCTURAL_MEDIUM_FACTOR,
};
use clap::Args;
use lod::gen::LodSceneLevel;

/// Per-ring seed salt (shared with [`spawn::FROND_RING_SEED_SALT`]).
pub use spawn::FROND_RING_SEED_SALT;

/// Medium outer edge: default structural Medium × 3 (200% further out).
const PALM_CROWN_STRUCTURAL_MEDIUM_FACTOR: f32 = STRUCTURAL_MEDIUM_FACTOR * 3.0;
/// Keep Low beyond Medium so band ordering stays valid.
const PALM_CROWN_STRUCTURAL_LOW_FACTOR: f32 = STRUCTURAL_LOW_FACTOR * 3.0;

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
				// Palmier than the single-ring mesh default; sparse fronds + short rachis.
				frond_count: 5,
				length: 1.6,
				width: 0.16,
				droop: 0.55,
				arch_lift: 0.28,
				twist: 0.55,
				leaflet_count: 16,
				// Two chords per frond — enough for droop without dense rachis tessellation.
				spine_segments: 2,
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

	/// One [`FrondCollection`] per frond (single run).
	///
	/// Ring-wide collections make the LOD extent the crown diameter, so UltraLow merge
	/// collapses to an oversized chord; per-frond collections keep extent ≈ rachis length.
	fn frond_nodes(&self) -> Vec<FoliageNode> {
		let mut nodes = Vec::new();
		for (index, anchor) in self.anchors.iter().enumerate() {
			let shape = self.ring_shape(index as u32);
			for run in shape.frond_runs_at(*anchor) {
				let placements: Vec<Placement> = run
					.into_iter()
					.filter_map(|seg| {
						Placement::frond_segment(seg.start, seg.direction, seg.length, seg.width)
					})
					.collect();
				if placements.is_empty() {
					continue;
				}
				nodes.push(FoliageNode::frond_collection(
					FrondCollection::new([FrondRun::from_placements(placements)])
						.bake_bounds_from_runs(),
					Placement::IDENTITY,
				));
			}
		}
		nodes
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

	/// Two layered balls with rotated pose offsets for a denser Low silhouette.
	fn layered_proxy_balls(&self) -> Vec<FoliageNode> {
		let (min, max) = self.crown_aabb();
		let center = (min + max) * 0.5;
		let half_extents = ((max - min) * 0.5).max(Vec3::splat(1e-4));
		// Slightly under-full AABB so the pair densifies without blowing the silhouette.
		let scale = half_extents * 0.9;
		let offset = Vec3::new(half_extents.x * 0.12, half_extents.y * 0.04, 0.0);
		let yaw_b = std::f32::consts::FRAC_PI_2;
		let center_a = center + offset;
		let center_b = center + Quat::from_rotation_y(yaw_b) * offset;
		vec![
			FoliageNode::layered_ball(
				Placement::new(center_a, 0.0)
					.with_pitch(0.18)
					.with_roll(-0.22)
					.with_scale(scale),
			)
			.with_material(chico_leaf_material_ref()),
			FoliageNode::layered_ball(
				Placement::new(center_b, yaw_b)
					.with_pitch(-0.28)
					.with_roll(0.4)
					.with_scale(scale),
			)
			.with_material(chico_leaf_material_ref()),
		]
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
			// Structural UltraLow collapses to Low content; both drop fronds for proxy balls.
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => {
				Layers::from_free(self.layered_proxy_balls())
			}
		}
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(
			StructuralLod::new(self.crown_center(), self.structural_radius())
				.with_factors(
					STRUCTURAL_HIGH_FACTOR,
					PALM_CROWN_STRUCTURAL_MEDIUM_FACTOR,
					PALM_CROWN_STRUCTURAL_LOW_FACTOR,
				),
		)
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
	fn high_emits_one_collection_per_frond() -> Result<()> {
		let built = crown(3).build();
		let nodes = built.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		// 3 rings × 6 fronds (test shape).
		assert_eq!(nodes.len(), built.anchors.len() * 6);
		let collection = nodes[0].geometry.as_frond_collection().expect("collection");
		assert_eq!(collection.runs.len(), 1);
		assert_eq!(collection.runs[0].segments.len(), 4);
		let (_, extent) = collection.center_and_extent();
		// Per-frond extent stays on rachis scale, not the full crown diameter.
		assert!(extent < built.shape.length * 1.5);
		Ok(())
	}

	#[test]
	fn medium_keeps_fronds_low_is_two_layered_balls() -> Result<()> {
		let built = crown(5).build();
		let medium = built.foliage_nodes_for_level(LodSceneLevel::Medium).flatten();
		assert!(!medium.is_empty());
		assert!(medium[0].geometry.as_frond_collection().is_some());

		let low = built.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
		assert_eq!(low.len(), 2);
		assert!(low.iter().all(|n| n.geometry.is_layered_ball()));
		assert_ne!(low[0].placement.yaw, low[1].placement.yaw);

		let ultra = built.foliage_nodes_for_level(LodSceneLevel::UltraLow).flatten();
		assert_eq!(ultra.len(), 2);
		assert!(ultra.iter().all(|n| n.geometry.is_layered_ball()));
		Ok(())
	}

	#[test]
	fn structural_medium_band_is_extended() -> Result<()> {
		let built = crown(0).build();
		let probe = built.structural_lod().expect("probe");
		assert_eq!(probe.medium_factor, STRUCTURAL_MEDIUM_FACTOR * 3.0);
		assert!(probe.low_factor > probe.medium_factor);
		Ok(())
	}

	#[test]
	fn default_rachis_and_frond_counts_are_sparse() -> Result<()> {
		let shape = PalmCrownParams::default().shape;
		assert!(shape.frond_count <= 5);
		assert!(shape.spine_segments <= 2);
		Ok(())
	}
}
