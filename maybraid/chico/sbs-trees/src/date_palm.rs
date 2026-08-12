//! **Date Palm** — columnar trunk + stacked frond crown ([#256](https://github.com/ramate-io/maybraid/issues/256), [RFC §3.1.7.9](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/09-date-palm/README.md)).
//!
//! [`DatePalmParams::build`] grows the trunk chain once into [`DatePalm`], which implements
//! [`VegetationComponents`]: trunk sticks at all bands; per-frond [`FrondCollection`]s at
//! High/Medium; dual layered-ball crown proxy at Low/UltraLow.
//!
//! Unit crown archetypes for Placement-scaled groves live on
//! [`PalmCrownParams`](crate::PalmCrownParams) (`unit_full_from_num` /
//! `unit_detail_from_num`). Date Palm keeps SBS trunk + height-fraction fronds; use
//! [`DatePalmParams::unit_full_from_num`] only to key trunk/foliage noise and mirror full
//! crown ring/frond counts.

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
	chico_stick_material_ref, FoliageNode, Layers, StickNode, VegetationComponents, StructuralLod,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use crate::palm_crown::{PalmCrownParams, FROND_RING_SEED_SALT};
use crate::palm_tree::{
	crown_aabb_from_rings, frond_collection_nodes, layered_proxy_balls, palm_structural_lod,
	trunk_stick_nodes, world_space_frond_shape,
};
use crate::torch_tree::structural_tree_radius;
use crown::frond_shape_for_ring;

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

impl VegetationComponents for DatePalm {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::from_free(trunk_stick_nodes(&self.chain))
			.map(|n| n.with_material(chico_stick_material_ref()))
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
