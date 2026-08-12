//! **Rory's Head-trained** — top-heavy trained tree with a single horizontal canopy ring ([#254](https://github.com/ramate-io/maybraid/issues/254), [RFC §3.1.7.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/07-rory-s-head-trained/README.md)).
//!
//! [`RorysHeadTrainedParams::build`] grows the ball-stick chain once into [`RorysHeadTrained`],
//! which implements [`VegetationComponents`].
//!
//! Structural / stick LOD matches Penmarch Torch (`torch_tree`); foliage keeps joint
//! candidates with cheap-ball banding and no layered mass proxies.

mod canopy;

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, RorysHeadTrainedSbs, StorybookTreeChain};
use chico_vegetation_components::{
	chico_leaf_material_ref, chico_stick_material_ref, FoliageNode, Layers, StickNode,
	VegetationComponents, StructuralLod,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use crate::torch_tree::{
	stick_nodes_high, stick_nodes_low, stick_nodes_medium, structural_lod_for,
};
use canopy::{
	foliage_nodes_banded, foliage_nodes_low, foliage_nodes_medium, HIGH_FOLIAGE_BANDS,
};

/// Authoring / CLI parameters for Rory's Head-trained.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct RorysHeadTrainedParams {
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: RorysHeadTrainedSbs,
}

impl Default for RorysHeadTrainedParams {
	fn default() -> Self {
		Self { geometry: RorysHeadTrainedSbs::default() }
	}
}

impl RorysHeadTrainedParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> RorysHeadTrained {
		RorysHeadTrained::from_params(self)
	}

	/// RFC bush / grape-vine preset (shorter stalk, `0.60 * H` spread).
	pub fn apply_bush_preset(&mut self) {
		self.geometry.apply_bush_preset();
	}
}

/// Built Rory's Head-trained: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct RorysHeadTrained {
	pub geometry: RorysHeadTrainedSbs,
	pub chain: BallStickChain<StorybookTreeChain>,
}

impl RorysHeadTrained {
	pub fn from_params(params: &RorysHeadTrainedParams) -> Self {
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

impl VegetationComponents for RorysHeadTrained {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => stick_nodes_high(&self.chain),
			LodSceneLevel::Medium => stick_nodes_medium(&self.chain),
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
			LodSceneLevel::High => {
				foliage_nodes_banded(&self.chain, HIGH_FOLIAGE_BANDS, leaf_r)
			}
			LodSceneLevel::Medium => foliage_nodes_medium(&self.chain, leaf_r),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => foliage_nodes_low(&self.chain, leaf_r),
		};
		Layers::from_free(nodes).map(|n| n.with_material(chico_leaf_material_ref()))
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(structural_lod_for(
			self.structural_center(),
			self.footprint_radius(),
			self.geometry.height(),
		))
	}
}
