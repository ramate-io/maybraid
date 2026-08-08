//! **Waialea Palm** — arched trunk + light upward frond crown ([#255](https://github.com/ramate-io/maybraid/issues/255), [RFC §3.1.7.8](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/08-waialea-palm/README.md)).
//!
//! [`WaialeaPalmParams::build`] grows the arched trunk once into [`WaialeaPalm`], which
//! implements [`VegetationComponents`]: trunk sticks; per-frond collections at High/Medium;
//! dual layered-ball crown proxy at Low/UltraLow.

mod crown;
pub mod render_item_plugin;
#[allow(dead_code)]
mod stick;
#[allow(dead_code)]
mod tuft;

use bevy::prelude::*;
use chico_ball_components::frond::FrondCrownShape;
use chico_sbs_geometry::{BallStickChain, WaialeaPalmChain, WaialeaPalmSbs};
use chico_vegetation_components::{
	FoliageNode, Layers, StickNode, VegetationComponents, StructuralLod,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use crate::palm_crown::FROND_RING_SEED_SALT;
use crate::palm_tree::{
	crown_aabb_from_rings, frond_collection_nodes, layered_proxy_balls, palm_structural_lod,
	trunk_stick_nodes, world_space_frond_shape,
};
use crate::torch_tree::structural_tree_radius;
use crown::frond_shape_for_ring;

/// Authoring / CLI parameters for Waialea Palm.
#[derive(Component, Clone, Args, Debug, PartialEq)]
#[command(rename_all = "kebab-case")]
pub struct WaialeaPalmParams {
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: WaialeaPalmSbs,
}

impl Default for WaialeaPalmParams {
	fn default() -> Self {
		Self { geometry: WaialeaPalmSbs::default() }
	}
}

impl WaialeaPalmParams {
	pub fn build(&self) -> WaialeaPalm {
		WaialeaPalm::from_params(self)
	}
}

/// Built Waialea Palm: geometry plus a single grown arched trunk chain.
#[derive(Clone)]
pub struct WaialeaPalm {
	pub geometry: WaialeaPalmSbs,
	pub chain: BallStickChain<WaialeaPalmChain>,
}

impl WaialeaPalm {
	pub fn from_params(params: &WaialeaPalmParams) -> Self {
		Self {
			geometry: params.geometry.clone(),
			chain: params.geometry.build_chain(),
		}
	}

	fn foliage_seed(&self) -> i32 {
		self.geometry.trunk_noise.seed
	}

	fn ring_shapes(&self) -> Vec<(Vec3, FrondCrownShape)> {
		let seed = self.foliage_seed();
		let scale = self.geometry.frond_world_scale;
		(0..self.geometry.crown.ring_count)
			.map(|ring| {
				let anchor = self.geometry.crown_ring_position(&self.chain, ring);
				let local = frond_shape_for_ring(
					&self.geometry,
					ring,
					seed.wrapping_add(ring as i32 * FROND_RING_SEED_SALT),
				);
				(anchor, world_space_frond_shape(local, scale))
			})
			.collect()
	}

	fn footprint_radius(&self) -> f32 {
		self.chain.footprint_radius_at_least(
			self.geometry.scale.stalk_base_radius_or_default().max(1e-3),
		)
	}
}

impl VegetationComponents for WaialeaPalm {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::from_free(trunk_stick_nodes(&self.chain))
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		match level {
			LodSceneLevel::High | LodSceneLevel::Medium => {
				Layers::from_free(frond_collection_nodes(self.ring_shapes()))
			}
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => {
				let (min, max) = crown_aabb_from_rings(self.ring_shapes());
				Layers::from_free(layered_proxy_balls(min, max))
			}
		}
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		let (min, max) = crown_aabb_from_rings(self.ring_shapes());
		let crown_center = (min + max) * 0.5;
		let crown_r = ((max - min) * 0.5).max_element();
		let radius = structural_tree_radius(self.footprint_radius(), self.geometry.height())
			.max(crown_r);
		Some(palm_structural_lod(crown_center, radius))
	}
}
