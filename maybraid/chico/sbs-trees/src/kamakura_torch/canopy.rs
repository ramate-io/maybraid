//! Terminal canopy: compact [`PlaneSplay`] on upper/outer nodes, [`ChicoBall`] elsewhere ([Kamakura torch (stashed near-vertical flame)).

use bevy::prelude::*;
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::render::mix_seed::node_mix_seed;
use chico_sbs_geometry::{kamakura_is_graph_terminal, BallStickChain, BallStickNode, KamakuraTorchChain, StorybookTreePhase};

use crate::layered_canopy::{LayeredTerminalCanopy, LayeredTerminalCanopyItem};

/// RFC §3.1.7.4 ball selection: terminal, upper belt, or far along limb.
fn should_allocate_foliage(
	node_idx: usize,
	hysteresis: &KamakuraTorchChain,
	chain: &BallStickChain<KamakuraTorchChain>,
) -> bool {
	if hysteresis.projection_length < 1e-6 {
		return false;
	}
	if !matches!(hysteresis.phase, StorybookTreePhase::BranchOut(_)) {
		return false;
	}
	let is_terminal = kamakura_is_graph_terminal(chain, node_idx);
	let upper = hysteresis.ring_u > 0.55;
	let outer = hysteresis.distance_from_anchor
		> 0.70 * hysteresis.projection_length;
	is_terminal || upper || outer
}

fn prefers_plane_splay(hysteresis: &KamakuraTorchChain, node_idx: usize, position: Vec3) -> bool {
	if hysteresis.ring_u > 0.35 {
		return true;
	}
	let seed = node_mix_seed(node_idx, position);
	seed % 10 < 4
}

#[derive(Clone)]
pub(crate) struct KamakuraTorchLeafCanopyRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub canopy: LayeredTerminalCanopy<LeafM, LeafS>,
	pub leaf_radius_world: f32,
}

impl<LeafM, LeafS> BallRenderRule<LayeredTerminalCanopyItem<LeafM, LeafS>, KamakuraTorchChain>
	for KamakuraTorchLeafCanopyRule<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		hysteresis: &KamakuraTorchChain,
		chain: &BallStickChain<KamakuraTorchChain>,
	) -> Option<(LayeredTerminalCanopyItem<LeafM, LeafS>, f32)> {
		if !should_allocate_foliage(node_idx, hysteresis, chain) {
			return None;
		}

		let scale = self.leaf_radius_world / node.radius.max(1e-4);

		if prefers_plane_splay(hysteresis, node_idx, node.position) {
			Some((self.canopy.plane_splay_item_varied(node_idx, node.position), scale))
		} else {
			Some((self.canopy.ball_item(), scale))
		}
	}
}
