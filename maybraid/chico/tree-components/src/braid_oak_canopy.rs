//! Per-node foliage payload for the Braid Oak Tree ([#234](https://github.com/ramate-io/maybraid/issues/234)).
//!
//! Allocation logic lives in
//! [`chico_sbs_trees::braid_oak_tree::canopy`](../../sbs-trees/src/braid_oak_tree/canopy.rs).

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use render_item::{CascadeChunk, RenderItem};

/// Exactly one mesh cluster per qualifying graph node.
#[derive(Clone)]
pub enum BraidOakCanopyFoliage<InnerM, InnerS, OuterM, OuterS>
where
	InnerM: Material,
	InnerS: Clone + Into<MeshMaterial3d<InnerM>>,
	OuterM: Material,
	OuterS: Clone + Into<MeshMaterial3d<OuterM>>,
{
	InnerBall(ChicoBall<InnerM, InnerS>),
	OuterSplay(PlaneSplay<OuterM, OuterS>),
}

impl<InnerM, InnerS, OuterM, OuterS> RenderItem
	for BraidOakCanopyFoliage<InnerM, InnerS, OuterM, OuterS>
where
	InnerM: Material + Send + Sync + 'static,
	InnerS: Clone + Into<MeshMaterial3d<InnerM>> + Send + Sync + 'static,
	OuterM: Material + Send + Sync + 'static,
	OuterS: Clone + Into<MeshMaterial3d<OuterM>> + Send + Sync + 'static,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		match self {
			Self::InnerBall(ball) => ball.spawn_render_items(commands, cascade_chunk, transform),
			Self::OuterSplay(splay) => splay.spawn_render_items(commands, cascade_chunk, transform),
		}
	}
}
