//! **Liam's Conifer** — sparse dry conifer ([RFC-183 §3.1.7.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/02-liam-s-conifer/README.md), [#244](https://github.com/ramate-io/maybraid/issues/244)).
//!
//! [`LiamsConiferParams::build`] grows the ball-stick chain once into [`LiamsConifer`], which
//! implements [`VegetationComponents`]. Sticks reuse Northern / Liam banding; foliage uses
//! cheap-ball joint clusters sized by [`LiamsConiferSbs::tuft_world_scale`] (no SucculentTuft
//! mesh under VegetationComponents).

pub mod render_item_plugin;
#[allow(dead_code)]
pub mod stick;
#[allow(dead_code)]
mod tuft;

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, LiamsConiferChain, LiamsConiferSbs};
use chico_vegetation_components::{
	FoliageNode, Layers, StickNode, VegetationComponents, VegetationStructuralLodProbe,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use crate::northern_conifer::canopy::{
	foliage_nodes_banded, foliage_nodes_low_single_proxy, foliage_nodes_medium_no_proxy,
	HIGH_FOLIAGE_BANDS,
};
use crate::northern_conifer::stick::{
	stick_nodes_high, stick_nodes_low, stick_nodes_medium_liams,
};
use crate::torch_tree::structural_lod_probe;

/// Authoring / CLI parameters for Liam's Conifer.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct LiamsConiferParams {
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: LiamsConiferSbs,
}

impl Default for LiamsConiferParams {
	fn default() -> Self {
		Self { geometry: LiamsConiferSbs::default() }
	}
}

impl LiamsConiferParams {
	pub fn build(&self) -> LiamsConifer {
		LiamsConifer::from_params(self)
	}
}

/// Built Liam's Conifer: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct LiamsConifer {
	pub geometry: LiamsConiferSbs,
	pub chain: BallStickChain<LiamsConiferChain>,
}

impl LiamsConifer {
	pub fn from_params(params: &LiamsConiferParams) -> Self {
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
		Vec3::new(0.0, self.geometry.scale.stalk_height * 0.5, 0.0)
	}

	fn height(&self) -> f32 {
		self.geometry.scale.stalk_height.max(1e-6)
	}

	fn tuft_radius_world(&self) -> f32 {
		self.geometry.tuft_world_scale()
	}
}

impl VegetationComponents for LiamsConifer {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => stick_nodes_high(&self.chain),
			LodSceneLevel::Medium => stick_nodes_medium_liams(&self.chain),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => stick_nodes_low(&self.chain),
		};
		Layers::from_free(nodes)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let tuft_r = self.tuft_radius_world();
		// Arid look: Medium drops density via banding only (no mass proxy); Low keeps one proxy.
		let nodes = match level {
			LodSceneLevel::High => {
				foliage_nodes_banded(&self.chain, HIGH_FOLIAGE_BANDS, tuft_r, 1.0, 0.0, 0.0)
			}
			LodSceneLevel::Medium => foliage_nodes_medium_no_proxy(&self.chain, tuft_r, 1.0),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => {
				foliage_nodes_low_single_proxy(&self.chain, tuft_r, 1.0)
			}
		};
		Layers::from_free(nodes)
	}

	fn structural_lod_probe(&self) -> Option<VegetationStructuralLodProbe> {
		Some(structural_lod_probe(
			self.structural_center(),
			self.footprint_radius(),
			self.height(),
		))
	}
}
