//! **Jungle Storybook Tree** — dense Storybook construction ([#235](https://github.com/ramate-io/maybraid/issues/235), [RFC §3.1.7.13](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/13-jungle-storybook-tree/README.md)).
//!
//! Same [`StorybookTreeChain`] geometry as [#230](https://github.com/ramate-io/maybraid/issues/230);
//! VegetationComponents emits jungle-growth clusters (palm fronds + spears) plus cheap canopy balls.

mod canopy;
#[allow(dead_code)]
pub mod render_item_plugin;

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, JungleStorybookTreeSbs, StorybookTreeChain};
use chico_vegetation_components::{
	chico_leaf_material_ref, chico_stick_material_ref, FoliageNode, Layers, StickNode,
	StructuralLod, VegetationComponents,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use crate::storybook_tree::canopy::MEDIUM_STICK_BANDS;
use crate::torch_tree::{stick_nodes_banded, stick_nodes_high, stick_nodes_low};
use canopy::{foliage_nodes_for_level, DEFAULT_JUNGLE_GROWTH_RADIUS_SCALE};

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 5.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 15.0;
const STRUCTURAL_LOW_FACTOR: f32 = 24.0;

/// Authoring / CLI parameters for Jungle Storybook Tree.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct JungleStorybookTreeParams {
	/// Flattened [`StorybookTreeSbs`](chico_sbs_geometry::StorybookTreeSbs) (clap defaults are storybook, not jungle).
	/// [`Self::build`] / [`JungleStorybookTree::from_params`] call [`JungleStorybookTreeSbs::apply_jungle_preset`].
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: JungleStorybookTreeSbs,

	/// Share of foliage-eligible nodes that spawn jungle growth.
	#[arg(long, default_value_t = 0.22)]
	pub growth_spawn_fraction: f32,

	/// Assembly scale for jungle-growth fronds (Storybook-specific; independent of Honu).
	#[arg(long, default_value_t = DEFAULT_JUNGLE_GROWTH_RADIUS_SCALE)]
	pub jungle_growth_radius_scale: f32,
}

impl Default for JungleStorybookTreeParams {
	fn default() -> Self {
		Self {
			geometry: JungleStorybookTreeSbs::default(),
			growth_spawn_fraction: 0.22,
			jungle_growth_radius_scale: DEFAULT_JUNGLE_GROWTH_RADIUS_SCALE,
		}
	}
}

impl JungleStorybookTreeParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> JungleStorybookTree {
		JungleStorybookTree::from_params(self)
	}
}

/// Built Jungle Storybook Tree: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct JungleStorybookTree {
	pub geometry: JungleStorybookTreeSbs,
	pub chain: BallStickChain<StorybookTreeChain>,
	pub growth_spawn_fraction: f32,
	pub jungle_growth_radius_scale: f32,
}

impl JungleStorybookTree {
	pub fn from_params(params: &JungleStorybookTreeParams) -> Self {
		let mut geometry = params.geometry.clone();
		geometry.apply_jungle_preset();
		Self {
			chain: geometry.build_chain(),
			geometry,
			growth_spawn_fraction: params.growth_spawn_fraction,
			jungle_growth_radius_scale: params.jungle_growth_radius_scale,
		}
	}

	fn footprint_radius(&self) -> f32 {
		self.chain
			.footprint_radius_at_least(self.geometry.scale.stalk_base_radius_or_default().max(1e-3))
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.geometry.height() * 0.5, 0.0)
	}

	fn leaf_radius_world(&self) -> f32 {
		self.geometry.leaf_radius_world()
	}

	fn height(&self) -> f32 {
		self.geometry.height()
	}
}

impl VegetationComponents for JungleStorybookTree {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => stick_nodes_high(&self.chain),
			LodSceneLevel::Medium => stick_nodes_banded(&self.chain, MEDIUM_STICK_BANDS),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => stick_nodes_low(&self.chain),
		};
		Layers::from_free(nodes).map(|n| n.with_material(chico_stick_material_ref()))
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		Layers::from_free(foliage_nodes_for_level(
			&self.chain,
			level,
			self.growth_spawn_fraction,
			self.jungle_growth_radius_scale,
			self.leaf_radius_world(),
		))
		.map(|n| {
			if n.geometry.is_frond_collection() {
				n
			} else {
				n.with_material(chico_leaf_material_ref())
			}
		})
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(
			StructuralLod::from_extent(
				self.structural_center(),
				self.footprint_radius(),
				self.height(),
			)
			.with_factors(
				STRUCTURAL_HIGH_FACTOR,
				STRUCTURAL_MEDIUM_FACTOR,
				STRUCTURAL_LOW_FACTOR,
			),
		)
	}
}
