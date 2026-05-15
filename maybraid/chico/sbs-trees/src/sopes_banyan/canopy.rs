//! Terminal canopy: mix **[Noisy Ball](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/02-noisy-ball/README.md)** and **[Plane Splay](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/05-plane-splay/README.md)** per [Sope's Banyan §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md).

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::render::ball::BallRenderRule;
use chico_sbs_geometry::{BallStickChain, BallStickNode, SopesBanyanChain, SopesBanyanPhase};
use render_item::{CascadeChunk, RenderItem};

/// Stable mixing key from graph index and node position.
fn canopy_mix_seed(node_idx: usize, position: Vec3) -> u32 {
	(node_idx as u32)
		.wrapping_mul(0x9E37_79B9)
		.wrapping_add(position.x.to_bits())
		.wrapping_add(position.y.to_bits().rotate_left(3))
		.wrapping_add(position.z.to_bits().rotate_left(7))
}

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
	let seed = canopy_mix_seed(node_idx, node.position);
	if descender_leaning {
		seed % 13 < 2
	} else {
		seed % 10 < 5
	}
}

/// One terminal canopy render item: volumetric noisy ball or radial plane splay blades.
#[derive(Clone, Debug)]
pub(crate) enum SopesBanyanTerminalCanopy<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	Ball(ChicoBall<LeafM, LeafS>),
	PlaneSplay(PlaneSplay<LeafM, LeafS>),
}

impl<LeafM, LeafS> RenderItem for SopesBanyanTerminalCanopy<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		match self {
			Self::Ball(b) => b.spawn_render_items(commands, cascade_chunk, transform),
			Self::PlaneSplay(p) => p.spawn_render_items(commands, cascade_chunk, transform),
		}
	}
}

#[derive(Clone)]
pub(crate) struct SopesBanyanLeafCanopyRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	pub leaf_ball: ChicoBall<LeafM, LeafS>,
	pub leaf_splay: PlaneSplay<LeafM, LeafS>,
	pub min_height: f32,
	/// World-space canopy radius numerator (uniform scale = this / [`BallStickNode::radius`]).
	pub leaf_radius_world: f32,
}

impl<LeafM, LeafS> BallRenderRule<SopesBanyanTerminalCanopy<LeafM, LeafS>, SopesBanyanChain>
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
	) -> Option<(SopesBanyanTerminalCanopy<LeafM, LeafS>, f32)> {
		if node.position.y < self.min_height {
			return None;
		}

		let scale = self.leaf_radius_world / node.radius;

		if canopy_prefers_plane_splay(node_idx, node, hysteresis) {
			let mut splay = self.leaf_splay.clone();
			let seed = canopy_mix_seed(node_idx, node.position);
			splay.icosphere_subdivisions = seed % 2;
			splay.leaf_disc_radius = 0.18 + 0.12 * ((seed % 17) as f32 / 16.0);
			Some((SopesBanyanTerminalCanopy::PlaneSplay(splay), scale))
		} else {
			Some((SopesBanyanTerminalCanopy::Ball(self.leaf_ball.clone()), scale))
		}
	}
}
