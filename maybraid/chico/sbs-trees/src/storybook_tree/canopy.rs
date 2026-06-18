//! Terminal canopy: [`PlaneSplay`] on outer and terminal joints per [RFC §3.1.7.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/01-storybook-tree/README.md).

use bevy::prelude::*;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::chain::storybook_tree::is_graph_terminal;
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::{BallStickChain, BallStickNode, StorybookTreeChain, StorybookTreePhase};

fn should_allocate_plane_splay(
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
	let is_terminal = is_graph_terminal(chain, node_idx);
	let outer = hysteresis.distance_from_anchor
		> hysteresis.outer_foliage_distance_fraction * hysteresis.projection_length;
	is_terminal || outer
}

#[derive(Clone)]
pub(crate) struct StorybookTreeLeafCanopyRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub leaf_splay: PlaneSplay<LeafM, LeafS>,
	pub leaf_radius_world: f32,
}

impl<LeafM, LeafS> BallRenderRule<PlaneSplay<LeafM, LeafS>, StorybookTreeChain>
	for StorybookTreeLeafCanopyRule<LeafM, LeafS>
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
		if !should_allocate_plane_splay(node_idx, hysteresis, chain) {
			return None;
		}
		let scale = self.leaf_radius_world / node.radius.max(1e-4);
		Some((self.leaf_splay.clone(), scale))
	}
}
