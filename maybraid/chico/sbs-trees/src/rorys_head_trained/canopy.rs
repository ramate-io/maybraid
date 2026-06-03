//! Small [`ChicoBall`] canopy at every graph joint (stalk and projection limbs) ([#254](https://github.com/ramate-io/maybraid/issues/254)).

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::{BallStickChain, BallStickNode, StorybookTreeChain};

/// Slightly undersized vs node radius so crook gaps stay covered without dominating limbs.
const JOINT_CANOPY_BALL_SCALE: f32 = 0.88;

#[derive(Clone)]
pub(crate) struct RorysHeadTrainedLeafCanopyRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub leaf_ball: ChicoBall<LeafM, LeafS>,
	pub leaf_radius_world: f32,
}

impl<LeafM, LeafS> BallRenderRule<ChicoBall<LeafM, LeafS>, StorybookTreeChain>
	for RorysHeadTrainedLeafCanopyRule<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		_node_idx: usize,
		node: &BallStickNode,
		_hysteresis: &StorybookTreeChain,
		_chain: &BallStickChain<StorybookTreeChain>,
	) -> Option<(ChicoBall<LeafM, LeafS>, f32)> {
		let scale = (self.leaf_radius_world / node.radius.max(1e-4)) * JOINT_CANOPY_BALL_SCALE;
		Some((self.leaf_ball.clone(), scale))
	}
}
