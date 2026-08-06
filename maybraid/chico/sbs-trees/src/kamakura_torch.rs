//! **Kamakura Torch** — near-vertical flame variant (linear 48°→70° crown); same vase profile as Penmarch.
//!
//! [`KamakuraTorch`] is authored params; [`KamakuraTorch::build`] grows the ball-stick
//! chain once into [`KamakuraTorchInstance`], which implements [`VegetationComponents`].

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, KamakuraTorchChain, KamakuraTorchSbs};
use chico_vegetation_components::{
	FoliageNode, Layers, StickNode, VegetationComponents, VegetationStructuralLodProbe,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use crate::torch_tree::{
	foliage_nodes_banded, foliage_nodes_high, stick_nodes_high, stick_nodes_low,
	stick_nodes_medium_banded, LOW_FOLIAGE_BANDS, MEDIUM_FOLIAGE_BANDS,
};

/// Typical Kamakura Torch params (geometry-only; materials are patched externally later).
pub type KamakuraTorchStd = KamakuraTorch;

/// Authoring / CLI parameters for Kamakura Torch.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct KamakuraTorch {
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: KamakuraTorchSbs,
}

impl Default for KamakuraTorch {
	fn default() -> Self {
		Self { geometry: KamakuraTorchSbs::default() }
	}
}

impl KamakuraTorch {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> KamakuraTorchInstance {
		KamakuraTorchInstance::from_params(self)
	}
}

/// Built Kamakura Torch: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct KamakuraTorchInstance {
	pub geometry: KamakuraTorchSbs,
	pub chain: BallStickChain<KamakuraTorchChain>,
}

impl KamakuraTorchInstance {
	pub fn from_params(params: &KamakuraTorch) -> Self {
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

impl VegetationComponents for KamakuraTorchInstance {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => stick_nodes_high(&self.chain),
			LodSceneLevel::Medium => stick_nodes_medium_banded(&self.chain),
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
			LodSceneLevel::High => foliage_nodes_high(&self.chain, leaf_r),
			LodSceneLevel::Medium => foliage_nodes_banded(&self.chain, MEDIUM_FOLIAGE_BANDS, leaf_r),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => {
				foliage_nodes_banded(&self.chain, LOW_FOLIAGE_BANDS, leaf_r)
			}
		};
		Layers::from_free(nodes)
	}

	fn structural_lod_probe(&self) -> Option<VegetationStructuralLodProbe> {
		Some(VegetationStructuralLodProbe::new(
			self.structural_center(),
			self.footprint_radius(),
		))
	}
}
