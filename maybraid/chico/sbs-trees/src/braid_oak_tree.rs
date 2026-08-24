//! **Braid Oak Tree** — gnarled broadleaf with crook-cylinder branches ([#234](https://github.com/ramate-io/maybraid/issues/234), [RFC §3.1.7.13](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/13-braid-oak/README.md)).
//!
//! [`BraidOakTreeParams::build`] applies the braid preset, grows the ball-stick chain once into
//! [`BraidOakTree`], which implements [`VegetationComponents`].
//!
//! [`BraidOakTree::unit_from_num`] / [`BraidOakTreeParams::into_unit_from_num`] normalize to
//! unit height and key layout / crook noise by a variant index so many plants share one
//! archetypal mesh (world size goes on [`Placement`](chico_vegetation_components::Placement)
//! scale). Emission folds sticks and cheap balls into collections.
//!
//! Stick LOD: High crook-centerline polylines (3 samples / 2 segments per crook); Medium uses
//! denser banded straight sticks (+20% vs storybook) with a layered canopy proxy; Low is stalk-only.

#[allow(dead_code)]
mod canopy;
#[allow(dead_code)]
pub(crate) mod joint_ball;
pub mod render_item_plugin;
pub(crate) mod stick;

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, BraidOakTreeSbs, StorybookTreeChain};
use chico_vegetation_components::{
	chico_leaf_material_ref, chico_stick_material_ref, FoliageNode, Layers, StickNode,
	StructuralLod, VegetationComponents,
};
use clap::Args;
use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::storybook_tree::canopy::{
	foliage_nodes_banded, foliage_nodes_low, foliage_nodes_medium_with_proxy,
	BRAID_MEDIUM_STICK_BANDS, HIGH_FOLIAGE_BANDS,
};
use crate::storybook_tree::{merge_cheap_ball_foliage, merge_kit_sticks};
use crate::torch_tree::{stick_nodes_banded, stick_nodes_low, HIGH_STICK_BANDS};
use stick::stick_nodes_high_crook;

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 5.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 15.0;
const STRUCTURAL_LOW_FACTOR: f32 = 24.0;

/// Authoring / CLI parameters for Braid Oak Tree.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct BraidOakTreeParams {
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: BraidOakTreeSbs,

	/// Stick-surface noise driving crook bend strength (High stick polylines).
	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = procedural_common::noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Stick Surface Noise"
	)]
	pub stick_surface_noise: NoiseParams,
}

impl Default for BraidOakTreeParams {
	fn default() -> Self {
		Self {
			geometry: BraidOakTreeSbs::default(),
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
		}
	}
}

impl BraidOakTreeParams {
	/// Apply braid preset and grow the ball-stick chain once.
	pub fn build(&self) -> BraidOakTree {
		BraidOakTree::from_params(self)
	}

	/// Unit-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::default().into_unit_from_num(num).0
	}

	/// Normalize this params set to unit height keyed by `num`.
	///
	/// Applies the braid preset first so world size is the post-preset height.
	pub fn into_unit_from_num(self, num: u32) -> (Self, f32) {
		let mut geometry = self.geometry;
		geometry.apply_braid_preset();
		let size = geometry.height().max(1e-4);
		let inv = 1.0 / size;
		geometry.scale.tree_height = 1.0;
		if let Some(radius) = geometry.scale.stalk_base_radius {
			geometry.scale.stalk_base_radius = Some((radius * inv).max(1e-6));
		}
		geometry.canopy_noise.seed = num as i32;
		geometry.anchor_perturbation.noise.seed = num as i32;
		let mut stick_surface_noise = self.stick_surface_noise;
		stick_surface_noise.seed = num as i32;
		(Self { geometry, stick_surface_noise }, size)
	}
}

/// Built Braid Oak: braid-preset geometry plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct BraidOakTree {
	pub geometry: BraidOakTreeSbs,
	pub chain: BallStickChain<StorybookTreeChain>,
	pub stick_surface_noise: NoiseParams,
}

impl BraidOakTree {
	pub fn from_params(params: &BraidOakTreeParams) -> Self {
		let mut geometry = params.geometry.clone();
		geometry.apply_braid_preset();
		Self {
			chain: geometry.build_chain(),
			geometry,
			stick_surface_noise: params.stick_surface_noise,
		}
	}

	fn footprint_radius(&self) -> f32 {
		self.chain
			.footprint_radius_at_least(self.geometry.scale.stalk_base_radius_or_default().max(1e-3))
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.geometry.height() * 0.5, 0.0)
	}

	fn leaf_radius_world(&self) -> f32 {
		self.geometry.leaf_radius_world()
	}

	/// Unit-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::from_params(&BraidOakTreeParams::unit_from_num(num))
	}
}

impl VegetationComponents for BraidOakTree {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => {
				stick_nodes_high_crook(&self.chain, self.stick_surface_noise, HIGH_STICK_BANDS)
			}
			LodSceneLevel::Medium => stick_nodes_banded(&self.chain, BRAID_MEDIUM_STICK_BANDS),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => stick_nodes_low(&self.chain),
		};
		let nodes: Vec<_> =
			nodes.into_iter().map(|n| n.with_material(chico_stick_material_ref())).collect();
		Layers::from_free(merge_kit_sticks(nodes))
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let leaf_r = self.leaf_radius_world();
		let nodes = match level {
			LodSceneLevel::High => foliage_nodes_banded(&self.chain, HIGH_FOLIAGE_BANDS, leaf_r),
			LodSceneLevel::Medium => foliage_nodes_medium_with_proxy(&self.chain, leaf_r),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => foliage_nodes_low(&self.chain, leaf_r),
		};
		let nodes: Vec<_> =
			nodes.into_iter().map(|n| n.with_material(chico_leaf_material_ref())).collect();
		Layers::from_free(merge_cheap_ball_foliage(nodes))
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(
			StructuralLod::from_extent(
				self.structural_center(),
				self.footprint_radius(),
				self.geometry.height(),
			)
			.with_factors(
				STRUCTURAL_HIGH_FACTOR,
				STRUCTURAL_MEDIUM_FACTOR,
				STRUCTURAL_LOW_FACTOR,
			),
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use chico_vegetation_components::FoliageGeometry;

	#[test]
	fn unit_from_num_is_unit_height_and_stable() -> Result<()> {
		let a = BraidOakTree::unit_from_num(3);
		let b = BraidOakTree::unit_from_num(3);
		let c = BraidOakTree::unit_from_num(4);
		assert!((a.geometry.height() - 1.0).abs() < 1e-5);
		assert_eq!(a.geometry.canopy_noise.seed, 3);
		assert_eq!(a.geometry.canopy_noise.seed, b.geometry.canopy_noise.seed);
		assert_eq!(a.chain.nodes.len(), b.chain.nodes.len());
		assert_ne!(a.geometry.canopy_noise.seed, c.geometry.canopy_noise.seed);
		assert_eq!(a.stick_surface_noise.seed, 3);
		Ok(())
	}

	#[test]
	fn into_unit_from_num_returns_world_size() -> Result<()> {
		let mut params = BraidOakTreeParams::default();
		params.geometry.scale.tree_height = 8.0;
		params.geometry.scale.stalk_base_radius = Some(0.4);
		let (unit, size) = params.into_unit_from_num(7);
		assert!((size - 8.0).abs() < 1e-5);
		assert!((unit.geometry.height() - 1.0).abs() < 1e-5);
		assert!((unit.geometry.scale.stalk_base_radius.unwrap() - 0.05).abs() < 1e-5);
		assert_eq!(unit.geometry.canopy_noise.seed, 7);
		Ok(())
	}

	#[test]
	fn high_emits_merged_stick_and_cheap_ball_collections() -> Result<()> {
		let tree = BraidOakTree::unit_from_num(1);
		let sticks = tree.stick_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(sticks.len(), 1);
		assert!(sticks[0].collection.is_some());
		let foliage = tree.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(foliage.len(), 1);
		assert!(matches!(foliage[0].geometry, FoliageGeometry::CheapBallCollection(_)));
		Ok(())
	}
}
