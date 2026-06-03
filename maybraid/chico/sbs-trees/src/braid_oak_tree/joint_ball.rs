use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::{BallStickChain, BallStickNode, StorybookTreeChain};

/// Stick-material [`ChicoBall`] at every graph node to hide crook-cylinder segment gaps.
#[derive(Clone)]
pub(crate) struct BraidOakJointBallRule<StickM, StickS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>>,
{
	pub joint_ball: ChicoBall<StickM, StickS>,
}

impl<StickM, StickS> BallRenderRule<ChicoBall<StickM, StickS>, StorybookTreeChain>
	for BraidOakJointBallRule<StickM, StickS>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		_node_idx: usize,
		_node: &BallStickNode,
		_hysteresis: &StorybookTreeChain,
		_chain: &BallStickChain<StorybookTreeChain>,
	) -> Option<(ChicoBall<StickM, StickS>, f32)> {
		Some((self.joint_ball.clone(), 0.88))
	}
}
