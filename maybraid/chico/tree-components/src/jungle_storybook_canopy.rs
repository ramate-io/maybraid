//! Per-node foliage payload for the Jungle Storybook Tree ([#235](https://github.com/ramate-io/maybraid/issues/235)).
//!
//! [`JungleStorybookCanopyFoliage`] is the [`RenderItem`] stored on each qualifying
//! [`BallStickNode`](chico_sbs_geometry::BallStickNode). Allocation logic lives in
//! [`chico_sbs_trees::jungle_storybook_tree::canopy`](../../sbs-trees/src/jungle_storybook_tree/canopy.rs).

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use render_item::{CascadeChunk, RenderItem};

use crate::JungleGrowth;

/// Exactly one mesh cluster per canopy graph node (mutually exclusive variants).
#[derive(Clone)]
pub enum JungleStorybookCanopyFoliage<
	InnerM,
	InnerS,
	OuterM,
	OuterS,
	BodyM,
	BodyS,
	FoliageM,
	FoliageS,
> where
	InnerM: Material,
	InnerS: Clone + Into<MeshMaterial3d<InnerM>>,
	OuterM: Material,
	OuterS: Clone + Into<MeshMaterial3d<OuterM>>,
	BodyM: Material,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>>,
	FoliageM: Material,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>>,
{
	/// Noisy inner-canopy sphere (lower ring-u, non-terminal limbs).
	InnerBall(ChicoBall<InnerM, InnerS>),
	/// Disc splay at limb tips and far along projections (outer shell).
	OuterSplay(PlaneSplay<OuterM, OuterS>),
	/// Epiphyte assembly (inner ball + frond crown + buddha-hand); replaces canopy meshes on that node.
	Growth(JungleGrowth<BodyM, BodyS, FoliageM, FoliageS>),
}

impl<InnerM, InnerS, OuterM, OuterS, BodyM, BodyS, FoliageM, FoliageS> RenderItem
	for JungleStorybookCanopyFoliage<InnerM, InnerS, OuterM, OuterS, BodyM, BodyS, FoliageM, FoliageS>
where
	InnerM: Material + Send + Sync + 'static,
	InnerS: Clone + Into<MeshMaterial3d<InnerM>> + Send + Sync + 'static,
	OuterM: Material + Send + Sync + 'static,
	OuterS: Clone + Into<MeshMaterial3d<OuterM>> + Send + Sync + 'static,
	BodyM: Material + Send + Sync + 'static,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>> + Send + Sync + 'static + Default,
	FoliageM: Material + Send + Sync + 'static,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>> + Send + Sync + 'static + Default,
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
			Self::Growth(growth) => growth.spawn_render_items(commands, cascade_chunk, transform),
		}
	}
}
