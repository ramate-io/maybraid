//! [`JungleGrowth`](chico_tree_components::JungleGrowth) clusters at sampled canopy nodes ([#235](https://github.com/ramate-io/maybraid/issues/235)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::chain::storybook_tree::StorybookTreeChain;
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::render::mix_seed::mix_seed_below_fraction;
use chico_sbs_geometry::{BallStickChain, BallStickNode};
use chico_tree_components::{JungleGrowth, JungleGrowthShape};
use procedural_common::NoiseParams;

use super::canopy::should_allocate_jungle_foliage;

/// Spawn transform scale relative to the graph node's branch radius.
pub const JUNGLE_GROWTH_RADIUS_SCALE: f32 = 1.2;

#[derive(Clone)]
pub(crate) struct JungleStorybookGrowthRule<BodyM, BodyS, FoliageM, FoliageS>
where
	BodyM: Material,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>>,
	FoliageM: Material,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>>,
{
	pub growth_spawn_fraction: f32,
	pub body_noise: NoiseParams,
	pub foliage_noise: NoiseParams,
	pub body_material: BodyS,
	pub foliage_material: FoliageS,
	pub(crate) __marker: PhantomData<fn() -> (BodyM, FoliageM)>,
}

impl<BodyM, BodyS, FoliageM, FoliageS>
	BallRenderRule<JungleGrowth<BodyM, BodyS, FoliageM, FoliageS>, StorybookTreeChain>
	for JungleStorybookGrowthRule<BodyM, BodyS, FoliageM, FoliageS>
where
	BodyM: Material + Send + Sync + 'static,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>> + Send + Sync + 'static + Default,
	FoliageM: Material + Send + Sync + 'static,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>> + Send + Sync + 'static + Default,
{
	fn ball_render_item_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		hysteresis: &StorybookTreeChain,
		chain: &BallStickChain<StorybookTreeChain>,
	) -> Option<(JungleGrowth<BodyM, BodyS, FoliageM, FoliageS>, f32)> {
		if !should_allocate_jungle_foliage(hysteresis, chain, node_idx) {
			return None;
		}
		if hysteresis.ring_u < 0.28 {
			return None;
		}
		if !mix_seed_below_fraction(node_idx, node.position, self.growth_spawn_fraction) {
			return None;
		}

		let mut shape = JungleGrowthShape::default();
		shape.seed = (node_idx as i32)
			.wrapping_add(node.position.x.to_bits() as i32)
			.wrapping_add(node.position.y.to_bits().rotate_left(5) as i32);

		let mut growth = JungleGrowth::<BodyM, BodyS, FoliageM, FoliageS>::default();
		growth.shape = shape;
		growth.body_noise = self.body_noise;
		growth.foliage_noise = self.foliage_noise;
		growth.body_material = self.body_material.clone();
		growth.foliage_material = self.foliage_material.clone();
		Some((growth, JUNGLE_GROWTH_RADIUS_SCALE))
	}
}
