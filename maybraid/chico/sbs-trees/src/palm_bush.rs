//! **Palm Bush** — trunkless ground-anchored frond cluster ([#231](https://github.com/ramate-io/maybraid/issues/231), [RFC §3.1.7.10](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/10-palm-bush/README.md)).
//!
//! [`PalmBushParams::build`] resolves ring anchors into [`PalmBush`], which implements
//! [`VegetationComponents`]: per-frond collections at High/Medium; dual layered-ball proxy
//! at Low/UltraLow (no sticks).

mod crown;
pub mod render_item_plugin;
#[allow(dead_code)]
mod tuft;

use bevy::prelude::*;
use chico_ball_components::frond::FrondCrownShape;
use chico_sbs_geometry::PalmBushSbs;
use chico_vegetation_components::{
	FoliageNode, Layers, StickNode, VegetationComponents, StructuralLod,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use crate::palm_crown::FROND_RING_SEED_SALT;
use crate::palm_tree::{
	crown_aabb_from_rings, frond_collection_nodes, layered_proxy_balls, palm_structural_lod,
	world_space_frond_shape,
};
use crown::frond_shape_for_ring;

/// Authoring / CLI parameters for Palm Bush.
#[derive(Component, Clone, Args, Debug, PartialEq)]
#[command(rename_all = "kebab-case")]
pub struct PalmBushParams {
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: PalmBushSbs,
}

impl Default for PalmBushParams {
	fn default() -> Self {
		Self { geometry: PalmBushSbs::default() }
	}
}

impl PalmBushParams {
	pub fn new(geometry: PalmBushSbs) -> Self {
		Self { geometry }
	}

	pub fn build(&self) -> PalmBush {
		PalmBush::from_params(self)
	}
}

/// Built Palm Bush: geometry (ring anchors are derived on demand).
#[derive(Clone, Debug, PartialEq)]
pub struct PalmBush {
	pub geometry: PalmBushSbs,
}

impl PalmBush {
	pub fn from_params(params: &PalmBushParams) -> Self {
		Self { geometry: params.geometry.clone() }
	}

	fn foliage_seed(&self) -> i32 {
		self.geometry.foliage_noise.seed
	}

	fn ring_shapes(&self) -> Vec<(Vec3, FrondCrownShape)> {
		let seed = self.foliage_seed();
		let scale = self.geometry.frond_world_scale;
		(0..self.geometry.crown.ring_count)
			.map(|ring| {
				let anchor = self.geometry.crown_ring_position(ring);
				let local = frond_shape_for_ring(
					&self.geometry,
					ring,
					seed.wrapping_add(ring as i32 * FROND_RING_SEED_SALT),
				);
				(anchor, world_space_frond_shape(local, scale))
			})
			.collect()
	}
}

impl VegetationComponents for PalmBush {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::new()
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
		let center = (min + max) * 0.5;
		let radius = ((max - min) * 0.5).max_element().max(1e-3);
		Some(palm_structural_lod(center, radius))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::gen::LodSceneLevel;

	#[test]
	fn high_emits_small_frond_collections() -> Result<()> {
		use crate::palm_tree::FRONDS_PER_COLLECTION;
		let built = PalmBushParams::default().build();
		let nodes = built.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		let fronds = built.geometry.crown.ring_count * built.geometry.crown.fronds_per_ring;
		let expected = (fronds as usize).div_ceil(FRONDS_PER_COLLECTION);
		assert_eq!(nodes.len(), expected);
		let collection = nodes[0].geometry.as_frond_collection().expect("collection");
		assert!(collection.runs.len() <= FRONDS_PER_COLLECTION);
		assert!(collection.runs[0].segments.len() >= 4, "authored rachis segments kept");
		Ok(())
	}

	#[test]
	fn low_is_two_layered_balls() -> Result<()> {
		let built = PalmBushParams::default().build();
		let low = built.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
		assert_eq!(low.len(), 2);
		assert!(low.iter().all(|n| n.geometry.is_layered_ball()));
		Ok(())
	}
}
