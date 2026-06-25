//! [`PlaneSplay`] at every ball-stick joint ([#236](https://github.com/ramate-io/maybraid/issues/236), RFC §3.1.7.14).

use bevy::prelude::*;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::render::mix_seed::node_mix_seed;
use chico_sbs_geometry::{BallStickChain, BallStickNode, FriendsConiferChain};

/// Needle-cluster world radius as a fraction of stalk height (RFC `0.018 * H`; denser than RFC for fuller Friend's silhouette).
pub const FRIENDS_SPLAY_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.028;

/// Local icosphere/plate sizing before joint scale (slightly larger plates than [`PlaneSplay`] defaults).
pub const FRIENDS_SPLAY_CORE_RADIUS: f32 = 0.85;
pub const FRIENDS_SPLAY_LEAF_DISC_RADIUS: f32 = 1.05;

/// Map RFC `splay_count` 2..4 to icosphere subdivision (`0` = 20 faces, `1` = 80, `2` = 320).
fn icosphere_subdivisions_for_node(node_idx: usize, position: Vec3) -> u32 {
	let t = (node_mix_seed(node_idx, position) as f32) / (u32::MAX as f32);
	if t < 0.25 {
		1
	} else if t < 0.55 {
		2
	} else {
		1
	}
}

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
		splay.icosphere_subdivisions = icosphere_subdivisions_for_node(node_idx, node.position);
		let scale = self.splay_radius_world / node.radius.max(1e-4);
		Some((splay, scale))
	}
}
