//! Foliage allocation for radial shoot joints ([#233](https://github.com/ramate-io/maybraid/issues/233)).

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_ball_components::tuft::SucculentTuft;
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::render::tuft::{tuft_mix_seed, tuft_transform_at_joint, TuftRenderRule};
use chico_sbs_geometry::{
	high_bush_is_graph_terminal, BallStickChain, BallStickNode, HighBushChain, HighBushPhase,
};
use procedural_common::NoiseParams;

/// RFC §3.1.7.12 ball selection for Common High Bush.
pub fn should_allocate_foliage(
	node_idx: usize,
	hysteresis: &HighBushChain,
	chain: &BallStickChain<HighBushChain>,
) -> bool {
	if matches!(hysteresis.phase, HighBushPhase::Root { .. }) {
		return false;
	}
	high_bush_is_graph_terminal(chain, node_idx)
		|| hysteresis.height_fraction() > 0.45
		|| hysteresis.branch_order() > 1
}

#[derive(Clone)]
pub(crate) struct HighBushSplayCanopyRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub leaf_splay: PlaneSplay<LeafM, LeafS>,
	pub leaf_radius_world: f32,
}

impl<LeafM, LeafS> BallRenderRule<PlaneSplay<LeafM, LeafS>, HighBushChain>
	for HighBushSplayCanopyRule<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		hysteresis: &HighBushChain,
		chain: &BallStickChain<HighBushChain>,
	) -> Option<(PlaneSplay<LeafM, LeafS>, f32)> {
		if !should_allocate_foliage(node_idx, hysteresis, chain) {
			return None;
		}
		let scale = self.leaf_radius_world / node.radius.max(1e-4);
		Some((self.leaf_splay.clone(), scale))
	}
}

#[derive(Clone)]
pub(crate) struct HighBushTuftCanopyRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub tuft_world_scale: f32,
	pub leaf_surface_noise: NoiseParams,
	pub leaf_material: LeafS,
	pub(crate) __marker: PhantomData<fn() -> LeafM>,
}

impl<LeafM, LeafS> TuftRenderRule<SucculentTuft<LeafM, LeafS>, HighBushChain>
	for HighBushTuftCanopyRule<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static + Default,
{
	fn tuft_placements_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		hysteresis: &HighBushChain,
		chain: &BallStickChain<HighBushChain>,
	) -> Vec<(SucculentTuft<LeafM, LeafS>, Transform)> {
		if !should_allocate_foliage(node_idx, hysteresis, chain) {
			return Vec::new();
		}
		let seed = tuft_mix_seed(node_idx, node.position) as i32;
		let element_count = 6 + (tuft_mix_seed(node_idx, node.position) % 3) as u32;
		let mut tuft = self
			.leaf_surface_noise
			.with_seed(seed)
			.build_scalar::<SucculentTuft<LeafM, LeafS>>();
		tuft.shape.element_count = element_count;
		tuft.material = self.leaf_material.clone();
		vec![(tuft, tuft_transform_at_joint(node, self.tuft_world_scale))]
	}
}
