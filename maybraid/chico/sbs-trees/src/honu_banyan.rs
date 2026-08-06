//! **Honu Banyan** — wide spreading banyan ([#250](https://github.com/ramate-io/maybraid/issues/250), [RFC §3.1.7.5](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/05-honu-banyan/README.md)).
//!
//! [`HonuBanyanParams::build`] grows the ball-stick chain once into [`HonuBanyan`],
//! which implements [`VegetationComponents`].
//!
//! Structural LOD:
//! - **High** — full sticks (3×4 rings, depth 3..5, child 1..3, ±70° ray, longer hops); jungle growth + banded canopy
//! - **Medium** — trunk + banded sticks; banded growth/canopy + mid layered proxy
//! - **Low** — trunk + ~1/4 descenders; cheap canopy balls + mid proxy

mod canopy;
pub mod render_item_plugin;
mod stick;

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, HonuBanyanChain, HonuBanyanSbs};
use chico_vegetation_components::{
	FoliageNode, Layers, StickNode, VegetationComponents, VegetationStructuralLodProbe,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use canopy::{foliage_nodes_for_level, DEFAULT_HONU_GROWTH_RADIUS_SCALE};
use stick::{
	keep_stick_on_low, stick_node_for_segment, stick_nodes_medium_banded, stick_role_for_segment,
};

/// Authoring / CLI parameters for Honu Banyan.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct HonuBanyanParams {
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: HonuBanyanSbs,

	/// Fraction of qualifying outer-ring nodes that spawn jungle growth.
	#[arg(long, default_value_t = 0.28)]
	pub growth_spawn_fraction: f32,

	/// Assembly scale for jungle-growth fronds (Honu-specific; independent of Storybook).
	#[arg(long, default_value_t = DEFAULT_HONU_GROWTH_RADIUS_SCALE)]
	pub jungle_growth_radius_scale: f32,
}

impl Default for HonuBanyanParams {
	fn default() -> Self {
		Self {
			geometry: HonuBanyanSbs::default(),
			growth_spawn_fraction: 0.28,
			jungle_growth_radius_scale: DEFAULT_HONU_GROWTH_RADIUS_SCALE,
		}
	}
}

impl HonuBanyanParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> HonuBanyan {
		HonuBanyan::from_params(self)
	}
}

/// Built Honu Banyan: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct HonuBanyan {
	pub geometry: HonuBanyanSbs,
	pub chain: BallStickChain<HonuBanyanChain>,
	pub growth_spawn_fraction: f32,
	pub jungle_growth_radius_scale: f32,
}

impl HonuBanyan {
	pub fn from_params(params: &HonuBanyanParams) -> Self {
		Self {
			geometry: params.geometry.clone(),
			chain: params.geometry.build_chain(),
			growth_spawn_fraction: params.growth_spawn_fraction,
			jungle_growth_radius_scale: params.jungle_growth_radius_scale,
		}
	}

	fn footprint_radius(&self) -> f32 {
		self.chain.footprint_radius_at_least(
			self.geometry.scale.to_stalk().stalk_base_radius.max(1e-3),
		)
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.geometry.scale.tree_height * 0.5, 0.0)
	}

	fn stick_nodes_high(&self) -> Vec<StickNode> {
		self.chain
			.segments_with_hysteresis()
			.filter_map(|(segment, parent, _)| stick_node_for_segment(&segment, parent))
			.collect()
	}

	fn stick_nodes_medium(&self) -> Vec<StickNode> {
		stick_nodes_medium_banded(
			self.chain
				.segments_with_hysteresis()
				.map(|(segment, parent, _)| (segment, parent)),
		)
	}

	fn stick_nodes_low(&self) -> Vec<StickNode> {
		let mut descender_index = 0usize;
		self.chain
			.segments_with_hysteresis()
			.filter_map(|(segment, parent, _)| {
				let role = stick_role_for_segment(&segment, parent);
				if !keep_stick_on_low(role, &mut descender_index) {
					return None;
				}
				stick_node_for_segment(&segment, parent)
			})
			.collect()
	}

	fn foliage_for(&self, level: LodSceneLevel) -> Vec<FoliageNode> {
		foliage_nodes_for_level(
			&self.chain,
			level,
			self.growth_spawn_fraction,
			self.jungle_growth_radius_scale,
			self.geometry.crown_floor_world_y(),
			self.geometry.leaf_ball_size(),
		)
	}
}

impl VegetationComponents for HonuBanyan {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => self.stick_nodes_high(),
			LodSceneLevel::Medium => self.stick_nodes_medium(),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => self.stick_nodes_low(),
		};
		Layers::from_free(nodes)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		Layers::from_free(self.foliage_for(level))
	}

	fn structural_lod_probe(&self) -> Option<VegetationStructuralLodProbe> {
		Some(VegetationStructuralLodProbe::new(
			self.structural_center(),
			self.footprint_radius(),
		))
	}
}
