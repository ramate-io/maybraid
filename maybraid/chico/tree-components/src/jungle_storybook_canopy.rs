//! Per-node foliage for the Jungle Storybook Tree ([#235](https://github.com/ramate-io/maybraid/issues/235)).

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use render_item::{CascadeChunk, RenderItem};

use crate::JungleGrowth;

/// Exactly one mesh cluster per canopy graph node: inner ball, outer splay, or jungle growth.
#[derive(Clone)]
pub enum JungleStorybookCanopyFoliage<InnerM, InnerS, OuterM, OuterS, BodyM, BodyS, FoliageM, FoliageS>
where
	InnerM: Material,
	InnerS: Clone + Into<MeshMaterial3d<InnerM>>,
	OuterM: Material,
	OuterS: Clone + Into<MeshMaterial3d<OuterM>>,
	BodyM: Material,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>>,
	FoliageM: Material,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>>,
{
	InnerBall(ChicoBall<InnerM, InnerS>),
	OuterSplay(PlaneSplay<OuterM, OuterS>),
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
