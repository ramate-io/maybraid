//! **Tuft Patch** — a few blade tufts scattered over a small ground area.
//!
//! Unlike the single-anchor tufts, which radiate every blade from one point, a tuft patch
//! deterministically picks a few anchor points within an XZ footprint and grows a blade tuft
//! at each, reading as one loose clump of grass rather than a fountain.

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::tuft::{BladeTuft, BladeTuftShape};
use clap::Args;
use procedural_common::{NoiseConfig, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::SkippedLeafMeshMaterial;

/// Typical [`StandardMaterial`] Tuft Patch using CLI-skipped leaf handles.
pub type TuftPatchStd = TuftPatch<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;

/// A patch of blade tufts scattered over an XZ footprint.
///
/// Anchor points and per-clump shape variation both derive deterministically from
/// [`BladeTuftShape::seed`]; placement structures vary instances by changing the seed.
#[derive(Component, Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct TuftPatch<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	/// Number of tuft clumps scattered over the patch.
	#[arg(long, default_value_t = 5)]
	pub clump_count: u32,

	/// Square patch footprint side length (m) the clumps scatter within.
	#[arg(long, default_value_t = 1.5)]
	pub patch_extent_xz: f32,

	#[command(flatten, next_help_heading = "Blade Tuft")]
	pub shape: BladeTuftShape,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	#[arg(skip)]
	__marker: PhantomData<fn() -> LeafM>,
}

impl<LeafM, LeafS> Default for TuftPatch<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Default,
{
	fn default() -> Self {
		Self {
			clump_count: 5,
			patch_extent_xz: 1.5,
			shape: BladeTuftShape::default(),
			leaf_material: LeafS::default(),
			__marker: PhantomData,
		}
	}
}

impl<LeafM, LeafS> TuftPatch<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	pub fn new(
		clump_count: u32,
		patch_extent_xz: f32,
		shape: BladeTuftShape,
		leaf_material: LeafS,
	) -> Self {
		Self { clump_count, patch_extent_xz, shape, leaf_material, __marker: PhantomData }
	}

	/// Deterministic patch-local clump anchors, scattered within the XZ footprint.
	pub fn clump_anchors(&self) -> Vec<Vec3> {
		let config = NoiseConfig::new(NoiseParams::from_scalar(self.shape.seed as f32, 1.0, 1.0, 1));
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
	fn clump_shape(&self, index: u32) -> BladeTuftShape {
		BladeTuftShape {
			seed: self.shape.seed.wrapping_add((index as i32 + 1) * 131),
			..self.shape.clone()
		}
	}
}

impl<LeafM, LeafS> RenderItem for TuftPatch<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let root = commands
			.spawn((self.clone(), cascade_chunk.clone(), transform, Visibility::default()))
			.id();

		for (index, anchor) in self.clump_anchors().into_iter().enumerate() {
			let tuft =
				BladeTuft::from_shape(self.clump_shape(index as u32), self.leaf_material.clone());
			tuft.spawn_render_items_under(
				commands,
				cascade_chunk,
				Transform::from_translation(anchor),
				Some(root),
			);
		}

		vec![root]
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	fn patch(seed: i32) -> TuftPatchStd {
		TuftPatchStd {
			shape: BladeTuftShape { seed, ..BladeTuftShape::default() },
			..TuftPatchStd::default()
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
}
