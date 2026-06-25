//! Terminal canopy: mix **[Noisy Ball](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/02-noisy-ball/README.md)** and **[Plane Splay](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/05-plane-splay/README.md)** per [Sope's Banyan §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md).

use bevy::prelude::*;
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::render::mix_seed::node_mix_seed;
use chico_sbs_geometry::{BallStickChain, BallStickNode, SopesBanyanChain, SopesBanyanPhase};

use crate::layered_canopy::{LayeredTerminalCanopy, LayeredTerminalCanopyItem};

/// Prefer plane splay in the rising crown; stay mostly on noisy balls along descenders (sparse foliage).
fn canopy_prefers_plane_splay(
	node_idx: usize,
	node: &BallStickNode,
	hysteresis: &SopesBanyanChain,
) -> bool {
	let descender_leaning = matches!(
		&hysteresis.phase,
		SopesBanyanPhase::StartDescender(_) | SopesBanyanPhase::EndDescender(_)
	);
	let seed = node_mix_seed(node_idx, node.position);
	if descender_leaning {
		seed % 13 < 2
	} else {
		seed % 10 < 5
	}
}

#[derive(Clone)]
pub(crate) struct SopesBanyanLeafCanopyRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub canopy: LayeredTerminalCanopy<LeafM, LeafS>,
	pub min_height: f32,
	/// World-space canopy radius numerator (uniform scale = this / [`BallStickNode::radius`]).
	pub leaf_radius_world: f32,
}

impl<LeafM, LeafS> BallRenderRule<LayeredTerminalCanopyItem<LeafM, LeafS>, SopesBanyanChain>
	for SopesBanyanLeafCanopyRule<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		hysteresis: &SopesBanyanChain,
		_chain: &BallStickChain<SopesBanyanChain>,
	) -> Option<(LayeredTerminalCanopyItem<LeafM, LeafS>, f32)> {
		if node.position.y < self.min_height {
			return None;
		}

		let scale = self.leaf_radius_world / node.radius;

		if canopy_prefers_plane_splay(node_idx, node, hysteresis) {
			Some((self.canopy.plane_splay_item_varied(node_idx, node.position), scale))
		} else {
			Some((self.canopy.ball_item(), scale))
		}
	}
}
