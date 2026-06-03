//! [`PlaneSplay`] at every ball-stick joint ([#236](https://github.com/ramate-io/maybraid/issues/236), RFC §3.1.7.14).

use bevy::prelude::*;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::render::mix_seed::node_mix_seed;
use chico_sbs_geometry::{BallStickChain, BallStickNode, FriendsConiferChain};

/// RFC needle-cluster radius as a fraction of stalk height (`0.018 * H`).
pub const FRIENDS_SPLAY_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.018;

#[derive(Clone)]
pub(crate) struct FriendsConiferCanopyRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub leaf_splay: PlaneSplay<LeafM, LeafS>,
	pub splay_radius_world: f32,
}

impl<LeafM, LeafS> BallRenderRule<PlaneSplay<LeafM, LeafS>, FriendsConiferChain>
	for FriendsConiferCanopyRule<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		_hysteresis: &FriendsConiferChain,
		_chain: &BallStickChain<FriendsConiferChain>,
	) -> Option<(PlaneSplay<LeafM, LeafS>, f32)> {
		let mut splay = self.leaf_splay.clone();
		let t = (node_mix_seed(node_idx, node.position) as f32) / (u32::MAX as f32);
		// RFC `splay_count` 2..4 → vary icosphere subdivision for needle density.
		splay.icosphere_subdivisions = if t < 0.33 { 0 } else if t < 0.66 { 1 } else { 0 };
		let scale = self.splay_radius_world / node.radius.max(1e-4);
		Some((splay, scale))
	}
}
