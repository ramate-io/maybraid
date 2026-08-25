//! **Date Palm** — columnar trunk + stacked frond crown ([#256](https://github.com/ramate-io/maybraid/issues/256), [RFC §3.1.7.9](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/09-date-palm/README.md)).
//!
//! [`DatePalmParams::build`] grows the trunk chain once into [`DatePalm`], which implements
//! [`VegetationComponents`]: trunk sticks at all bands; per-frond [`FrondCollection`]s at
//! High/Medium; dual layered-ball crown proxy at Low/UltraLow.
//!
//! [`DatePalm::unit_from_num`] / [`DatePalmParams::into_unit_from_num`] normalize the
//! SBS trunk to unit height and key trunk noise by a variant index. Emission folds
//! trunk sticks into a collection; frond collections stay separate.
//!
//! Unit crown archetypes for Placement-scaled groves also live on
//! [`PalmCrownParams`](crate::PalmCrownParams) (`unit_full_for_height_from_num` /
//! `unit_detail_for_height_from_num`). [`DatePalmParams::unit_full_from_num`] only keys
//! trunk/foliage noise and mirrors full crown ring/frond counts.

mod crown;
pub mod render_item_plugin;
#[allow(dead_code)]
mod stick;
#[allow(dead_code)]
mod tuft;

use bevy::prelude::*;
use chico_ball_components::frond::FrondCrownShape;
use chico_sbs_geometry::{BallStickChain, DatePalmChain, DatePalmSbs};
use chico_vegetation_components::{
	chico_stick_material_ref, FoliageNode, Layers, StickNode, StructuralLod, VegetationComponents,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use crate::palm_crown::{PalmCrownParams, FROND_RING_SEED_SALT};
use crate::palm_tree::{
	crown_aabb_from_rings, crown_lod_probe, frond_collection_nodes, layered_proxy_balls,
	trunk_stick_nodes, world_space_frond_shape,
};
use crate::storybook_tree::merge_kit_sticks;
use crown::frond_shape_for_ring;

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 50.0;
const STRUCTURAL_LOW_FACTOR: f32 = 72.0;

/// Authoring / CLI parameters for Date Palm.
#[derive(Component, Clone, Args, Debug, PartialEq)]
#[command(rename_all = "kebab-case")]
pub struct DatePalmParams {
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: DatePalmSbs,
}

impl Default for DatePalmParams {
	fn default() -> Self {
		Self { geometry: DatePalmSbs::default() }
	}
}

impl DatePalmParams {
	/// Tree-top date palm keyed by `num` — crown counts track
	/// [`PalmCrownParams::unit_full_from_num`] (SBS frond shaping still height-fraction).
	pub fn unit_full_from_num(num: u32) -> Self {
		let crown = PalmCrownParams::unit_full_from_num(num);
		let mut params = Self::default();
		params.geometry.crown.ring_count = crown.ring_count;
		params.geometry.crown.fronds_per_ring = crown.shape.frond_count;
		params.geometry.trunk_noise.seed = num as i32;
		params
	}

	/// Unit-height palm whose trunk noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::default().into_unit_from_num(num).0
	}

	/// Normalize this params set to unit height keyed by `num`.
	pub fn into_unit_from_num(self, num: u32) -> (Self, f32) {
		let mut geometry = self.geometry;
		let size = geometry.height().max(1e-4);
		let inv = 1.0 / size;
		geometry.scale.stalk_height = 1.0;
		if let Some(radius) = geometry.scale.stalk_base_radius {
			geometry.scale.stalk_base_radius = Some((radius * inv).max(1e-6));
		}
		geometry.trunk_noise.seed = num as i32;
		(Self { geometry }, size)
	}

	pub fn build(&self) -> DatePalm {
		DatePalm::from_params(self)
	}
}

/// Built Date Palm: geometry plus a single grown trunk chain.
#[derive(Clone)]
pub struct DatePalm {
	pub geometry: DatePalmSbs,
	pub chain: BallStickChain<DatePalmChain>,
}

impl DatePalm {
	pub fn from_params(params: &DatePalmParams) -> Self {
		Self { geometry: params.geometry.clone(), chain: params.geometry.build_chain() }
	}

	/// Unit-height palm whose trunk noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::from_params(&DatePalmParams::unit_from_num(num))
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

impl VegetationComponents for DatePalm {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		let nodes: Vec<_> = trunk_stick_nodes(&self.chain)
			.into_iter()
			.map(|n| n.with_material(chico_stick_material_ref()))
			.collect();
		Layers::from_free(merge_kit_sticks(nodes))
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
			&DatePalmParams::default().build(),
		);
		Ok(())
	}

	#[test]
	fn low_is_two_layered_balls() -> Result<()> {
		let built = DatePalmParams::default().build();
		let low = built.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
		assert_eq!(low.len(), 2);
		assert!(low.iter().all(|n| n.geometry.is_layered_ball()));
		Ok(())
	}

	#[test]
	fn unit_from_num_is_unit_height_and_stable() -> Result<()> {
		let a = DatePalm::unit_from_num(3);
		let b = DatePalm::unit_from_num(3);
		let c = DatePalm::unit_from_num(4);
		assert!((a.geometry.height() - 1.0).abs() < 1e-5);
		assert_eq!(a.geometry.trunk_noise.seed, 3);
		assert_eq!(a.geometry.trunk_noise.seed, b.geometry.trunk_noise.seed);
		assert_eq!(a.chain.nodes.len(), b.chain.nodes.len());
		assert_ne!(a.geometry.trunk_noise.seed, c.geometry.trunk_noise.seed);
		Ok(())
	}

	#[test]
	fn into_unit_from_num_returns_world_size() -> Result<()> {
		let mut params = DatePalmParams::default();
		params.geometry.scale.stalk_height = 8.0;
		params.geometry.scale.stalk_base_radius = Some(0.4);
		let (unit, size) = params.into_unit_from_num(7);
		assert!((size - 8.0).abs() < 1e-5);
		assert!((unit.geometry.height() - 1.0).abs() < 1e-5);
		assert!((unit.geometry.scale.stalk_base_radius.unwrap() - 0.05).abs() < 1e-5);
		assert_eq!(unit.geometry.trunk_noise.seed, 7);
		Ok(())
	}

	#[test]
	fn high_emits_merged_stick_collection() -> Result<()> {
		let tree = DatePalm::unit_from_num(1);
		let sticks = tree.stick_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(sticks.len(), 1);
		assert!(sticks[0].collection.is_some());
		let foliage = tree.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(!foliage.is_empty());
		assert!(foliage.iter().all(|n| n.geometry.is_frond_collection()));
		Ok(())
	}
}
