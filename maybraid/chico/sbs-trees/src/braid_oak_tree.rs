//! **Braid Oak Tree** — gnarled broadleaf with crook-cylinder branches ([#234](https://github.com/ramate-io/maybraid/issues/234), [RFC §3.1.7.13](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/13-braid-oak/README.md)).
//!
//! [`BraidOakTreeParams::build`] applies the braid preset, grows the ball-stick chain once into
//! [`BraidOakTree`], which implements [`VegetationComponents`].
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
	VegetationComponents, StructuralLod,
};
use clap::Args;
use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::storybook_tree::canopy::{
	foliage_nodes_banded, foliage_nodes_low, foliage_nodes_medium_with_proxy, HIGH_FOLIAGE_BANDS,
	BRAID_MEDIUM_STICK_BANDS,
};
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
		self.chain.footprint_radius_at_least(
			self.geometry.scale.stalk_base_radius_or_default().max(1e-3),
		)
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.geometry.height() * 0.5, 0.0)
	}

	fn leaf_radius_world(&self) -> f32 {
		self.geometry.leaf_radius_world()
	}
}

impl VegetationComponents for BraidOakTree {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => stick_nodes_high_crook(
				&self.chain,
				self.stick_surface_noise,
				HIGH_STICK_BANDS,
			),
			LodSceneLevel::Medium => stick_nodes_banded(&self.chain, BRAID_MEDIUM_STICK_BANDS),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => stick_nodes_low(&self.chain),
		};
		Layers::from_free(nodes).map(|n| n.with_material(chico_stick_material_ref()))
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
		Layers::from_free(nodes).map(|n| n.with_material(chico_leaf_material_ref()))
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
