//! **Rory's Head-trained** — top-heavy trained tree with a single horizontal canopy ring ([#254](https://github.com/ramate-io/maybraid/issues/254), [RFC §3.1.7.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/07-rory-s-head-trained/README.md)).
//!
//! [`RorysHeadTrained`] is authored params; [`RorysHeadTrained::build`] grows the ball-stick
//! chain once into [`RorysHeadTrainedInstance`], which implements [`VegetationComponents`].

mod canopy;
mod stick;

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, RorysHeadTrainedSbs, StorybookTreeChain};
use chico_vegetation_components::{
	FoliageNode, Layers, StickNode, VegetationComponents, VegetationStructuralLodProbe,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use canopy::{
	foliage_nodes_banded, foliage_nodes_high, LOW_FOLIAGE_BANDS, MEDIUM_FOLIAGE_BANDS,
};
use stick::{stick_nodes_high, stick_nodes_low, stick_nodes_medium_banded};

/// Typical Rory's Head-trained params (geometry-only; materials are patched externally later).
pub type RorysHeadTrainedStd = RorysHeadTrained;

/// Authoring / CLI parameters for Rory's Head-trained.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct RorysHeadTrained {
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: RorysHeadTrainedSbs,
}

impl Default for RorysHeadTrained {
	fn default() -> Self {
		Self { geometry: RorysHeadTrainedSbs::default() }
	}
}

impl RorysHeadTrained {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> RorysHeadTrainedInstance {
		RorysHeadTrainedInstance::from_params(self)
	}

	/// RFC bush / grape-vine preset (shorter stalk, `0.60 * H` spread).
	pub fn apply_bush_preset(&mut self) {
		self.geometry.apply_bush_preset();
	}
}

/// Built Rory's Head-trained: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct RorysHeadTrainedInstance {
	pub geometry: RorysHeadTrainedSbs,
	pub chain: BallStickChain<StorybookTreeChain>,
}

impl RorysHeadTrainedInstance {
	pub fn from_params(params: &RorysHeadTrained) -> Self {
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

impl VegetationComponents for RorysHeadTrainedInstance {
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
