use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::{BallStickChain, BallStickNode, SopesBanyanChain};

#[derive(Clone)]
pub(crate) struct SopesBanyanJointBallRule<StickM, StickS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>>,
{
	pub joint_ball: ChicoBall<StickM, StickS>,
}

impl<StickM, StickS> BallRenderRule<ChicoBall<StickM, StickS>, SopesBanyanChain>
	for SopesBanyanJointBallRule<StickM, StickS>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		node_idx: usize,
		_node: &BallStickNode,
		_hysteresis: &SopesBanyanChain,
		chain: &BallStickChain<SopesBanyanChain>,
	) -> Option<(ChicoBall<StickM, StickS>, f32)> {
		if chain.children[node_idx].is_empty() {
			return None;
		}
		Some((self.joint_ball.clone(), 1.0))
	}
}
