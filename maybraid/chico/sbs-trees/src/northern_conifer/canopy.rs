//! [`PlaneSplay`] at every ball-stick joint ([#232](https://github.com/ramate-io/maybraid/issues/232), RFC §3.1.7.11).

use bevy::prelude::*;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::render::mix_seed::{mix_seed_below_fraction, node_mix_seed};
use chico_sbs_geometry::{BallStickChain, BallStickNode, LiamsConiferChain};

/// Needle-cluster world radius as a fraction of stalk height (RFC `0.018 * H`).
pub const NORTHERN_SPLAY_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.04;

/// Local icosphere/plate sizing before joint scale (narrow needle clusters).
pub const NORTHERN_SPLAY_CORE_RADIUS: f32 = 0.75;
pub const NORTHERN_SPLAY_LEAF_DISC_RADIUS: f32 = 0.95;

/// Map RFC `splay_count` 2..4 to icosphere subdivision (`2` = 320 faces, `3` = 1280, `4` = 5120).
fn icosphere_subdivisions_for_node(node_idx: usize, position: Vec3) -> u32 {
	let t = (node_mix_seed(node_idx, position) as f32) / (u32::MAX as f32);
	if t < 0.34 {
		2
	} else if t < 0.67 {
		3
	} else {
		4
	}
}

#[derive(Clone)]
pub(crate) struct NorthernConiferCanopyRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub leaf_splay: PlaneSplay<LeafM, LeafS>,
	pub splay_radius_world: f32,
	pub splay_spawn_fraction: f32,
}

impl<LeafM, LeafS> BallRenderRule<PlaneSplay<LeafM, LeafS>, LiamsConiferChain>
	for NorthernConiferCanopyRule<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		_hysteresis: &LiamsConiferChain,
		_chain: &BallStickChain<LiamsConiferChain>,
	) -> Option<(PlaneSplay<LeafM, LeafS>, f32)> {
		if !mix_seed_below_fraction(node_idx, node.position, self.splay_spawn_fraction) {
			return None;
		}
		let mut splay = self.leaf_splay.clone();
		splay.icosphere_subdivisions = icosphere_subdivisions_for_node(node_idx, node.position);
		let scale = self.splay_radius_world / node.radius.max(1e-4);
		Some((splay, scale))
	}
}
