//! **Storybook Tree** — default broadleaf ball-stick assembly ([#230](https://github.com/ramate-io/maybraid/issues/230), [RFC §3.1.7.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/01-storybook-tree/README.md)).
//!
//! [`StorybookTreeParams::build`] grows the ball-stick chain once into [`StorybookTree`],
//! which implements [`VegetationComponents`].
//!
//! Structural / stick LOD matches Penmarch Torch (`torch_tree`); foliage keeps outer /
//! terminal plane-splay candidates with torch-like banding and no mass proxies.

mod canopy;
pub mod render_item_plugin;
pub(crate) mod stick;

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, StorybookTreeChain, StorybookTreeSbs};
use chico_vegetation_components::{
	FoliageNode, Layers, StickNode, VegetationComponents, VegetationStructuralLodProbe,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use crate::torch_tree::{
	stick_nodes_high, stick_nodes_low, stick_nodes_medium, structural_lod_probe,
};
use canopy::{
	foliage_nodes_banded, foliage_nodes_low, foliage_nodes_medium, HIGH_FOLIAGE_BANDS,
};

/// Authoring / CLI parameters for Storybook Tree.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct StorybookTreeParams {
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: StorybookTreeSbs,
}

impl Default for StorybookTreeParams {
	fn default() -> Self {
		Self { geometry: StorybookTreeSbs::default() }
	}
}

impl StorybookTreeParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> StorybookTree {
		StorybookTree::from_params(self)
	}
}

/// Built Storybook Tree: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct StorybookTree {
	pub geometry: StorybookTreeSbs,
	pub chain: BallStickChain<StorybookTreeChain>,
}

impl StorybookTree {
	pub fn from_params(params: &StorybookTreeParams) -> Self {
		Self {
			geometry: params.geometry.clone(),
			chain: params.geometry.build_chain(),
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

impl VegetationComponents for StorybookTree {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => stick_nodes_high(&self.chain),
			LodSceneLevel::Medium => stick_nodes_medium(&self.chain),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => stick_nodes_low(&self.chain),
		};
		Layers::from_free(nodes)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let leaf_r = self.leaf_radius_world();
		let nodes = match level {
			LodSceneLevel::High => {
				foliage_nodes_banded(&self.chain, HIGH_FOLIAGE_BANDS, leaf_r)
			}
			LodSceneLevel::Medium => foliage_nodes_medium(&self.chain, leaf_r),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => foliage_nodes_low(&self.chain, leaf_r),
		};
		Layers::from_free(nodes)
	}

	fn structural_lod_probe(&self) -> Option<VegetationStructuralLodProbe> {
		Some(structural_lod_probe(
			self.structural_center(),
			self.footprint_radius(),
			self.geometry.height(),
		))
	}
}
