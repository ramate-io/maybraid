//! **Penmarch Torch** — vase-profile upward flame tree ([#248](https://github.com/ramate-io/maybraid/issues/248), [RFC §3.1.7.4](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/04-penmarch-torch/README.md)).
//!
//! [`PenmarchTorchParams::build`] grows the ball-stick chain once into [`PenmarchTorch`],
//! which implements [`VegetationComponents`].

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, PenmarchTorchChain, PenmarchTorchSbs};
use chico_vegetation_components::{
	FoliageNode, Layers, StickNode, VegetationComponents, VegetationStructuralLodProbe,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use crate::torch_tree::{
	foliage_nodes_banded, foliage_nodes_low, stick_nodes_high, stick_nodes_low,
	stick_nodes_medium, structural_lod_probe, HIGH_FOLIAGE_BANDS, MEDIUM_FOLIAGE_BANDS,
};

/// Authoring / CLI parameters for Penmarch Torch.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct PenmarchTorchParams {
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: PenmarchTorchSbs,
}

impl Default for PenmarchTorchParams {
	fn default() -> Self {
		Self { geometry: PenmarchTorchSbs::default() }
	}
}

impl PenmarchTorchParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> PenmarchTorch {
		PenmarchTorch::from_params(self)
	}
}

/// Built Penmarch Torch: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct PenmarchTorch {
	pub geometry: PenmarchTorchSbs,
	pub chain: BallStickChain<PenmarchTorchChain>,
}

impl PenmarchTorch {
	pub fn from_params(params: &PenmarchTorchParams) -> Self {
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

impl VegetationComponents for PenmarchTorch {
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
			LodSceneLevel::Medium => {
				foliage_nodes_banded(&self.chain, MEDIUM_FOLIAGE_BANDS, leaf_r)
			}
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
