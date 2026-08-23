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
	chico_stick_material_ref, FoliageNode, Layers, StickNode, StructuralLod, VegetationComponents,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use crate::palm_crown::FROND_RING_SEED_SALT;
use crate::palm_tree::{
	crown_aabb_from_rings, crown_lod_probe, frond_collection_nodes, layered_proxy_balls,
	trunk_stick_nodes, world_space_frond_shape,
};
use crown::frond_shape_for_ring;

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 36.0;
const STRUCTURAL_LOW_FACTOR: f32 = 72.0;

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
		Self { geometry: params.geometry.clone(), chain: params.geometry.build_chain() }
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
		self.chain
			.footprint_radius_at_least(self.geometry.scale.stalk_base_radius_or_default().max(1e-3))
	}
}

impl VegetationComponents for WaialeaPalm {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::from_free(trunk_stick_nodes(&self.chain))
			.map(|n| n.with_material(chico_stick_material_ref()))
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let rings = self.ring_shapes();
		match level {
			LodSceneLevel::High | LodSceneLevel::Medium => {
				let (center, radius) = crown_lod_probe(
					&rings,
					Some((self.footprint_radius(), self.geometry.height())),
				);
				Layers::from_free(frond_collection_nodes(&rings, center, radius))
			}
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => {
				let (min, max) = crown_aabb_from_rings(&rings);
				Layers::from_free(layered_proxy_balls(min, max))
			}
		}
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		let rings = self.ring_shapes();
		let (center, radius) =
			crown_lod_probe(&rings, Some((self.footprint_radius(), self.geometry.height())));
		Some(StructuralLod::new(center, radius).with_factors(
			STRUCTURAL_HIGH_FACTOR,
			STRUCTURAL_MEDIUM_FACTOR,
			STRUCTURAL_LOW_FACTOR,
		))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::gen::LodSceneLevel;

	#[test]
	fn high_collections_use_structural_crown_probe() -> Result<()> {
		crate::palm_tree::assert_high_collections_match_structural_lod(
			&WaialeaPalmParams::default().build(),
		);
		Ok(())
	}

	#[test]
	fn low_is_two_layered_balls() -> Result<()> {
		let built = WaialeaPalmParams::default().build();
		let low = built.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
		assert_eq!(low.len(), 2);
		assert!(low.iter().all(|n| n.geometry.is_layered_ball()));
		Ok(())
	}
}
