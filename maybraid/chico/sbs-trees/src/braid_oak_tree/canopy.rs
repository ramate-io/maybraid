//! Braid Oak foliage: legacy RenderItem inner balls and outer splay ([#234](https://github.com/ramate-io/maybraid/issues/234)).
//!
//! VegetationComponents foliage reuses [`crate::storybook_tree::canopy`]; this module is kept for
//! the RenderItem path.

#![allow(dead_code)]

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::chain::storybook_tree::{
	is_graph_terminal, StorybookTreeChain, StorybookTreePhase,
};
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::{BallStickChain, BallStickNode};
use chico_tree_components::BraidOakCanopyFoliage;

/// Inner foliage allowed once the ring is above the lower trunk belt (RFC `height_fraction > 0.35`).
const MIN_RING_U_FOR_INNER_FOLIAGE: f32 = 0.45;
/// Mid-limb nodes qualify without waiting for a terminal hop.
const MIN_BRANCH_ORDER_FOR_FOLIAGE: usize = 2;

fn qualifies_for_foliage(
	node_idx: usize,
	hysteresis: &StorybookTreeChain,
	chain: &BallStickChain<StorybookTreeChain>,
) -> bool {
	if hysteresis.projection_length < 1e-6 {
		return false;
	}
	if !matches!(hysteresis.phase, StorybookTreePhase::BranchOut(_)) {
		return false;
	}
	is_graph_terminal(chain, node_idx)
		|| hysteresis.branch_order() > MIN_BRANCH_ORDER_FOR_FOLIAGE
		|| hysteresis.ring_u > MIN_RING_U_FOR_INNER_FOLIAGE
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
pub(crate) struct BraidOakFoliageRule<InnerM, InnerS, OuterM, OuterS>
where
	InnerM: Material,
	InnerS: Clone + Into<MeshMaterial3d<InnerM>>,
	OuterM: Material,
	OuterS: Clone + Into<MeshMaterial3d<OuterM>>,
{
	pub inner_ball: ChicoBall<InnerM, InnerS>,
	pub outer_splay: PlaneSplay<OuterM, OuterS>,
	pub leaf_radius_world: f32,
}

impl<InnerM, InnerS, OuterM, OuterS>
	BallRenderRule<BraidOakCanopyFoliage<InnerM, InnerS, OuterM, OuterS>, StorybookTreeChain>
	for BraidOakFoliageRule<InnerM, InnerS, OuterM, OuterS>
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
		if !qualifies_for_foliage(node_idx, hysteresis, chain) {
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
