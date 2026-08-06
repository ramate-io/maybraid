//! **Tuft Patch** — a few blade tufts scattered over a small ground area.
//!
//! Unlike the single-anchor tufts, which radiate every blade from one point, a tuft patch
//! deterministically picks a few anchor points within an XZ footprint and grows a blade tuft
//! at each, reading as one loose clump of grass rather than a fountain.
//!
//! [`TuftPatchParams::build`] grows clump anchors once into [`TuftPatch`], which implements
//! [`VegetationComponents`] via one [`FoliageNode`] with [`FrondCollection`] geometry per
//! clump (straight frond segments, solid-green in the playground).

use bevy::prelude::*;
use chico_ball_components::tuft::BladeTuftShape;
use chico_vegetation_components::{
	FoliageNode, FrondCollection, Layers, Placement, StickNode, VegetationComponents,
};
use clap::Args;
use lod::gen::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};

/// Authoring / CLI parameters for a tuft patch.
#[derive(Component, Clone, Args, Debug, PartialEq)]
#[command(rename_all = "kebab-case")]
pub struct TuftPatchParams {
	/// Number of tuft clumps scattered over the patch.
	#[arg(long, default_value_t = 5)]
	pub clump_count: u32,

	/// Square patch footprint side length (m) the clumps scatter within.
	#[arg(long, default_value_t = 1.5)]
	pub patch_extent_xz: f32,

	#[command(flatten, next_help_heading = "Blade Tuft")]
	pub shape: BladeTuftShape,
}

impl Default for TuftPatchParams {
	fn default() -> Self {
		Self {
			clump_count: 5,
			patch_extent_xz: 1.5,
			shape: BladeTuftShape::default(),
		}
	}
}

impl TuftPatchParams {
	pub fn new(clump_count: u32, patch_extent_xz: f32, shape: BladeTuftShape) -> Self {
		Self { clump_count, patch_extent_xz, shape }
	}

	/// Deterministic patch-local clump anchors, scattered within the XZ footprint.
	pub fn clump_anchors(&self) -> Vec<Vec3> {
		let config =
			NoiseConfig::new(NoiseParams::from_scalar(self.shape.seed as f32, 1.0, 1.0, 1));
		let half = (self.patch_extent_xz * 0.5).max(0.0);
		(0..self.clump_count)
			.map(|i| {
				// Non-integer lanes keep samples off gradient-noise lattice points.
				let lane = (i as f32 + 0.5) * 3.7;
				let x = config.sample_range_f32_4d(-half, half, lane, 0.0, 0.0, 1.0);
				let z = config.sample_range_f32_4d(-half, half, lane, 0.0, 0.0, 2.0);
				Vec3::new(x, 0.0, z)
			})
			.collect()
	}

	/// The authored shape re-seeded for clump `index`, so clumps differ in blade layout.
	pub fn clump_shape(&self, index: u32) -> BladeTuftShape {
		BladeTuftShape {
			seed: self.shape.seed.wrapping_add((index as i32 + 1) * 131),
			..self.shape.clone()
		}
	}

	/// Grow clump anchors once for presentation / LOD emission.
	pub fn build(&self) -> TuftPatch {
		TuftPatch::from_params(self)
	}
}

/// Built tuft patch: params plus resolved clump anchors.
#[derive(Clone, Debug, PartialEq)]
pub struct TuftPatch {
	pub clump_count: u32,
	pub patch_extent_xz: f32,
	pub shape: BladeTuftShape,
	pub anchors: Vec<Vec3>,
}

impl TuftPatch {
	pub fn from_params(params: &TuftPatchParams) -> Self {
		Self {
			clump_count: params.clump_count,
			patch_extent_xz: params.patch_extent_xz,
			shape: params.shape.clone(),
			anchors: params.clump_anchors(),
		}
	}

	fn clump_shape(&self, index: u32) -> BladeTuftShape {
		BladeTuftShape {
			seed: self.shape.seed.wrapping_add((index as i32 + 1) * 131),
			..self.shape.clone()
		}
	}

	fn clump_node(&self, index: usize, anchor: Vec3) -> Option<FoliageNode> {
		let shape = self.clump_shape(index as u32);
		// Chained frond segments per blade (`bend_segments` + sway noise → kinks).
		let placements: Vec<Placement> = shape
			.frond_segments_at(anchor)
			.into_iter()
			.filter_map(|seg| {
				Placement::frond_segment(seg.start, seg.direction, seg.length, seg.width)
			})
			.collect();
		if placements.is_empty() {
			return None;
		}
		Some(FoliageNode::frond_collection(
			FrondCollection::segments(placements),
			Placement::IDENTITY,
		))
	}

	fn foliage_nodes(&self) -> Vec<FoliageNode> {
		self.anchors
			.iter()
			.enumerate()
			.filter_map(|(index, anchor)| self.clump_node(index, *anchor))
			.collect()
	}
}

impl VegetationComponents for TuftPatch {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::new()
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		match level {
			// Collections keep an UltraLow marker themselves; structural UltraLow drops them.
			LodSceneLevel::UltraLow => Layers::new(),
			_ => Layers::from_free(self.foliage_nodes()),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::gen::LodSceneLevel;

	fn patch(seed: i32) -> TuftPatchParams {
		TuftPatchParams {
			shape: BladeTuftShape { seed, ..BladeTuftShape::default() },
			..TuftPatchParams::default()
		}
	}

	#[test]
	fn anchors_stay_within_patch_footprint() -> Result<()> {
		let patch = patch(7);
		let anchors = patch.clump_anchors();
		assert_eq!(anchors.len(), patch.clump_count as usize);
		let half = patch.patch_extent_xz * 0.5;
		for anchor in &anchors {
			assert!(anchor.x.abs() <= half);
			assert!(anchor.z.abs() <= half);
			assert_eq!(anchor.y, 0.0);
		}
		Ok(())
	}

	#[test]
	fn anchors_are_deterministic_per_seed_and_scattered() -> Result<()> {
		let anchors = patch(7).clump_anchors();
		assert_eq!(anchors, patch(7).clump_anchors());
		let distinct = anchors
			.iter()
			.enumerate()
			.all(|(i, a)| anchors.iter().skip(i + 1).all(|b| a.distance(*b) > 1e-4));
		assert!(distinct, "expected scattered anchors, got {anchors:?}");
		Ok(())
	}

	#[test]
	fn clump_shapes_vary_by_seed_only() -> Result<()> {
		let patch = patch(7);
		let a = patch.clump_shape(0);
		let b = patch.clump_shape(1);
		assert_ne!(a.seed, b.seed);
		assert_eq!(a.blade_count, b.blade_count);
		assert_eq!(a.blade_length, b.blade_length);
		Ok(())
	}

	#[test]
	fn build_emits_one_collection_node_per_clump() -> Result<()> {
		let params = TuftPatchParams {
			clump_count: 2,
			shape: BladeTuftShape {
				blade_count: 4,
				bend_segments: 2,
				seed: 3,
				..BladeTuftShape::default()
			},
			..TuftPatchParams::default()
		};
		let built = params.build();
		let nodes = built.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(nodes.len(), 2);
		let collection = nodes[0].geometry.as_frond_collection().expect("collection geom");
		// 4 blades × 2 bend segments.
		assert_eq!(collection.members.len(), 8);
		assert_eq!(collection.members_for_level(LodSceneLevel::Medium).len(), 4);
		assert_eq!(collection.members_for_level(LodSceneLevel::UltraLow).len(), 1);
		Ok(())
	}
}
