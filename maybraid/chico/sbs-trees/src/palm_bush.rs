//! **Palm Bush** — trunkless ground-anchored frond cluster ([#231](https://github.com/ramate-io/maybraid/issues/231), [RFC §3.1.7.10](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/10-palm-bush/README.md)).
//!
//! [`PalmBushParams::build`] resolves ring anchors into [`PalmBush`], which implements
//! [`VegetationComponents`]: per-frond collections at High/Medium; dual layered-ball proxy
//! at Low/UltraLow (no sticks).
//!
//! Standalone unit crowns (grove Placement scale) prefer
//! [`PalmCrownParams::unit_detail_for_height_from_num`](crate::PalmCrownParams::unit_detail_for_height_from_num).
//! [`PalmBushParams::unit_detail_from_num`] is a thin SBS bridge that keys foliage noise and
//! mirrors detail crown counts without rewriting height-fraction frond shaping.

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

use crate::palm_crown::{PalmCrownParams, FROND_RING_SEED_SALT};
use crate::palm_tree::{
	crown_aabb_from_rings, crown_lod_probe, frond_collection_nodes, layered_proxy_balls,
	world_space_frond_shape,
};
use crown::frond_shape_for_ring;

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 36.0;
const STRUCTURAL_LOW_FACTOR: f32 = 72.0;

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

	/// Understory bush keyed by `num` — crown counts track
	/// [`PalmCrownParams::unit_detail_from_num`].
	///
	/// Frond lengths still follow SBS height fractions; use [`PalmCrownParams`] directly when
	/// you need a unit-normalized crown mesh.
	pub fn unit_detail_from_num(num: u32) -> Self {
		let crown = PalmCrownParams::unit_detail_from_num(num);
		let mut params = Self::default();
		params.geometry.crown.ring_count = crown.ring_count;
		params.geometry.crown.fronds_per_ring = crown.shape.frond_count;
		params.geometry.foliage_noise.seed = num as i32;
		params
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
		let rings = self.ring_shapes();
		match level {
			LodSceneLevel::High | LodSceneLevel::Medium => {
				let (center, radius) = crown_lod_probe(&rings, None);
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
		let (center, radius) = crown_lod_probe(&rings, None);
		Some(
			StructuralLod::new(center, radius).with_factors(
				STRUCTURAL_HIGH_FACTOR,
				STRUCTURAL_MEDIUM_FACTOR,
				STRUCTURAL_LOW_FACTOR,
			),
		)
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
		crate::palm_tree::assert_high_collections_match_structural_lod(&built);
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
