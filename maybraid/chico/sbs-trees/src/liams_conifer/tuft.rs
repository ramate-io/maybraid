use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::tuft::ChicoTuft;
use chico_sbs_geometry::render::tuft::{
	joint_branch_axis, tuft_mix_seed, tuft_transforms_at_joint, TuftRenderRule,
};
use chico_sbs_geometry::{BallStickChain, BallStickNode, LiamsConiferChain};
use procedural_common::NoiseParams;

#[derive(Clone)]
pub(crate) struct LiamsConiferTuftRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub tuft_world_scale: f32,
	pub leaf_surface_noise: NoiseParams,
	pub leaf_material: LeafS,
	pub(crate) __marker: PhantomData<fn() -> LeafM>,
}

impl<LeafM, LeafS> TuftRenderRule<ChicoTuft<LeafM, LeafS>, LiamsConiferChain>
	for LiamsConiferTuftRule<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static + Default,
{
	fn tuft_placements_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		_hysteresis: &LiamsConiferChain,
		chain: &BallStickChain<LiamsConiferChain>,
	) -> Vec<(ChicoTuft<LeafM, LeafS>, Transform)> {
		let axis = joint_branch_axis(chain, node_idx);
		let transforms = tuft_transforms_at_joint(node, axis, self.tuft_world_scale, node_idx);
		let seed = tuft_mix_seed(node_idx, node.position) as i32;

		transforms
			.into_iter()
			.enumerate()
			.map(|(i, transform)| {
				let mut tuft = self
					.leaf_surface_noise
					.with_seed(seed.wrapping_add(i as i32))
					.build_scalar::<ChicoTuft<LeafM, LeafS>>();
				tuft.material = self.leaf_material.clone();
				(tuft, transform)
			})
			.collect()
	}
}
