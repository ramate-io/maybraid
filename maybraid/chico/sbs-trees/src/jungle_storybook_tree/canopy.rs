//! Layered jungle canopy: inner noisy balls, outer plane splays ([#235](https://github.com/ramate-io/maybraid/issues/235)).

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::chain::storybook_tree::{
	is_graph_terminal, StorybookTreeChain, StorybookTreePhase,
};
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::{BallStickChain, BallStickNode};

/// RFC §3.1.7.13: foliage throughout the canopy, not only the outer shell.
pub(crate) fn should_allocate_jungle_foliage(
	hysteresis: &StorybookTreeChain,
	chain: &BallStickChain<StorybookTreeChain>,
	node_idx: usize,
) -> bool {
	if hysteresis.projection_length < 1e-6 {
		return false;
	}
	if !matches!(hysteresis.phase, StorybookTreePhase::BranchOut(_)) {
		return false;
	}
	hysteresis.ring_u > 0.40 || is_graph_terminal(chain, node_idx) || hysteresis.branch_order() > 1
}

pub(crate) fn prefers_outer_splay(
	hysteresis: &StorybookTreeChain,
	chain: &BallStickChain<StorybookTreeChain>,
	node_idx: usize,
) -> bool {
	let is_terminal = is_graph_terminal(chain, node_idx);
	let outer = hysteresis.distance_from_anchor > 0.55 * hysteresis.projection_length.max(1e-6);
	is_terminal || outer
}

#[derive(Clone)]
pub(crate) struct JungleStorybookInnerCanopyRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub leaf_ball: ChicoBall<LeafM, LeafS>,
	pub leaf_radius_world: f32,
}

impl<LeafM, LeafS> BallRenderRule<ChicoBall<LeafM, LeafS>, StorybookTreeChain>
	for JungleStorybookInnerCanopyRule<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		hysteresis: &StorybookTreeChain,
		chain: &BallStickChain<StorybookTreeChain>,
	) -> Option<(ChicoBall<LeafM, LeafS>, f32)> {
		if !should_allocate_jungle_foliage(hysteresis, chain, node_idx) {
			return None;
		}
		if prefers_outer_splay(hysteresis, chain, node_idx) {
			return None;
		}
		let scale = self.leaf_radius_world / node.radius.max(1e-4);
		Some((self.leaf_ball.clone(), scale))
	}
}

#[derive(Clone)]
pub(crate) struct JungleStorybookOuterCanopyRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub leaf_splay: PlaneSplay<LeafM, LeafS>,
	pub leaf_radius_world: f32,
}

impl<LeafM, LeafS> BallRenderRule<PlaneSplay<LeafM, LeafS>, StorybookTreeChain>
	for JungleStorybookOuterCanopyRule<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		hysteresis: &StorybookTreeChain,
		chain: &BallStickChain<StorybookTreeChain>,
	) -> Option<(PlaneSplay<LeafM, LeafS>, f32)> {
		return None;

		if !should_allocate_jungle_foliage(hysteresis, chain, node_idx) {
			return None;
		}
		if !prefers_outer_splay(hysteresis, chain, node_idx) {
			return None;
		}
		let scale = self.leaf_radius_world / node.radius.max(1e-4);
		let mut splay = self.leaf_splay.clone();
		let seed = chico_sbs_geometry::render::mix_seed::node_mix_seed(node_idx, node.position);
		splay.icosphere_subdivisions = seed % 2;
		splay.leaf_disc_radius = 0.18 + 0.12 * ((seed % 17) as f32 / 16.0);
		Some((splay, scale))
	}
}
