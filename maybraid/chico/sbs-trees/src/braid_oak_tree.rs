//! **Braid Oak Tree** — gnarled broadleaf with crook-cylinder branches ([#234](https://github.com/ramate-io/maybraid/issues/234), [RFC §3.1.7.13](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/13-braid-oak/README.md)).

mod canopy;
pub mod render_item_plugin;
pub(crate) mod joint_ball;
pub(crate) mod stick;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::render::ball::BallRenderHelper;
use chico_sbs_geometry::render::stick::StickRenderHelper;
use chico_sbs_geometry::{BallStickChain, BraidOakTreeSbs, StorybookTreeChain};
use clap::Args;
use procedural_common::noise_params_from_scalar_str;
use procedural_common::NoiseParams;
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::{
	SkippedInnerLeafMeshMaterial, SkippedOuterLeafMeshMaterial, SkippedStickMeshMaterial,
};
use canopy::BraidOakFoliageRule;
use joint_ball::BraidOakJointBallRule;
use stick::BraidOakTreeStickRule;

/// Typical [`StandardMaterial`] braid oak with CLI-skipped handles.
pub type BraidOakTreeStd = BraidOakTree<
	StandardMaterial,
	SkippedStickMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedInnerLeafMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedOuterLeafMeshMaterial<StandardMaterial>,
>;

#[derive(Component, Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct BraidOakTree<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	InnerLeafM: Material,
	InnerLeafS: Clone + Into<MeshMaterial3d<InnerLeafM>> + Args,
	OuterLeafM: Material,
	OuterLeafS: Clone + Into<MeshMaterial3d<OuterLeafM>> + Args,
{
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: BraidOakTreeSbs,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Stick Surface Noise"
	)]
	pub stick_surface_noise: NoiseParams,

	#[command(flatten, next_help_heading = "Inner Leaf Material")]
	pub inner_leaf_material: InnerLeafS,

	#[command(flatten, next_help_heading = "Outer Leaf Material")]
	pub outer_leaf_material: OuterLeafS,

	#[arg(
		long,
		default_value = "0,1,0.06,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Inner Leaf Surface Noise"
	)]
	pub inner_leaf_surface_noise: NoiseParams,

	#[arg(skip)]
	__marker: PhantomData<(fn() -> StickM, fn() -> InnerLeafM, fn() -> OuterLeafM)>,
}

impl<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS> Default
	for BraidOakTree<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Default,
	InnerLeafM: Material,
	InnerLeafS: Clone + Into<MeshMaterial3d<InnerLeafM>> + Args + Default,
	OuterLeafM: Material,
	OuterLeafS: Clone + Into<MeshMaterial3d<OuterLeafM>> + Args + Default,
{
	fn default() -> Self {
		Self {
			geometry: BraidOakTreeSbs::default(),
			stick_material: StickS::default(),
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			inner_leaf_material: InnerLeafS::default(),
			outer_leaf_material: OuterLeafS::default(),
			inner_leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS>
	BraidOakTree<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	InnerLeafM: Material,
	InnerLeafS: Clone + Into<MeshMaterial3d<InnerLeafM>> + Args,
	OuterLeafM: Material,
	OuterLeafS: Clone + Into<MeshMaterial3d<OuterLeafM>> + Args,
{
	pub fn geometry_for_render(&self) -> BraidOakTreeSbs {
		let mut geometry = self.geometry.clone();
		geometry.apply_braid_preset();
		geometry
	}

	pub fn build_chain(&self) -> BallStickChain<StorybookTreeChain> {
		self.geometry_for_render().build_chain()
	}
}

impl<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS> RenderItem
	for BraidOakTree<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
	InnerLeafM: Material + Send + Sync + 'static,
	InnerLeafS: Clone + Into<MeshMaterial3d<InnerLeafM>> + Args + Send + Sync + 'static + Default,
	OuterLeafM: Material + Send + Sync + 'static,
	OuterLeafS: Clone + Into<MeshMaterial3d<OuterLeafM>> + Args + Send + Sync + 'static + Default,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let root = commands
			.spawn((self.clone(), cascade_chunk.clone(), transform, Visibility::default()))
			.id();
		let geometry = self.geometry_for_render();
		let chain = geometry.build_chain();
		let leaf_radius = geometry.leaf_radius_world();

		let stick_rule = BraidOakTreeStickRule::<StickM, StickS> {
			stick_surface_noise: self.stick_surface_noise,
			stick_material: self.stick_material.clone(),
			__marker: PhantomData,
		};

		StickRenderHelper::new(chain.clone(), stick_rule).spawn_render_items_under(
			commands,
			cascade_chunk,
			Transform::IDENTITY,
			Some(root),
		);

		let mut joint_ball = NoiseParams::from_scalar(0.0, 1.0, 0.04, 1)
			.build_scalar::<ChicoBall<StickM, StickS>>();
		joint_ball.material = self.stick_material.clone();
		let joint_rule = BraidOakJointBallRule { joint_ball };
		BallRenderHelper::new(chain.clone(), joint_rule).spawn_render_items_under(
			commands,
			cascade_chunk,
			Transform::IDENTITY,
			Some(root),
		);

		let mut inner_ball =
			self.inner_leaf_surface_noise.build_scalar::<chico_ball_components::chico_ball::ChicoBall<
				InnerLeafM,
				InnerLeafS,
			>>();
		inner_ball.material = self.inner_leaf_material.clone();
		let mut outer_splay = PlaneSplay::<OuterLeafM, OuterLeafS>::default();
		outer_splay.material = self.outer_leaf_material.clone();

		let foliage_rule = BraidOakFoliageRule {
			inner_ball,
			outer_splay,
			leaf_radius_world: leaf_radius,
		};

		BallRenderHelper::new(chain, foliage_rule).spawn_render_items_under(
			commands,
			cascade_chunk,
			Transform::IDENTITY,
			Some(root),
		);

		vec![root]
	}
}
