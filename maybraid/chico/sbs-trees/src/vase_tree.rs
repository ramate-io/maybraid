//! **Vase Tree** — upward-opening vase-profile broadleaf ([#246](https://github.com/ramate-io/maybraid/issues/246), [RFC §3.1.7.3](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/03-vase-tree/README.md)).

mod canopy;
pub mod render_item_plugin;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::render::ball::BallRenderHelper;
use chico_sbs_geometry::render::stick::StickRenderHelper;
use chico_sbs_geometry::{stalk_tip_from_chain, BallStickChain, StorybookTreeChain, VaseTreeSbs};
use clap::Args;
use procedural_common::noise_params_from_scalar_str;
use procedural_common::NoiseParams;
use render_item::{CascadeChunk, RenderItem};

use chico_sbs_geometry::DEFAULT_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT;

use crate::conifer_canopy_apex::spawn_apex_chico_ball_at_tip_with_radius;
use crate::skipped_mesh_material::{
	SkippedInnerLeafMeshMaterial, SkippedOuterLeafMeshMaterial, SkippedStickMeshMaterial,
};
use crate::storybook_tree::stick::StorybookTreeStickRule;
use canopy::VaseTreeFoliageRule;

pub type VaseTreeStd = VaseTree<
	StandardMaterial,
	SkippedStickMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedInnerLeafMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedOuterLeafMeshMaterial<StandardMaterial>,
>;

#[derive(Component, Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct VaseTree<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	InnerLeafM: Material,
	InnerLeafS: Clone + Into<MeshMaterial3d<InnerLeafM>> + Args,
	OuterLeafM: Material,
	OuterLeafS: Clone + Into<MeshMaterial3d<OuterLeafM>> + Args,
{
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: VaseTreeSbs,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	#[command(flatten, next_help_heading = "Inner Leaf Material")]
	pub inner_leaf_material: InnerLeafS,

	#[command(flatten, next_help_heading = "Outer Leaf Material")]
	pub outer_leaf_material: OuterLeafS,

	/// Crown [`ChicoBall`] world radius as a fraction of tree height `H`.
	#[arg(
		long,
		default_value_t = DEFAULT_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
		help_heading = "Foliage"
	)]
	pub apex_ball_radius_fraction_of_height: f32,

	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Trunk Surface Noise"
	)]
	pub stick_surface_noise: NoiseParams,

	#[arg(
		long,
		default_value = "0,1,0.06,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Inner Leaf Surface Noise"
	)]
	pub inner_leaf_surface_noise: NoiseParams,

	#[arg(skip)]
	__marker: PhantomData<(fn() -> StickM, fn() -> InnerLeafM, fn() -> OuterLeafM)>,
}

impl<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS> Default
	for VaseTree<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS>
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
			geometry: VaseTreeSbs::default(),
			stick_material: StickS::default(),
			inner_leaf_material: InnerLeafS::default(),
			outer_leaf_material: OuterLeafS::default(),
			apex_ball_radius_fraction_of_height: DEFAULT_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			inner_leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS>
	VaseTree<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	InnerLeafM: Material,
	InnerLeafS: Clone + Into<MeshMaterial3d<InnerLeafM>> + Args,
	OuterLeafM: Material,
	OuterLeafS: Clone + Into<MeshMaterial3d<OuterLeafM>> + Args,
{
	pub fn build_chain(&self) -> BallStickChain<StorybookTreeChain> {
		self.geometry.build_chain()
	}

	pub fn apply_bush_preset(&mut self) {
		self.geometry.apply_bush_preset();
	}
}

impl<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS> RenderItem
	for VaseTree<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS>
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
		let chain = self.build_chain();
		let stick_rule = StorybookTreeStickRule::<StickM, StickS> {
			surface_noise: self.stick_surface_noise,
			stick_material: self.stick_material.clone(),
			__marker: PhantomData,
		};

		StickRenderHelper::new(chain.clone(), stick_rule).spawn_render_items_under(
			commands,
			cascade_chunk,
			Transform::IDENTITY,
			Some(root),
		);

		let mut inner_ball = self
			.inner_leaf_surface_noise
			.build_scalar::<ChicoBall<InnerLeafM, InnerLeafS>>();
		inner_ball.material = self.inner_leaf_material.clone();
		let mut outer_splay = PlaneSplay::<OuterLeafM, OuterLeafS>::default();
		outer_splay.material = self.outer_leaf_material.clone();
		let foliage_rule = VaseTreeFoliageRule {
			inner_ball,
			outer_splay,
			leaf_radius_world: self.geometry.leaf_radius_world(),
			upper_foliage_ring_u: self.geometry.canopy.upper_foliage_ring_u,
		};

		BallRenderHelper::new(chain.clone(), foliage_rule).spawn_render_items_under(
			commands,
			cascade_chunk,
			Transform::IDENTITY,
			Some(root),
		);

		let tip = stalk_tip_from_chain(&chain);
		spawn_apex_chico_ball_at_tip_with_radius::<InnerLeafM, _>(
			self.geometry.apex_radius_world(self.apex_ball_radius_fraction_of_height),
			&tip,
			commands,
			cascade_chunk,
			root,
			&self.inner_leaf_surface_noise,
			self.inner_leaf_material.clone(),
		);

		vec![root]
	}
}
