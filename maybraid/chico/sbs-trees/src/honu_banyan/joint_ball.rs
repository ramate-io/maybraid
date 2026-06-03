use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::{BallStickChain, BallStickNode, HonuBanyanChain};

#[derive(Clone)]
pub(crate) struct HonuBanyanJointBallRule<StickM, StickS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>>,
{
	pub joint_ball: ChicoBall<StickM, StickS>,
}

impl<StickM, StickS> BallRenderRule<ChicoBall<StickM, StickS>, HonuBanyanChain>
	for HonuBanyanJointBallRule<StickM, StickS>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		node_idx: usize,
		_node: &BallStickNode,
		_hysteresis: &HonuBanyanChain,
		chain: &BallStickChain<HonuBanyanChain>,
	) -> Option<(ChicoBall<StickM, StickS>, f32)> {
		if chain.children[node_idx].is_empty() {
			return None;
		}
		Some((self.joint_ball.clone(), 1.0))
	}
}
