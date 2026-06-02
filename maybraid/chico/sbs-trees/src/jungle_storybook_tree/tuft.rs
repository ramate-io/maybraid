//! Sparse succulent tuft protrusions on jungle storybook joints.

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::tuft::SucculentTuft;
use chico_sbs_geometry::chain::storybook_tree::StorybookTreeChain;
use chico_sbs_geometry::render::mix_seed::{mix_seed_below_fraction, node_mix_seed};
use chico_sbs_geometry::render::tuft::{tuft_transform_at_joint, TuftRenderRule};
use chico_sbs_geometry::{BallStickChain, BallStickNode};
use procedural_common::NoiseParams;

use super::canopy::should_allocate_jungle_foliage;

#[derive(Clone)]
pub(crate) struct JungleStorybookTuftRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub tuft_world_scale: f32,
	pub tuft_spawn_fraction: f32,
	pub leaf_surface_noise: NoiseParams,
	pub leaf_material: LeafS,
	pub(crate) __marker: PhantomData<fn() -> LeafM>,
}

impl<LeafM, LeafS> TuftRenderRule<SucculentTuft<LeafM, LeafS>, StorybookTreeChain>
	for JungleStorybookTuftRule<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static + Default,
{
	fn tuft_placements_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		hysteresis: &StorybookTreeChain,
		chain: &BallStickChain<StorybookTreeChain>,
	) -> Vec<(SucculentTuft<LeafM, LeafS>, Transform)> {
		if !should_allocate_jungle_foliage(hysteresis, chain, node_idx) {
			return Vec::new();
		}
		if !mix_seed_below_fraction(node_idx, node.position, self.tuft_spawn_fraction) {
			return Vec::new();
		}

		let seed = node_mix_seed(node_idx, node.position) as i32;
		let element_count = 5 + (node_mix_seed(node_idx, node.position) % 4) as u32;

		let mut tuft = self
			.leaf_surface_noise
			.with_seed(seed)
			.build_scalar::<SucculentTuft<LeafM, LeafS>>();
		tuft.shape.element_count = element_count;
		tuft.material = self.leaf_material.clone();

		vec![(
			tuft,
			tuft_transform_at_joint(node, self.tuft_world_scale),
		)]
	}
}
