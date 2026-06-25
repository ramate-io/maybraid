//! Vase canopy: inner balls and outer splays on upper / outer nodes ([#246](https://github.com/ramate-io/maybraid/issues/246)).

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::chain::storybook_tree::is_graph_terminal;
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::{BallStickChain, BallStickNode, StorybookTreeChain, StorybookTreePhase};
use chico_tree_components::BraidOakCanopyFoliage;

fn qualifies_for_foliage(
	node_idx: usize,
	hysteresis: &StorybookTreeChain,
	chain: &BallStickChain<StorybookTreeChain>,
	upper_foliage_ring_u: f32,
) -> bool {
	if hysteresis.projection_length < 1e-6 {
		return false;
	}
	if !matches!(hysteresis.phase, StorybookTreePhase::BranchOut(_)) {
		return false;
	}
	is_graph_terminal(chain, node_idx)
		|| hysteresis.ring_u > upper_foliage_ring_u
		|| hysteresis.distance_from_anchor
			> hysteresis.outer_foliage_distance_fraction * hysteresis.projection_length.max(1e-6)
}

fn is_outer_foliage(
	node_idx: usize,
	hysteresis: &StorybookTreeChain,
	chain: &BallStickChain<StorybookTreeChain>,
) -> bool {
	is_graph_terminal(chain, node_idx)
		|| hysteresis.distance_from_anchor
			> hysteresis.outer_foliage_distance_fraction * hysteresis.projection_length.max(1e-6)
}

#[derive(Clone)]
pub(crate) struct VaseTreeFoliageRule<InnerM, InnerS, OuterM, OuterS>
where
	InnerM: Material,
	InnerS: Clone + Into<MeshMaterial3d<InnerM>>,
	OuterM: Material,
	OuterS: Clone + Into<MeshMaterial3d<OuterM>>,
{
	pub inner_ball: ChicoBall<InnerM, InnerS>,
	pub outer_splay: PlaneSplay<OuterM, OuterS>,
	pub leaf_radius_world: f32,
	pub upper_foliage_ring_u: f32,
}

impl<InnerM, InnerS, OuterM, OuterS>
	BallRenderRule<BraidOakCanopyFoliage<InnerM, InnerS, OuterM, OuterS>, StorybookTreeChain>
	for VaseTreeFoliageRule<InnerM, InnerS, OuterM, OuterS>
where
	InnerM: Material + Send + Sync + 'static,
	InnerS: Clone + Into<MeshMaterial3d<InnerM>> + Send + Sync + 'static,
	OuterM: Material + Send + Sync + 'static,
	OuterS: Clone + Into<MeshMaterial3d<OuterM>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		hysteresis: &StorybookTreeChain,
		chain: &BallStickChain<StorybookTreeChain>,
	) -> Option<(BraidOakCanopyFoliage<InnerM, InnerS, OuterM, OuterS>, f32)> {
		if !qualifies_for_foliage(node_idx, hysteresis, chain, self.upper_foliage_ring_u) {
			return None;
		}
		let scale = self.leaf_radius_world / node.radius.max(1e-4);
		if is_outer_foliage(node_idx, hysteresis, chain) {
			Some((BraidOakCanopyFoliage::OuterSplay(self.outer_splay.clone()), scale))
		} else {
			Some((BraidOakCanopyFoliage::InnerBall(self.inner_ball.clone()), scale))
		}
	}
}
