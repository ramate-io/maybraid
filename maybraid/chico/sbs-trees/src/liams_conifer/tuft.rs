use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::tuft::ChicoTuft;
use chico_sbs_geometry::render::tuft::{tuft_mix_seed, tuft_transform_at_joint, TuftRenderRule};
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
		_chain: &BallStickChain<LiamsConiferChain>,
	) -> Vec<(ChicoTuft<LeafM, LeafS>, Transform)> {
		let seed = tuft_mix_seed(node_idx, node.position) as i32;
		let spear_count = 6 + (tuft_mix_seed(node_idx, node.position) % 3) as u32;

		let mut tuft = self.leaf_surface_noise.with_seed(seed).build_scalar::<ChicoTuft<LeafM, LeafS>>();
		tuft.spear_count = spear_count;
		tuft.material = self.leaf_material.clone();

		vec![(
			tuft,
			tuft_transform_at_joint(node, self.tuft_world_scale),
		)]
	}
}
