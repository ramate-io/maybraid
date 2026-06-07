//! **Jungle Storybook Tree** — dense Storybook construction ([#235](https://github.com/ramate-io/maybraid/issues/235), [RFC §3.1.7.13](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/13-jungle-storybook-tree/README.md)).

//! Same [`StorybookTreeChain`] geometry as [#230](https://github.com/ramate-io/maybraid/issues/230); layered canopy foliage and [`JungleGrowth`](chico_tree_components::JungleGrowth) clusters (no separate joint tufts).

mod canopy;
pub mod render_item_plugin;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::render::ball::BallRenderHelper;
use chico_sbs_geometry::render::stick::StickRenderHelper;
use chico_sbs_geometry::{BallStickChain, JungleStorybookTreeSbs, StorybookTreeChain};
use clap::Args;
use chico_tree_components::{SkippedBodyMeshMaterial, SkippedFoliageMeshMaterial};
use procedural_common::noise_params_from_scalar_str;
use procedural_common::NoiseParams;
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::{
	SkippedInnerLeafMeshMaterial, SkippedOuterLeafMeshMaterial, SkippedStickMeshMaterial,
};
use crate::storybook_tree::stick::StorybookTreeStickRule;
use canopy::JungleStorybookFoliageRule;

/// Typical [`StandardMaterial`] jungle storybook with CLI-skipped handles.
pub type JungleStorybookTreeStd = JungleStorybookTree<
	StandardMaterial,
	SkippedStickMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedInnerLeafMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedOuterLeafMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedBodyMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedFoliageMeshMaterial<StandardMaterial>,
>;

/// Render-time knobs not encoded in SBS geometry.
#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct JungleStorybookConstructionParams {
	/// Share of foliage-eligible nodes that spawn [`JungleGrowth`](chico_tree_components::JungleGrowth) instead of inner/outer canopy meshes.
	#[arg(long, default_value_t = 0.65)]
	pub growth_spawn_fraction: f32,
}

impl Default for JungleStorybookConstructionParams {
	fn default() -> Self {
		Self { growth_spawn_fraction: 0.65 }
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct JungleStorybookTree<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS, BodyM, BodyS, FoliageM, FoliageS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	InnerLeafM: Material,
	InnerLeafS: Clone + Into<MeshMaterial3d<InnerLeafM>> + Args,
	OuterLeafM: Material,
	OuterLeafS: Clone + Into<MeshMaterial3d<OuterLeafM>> + Args,
	BodyM: Material,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>> + Args,
	FoliageM: Material,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>> + Args,
{
	/// Flattened [`StorybookTreeSbs`] (clap defaults are storybook, not jungle). [`Self::geometry_for_render`] calls [`JungleStorybookTreeSbs::apply_jungle_preset`].
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: JungleStorybookTreeSbs,

	#[command(flatten, next_help_heading = "Construction")]
	pub construction: JungleStorybookConstructionParams,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	#[command(flatten, next_help_heading = "Inner Leaf Material")]
	pub inner_leaf_material: InnerLeafS,

	#[command(flatten, next_help_heading = "Outer Leaf Material")]
	pub outer_leaf_material: OuterLeafS,

	#[command(flatten, next_help_heading = "Growth Body Material")]
	pub growth_body_material: BodyS,

	#[command(flatten, next_help_heading = "Growth Foliage Material")]
	pub growth_foliage_material: FoliageS,

	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Trunk Surface Noise"
	)]
	pub stick_surface_noise: NoiseParams,

	#[arg(
		long,
		default_value = "0,1,0.06,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Inner Leaf Surface Noise"
	)]
	pub inner_leaf_surface_noise: NoiseParams,

	#[arg(
		long,
		default_value = "0,1,0.06,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Outer Leaf Surface Noise"
	)]
	pub outer_leaf_surface_noise: NoiseParams,

	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Growth Body Noise"
	)]
	pub growth_body_noise: NoiseParams,

	#[arg(
		long,
		default_value = "0,1,0.06,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Growth Foliage Noise"
	)]
	pub growth_foliage_noise: NoiseParams,

	#[arg(skip)]
	__marker: PhantomData<(fn() -> StickM, fn() -> InnerLeafM, fn() -> OuterLeafM, fn() -> BodyM, fn() -> FoliageM)>,
}

impl<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS, BodyM, BodyS, FoliageM, FoliageS>
	Default for JungleStorybookTree<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS, BodyM, BodyS, FoliageM, FoliageS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Default,
	InnerLeafM: Material,
	InnerLeafS: Clone + Into<MeshMaterial3d<InnerLeafM>> + Args + Default,
	OuterLeafM: Material,
	OuterLeafS: Clone + Into<MeshMaterial3d<OuterLeafM>> + Args + Default,
	BodyM: Material,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>> + Args + Default,
	FoliageM: Material,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>> + Args + Default,
{
	fn default() -> Self {
		Self {
			geometry: JungleStorybookTreeSbs::default(),
			construction: JungleStorybookConstructionParams::default(),
			stick_material: StickS::default(),
			inner_leaf_material: InnerLeafS::default(),
			outer_leaf_material: OuterLeafS::default(),
			growth_body_material: BodyS::default(),
			growth_foliage_material: FoliageS::default(),
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			inner_leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			outer_leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			growth_body_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			growth_foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS, BodyM, BodyS, FoliageM, FoliageS>
	JungleStorybookTree<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS, BodyM, BodyS, FoliageM, FoliageS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	InnerLeafM: Material,
	InnerLeafS: Clone + Into<MeshMaterial3d<InnerLeafM>> + Args,
	OuterLeafM: Material,
	OuterLeafS: Clone + Into<MeshMaterial3d<OuterLeafM>> + Args,
	BodyM: Material,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>> + Args,
	FoliageM: Material,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>> + Args,
{
	/// SBS snapshot with jungle preset applied (safe to call after CLI parse).
	pub fn geometry_for_render(&self) -> JungleStorybookTreeSbs {
		let mut geometry = self.geometry.clone();
		geometry.apply_jungle_preset();
		geometry
	}

	pub fn build_chain(&self) -> BallStickChain<StorybookTreeChain> {
		self.geometry_for_render().build_chain()
	}
}

impl<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS, BodyM, BodyS, FoliageM, FoliageS> RenderItem
	for JungleStorybookTree<StickM, StickS, InnerLeafM, InnerLeafS, OuterLeafM, OuterLeafS, BodyM, BodyS, FoliageM, FoliageS>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
	InnerLeafM: Material + Send + Sync + 'static,
	InnerLeafS: Clone + Into<MeshMaterial3d<InnerLeafM>> + Args + Send + Sync + 'static + Default,
	OuterLeafM: Material + Send + Sync + 'static,
	OuterLeafS: Clone + Into<MeshMaterial3d<OuterLeafM>> + Args + Send + Sync + 'static + Default,
	BodyM: Material + Send + Sync + 'static,
	BodyS: Clone + Into<MeshMaterial3d<BodyM>> + Args + Send + Sync + 'static + Default,
	FoliageM: Material + Send + Sync + 'static,
	FoliageS: Clone + Into<MeshMaterial3d<FoliageM>> + Args + Send + Sync + 'static + Default,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let geometry = self.geometry_for_render();
		let chain = geometry.build_chain();
		let leaf_radius = geometry.leaf_radius_world();

		let stick_rule = StorybookTreeStickRule::<StickM, StickS> {
			surface_noise: self.stick_surface_noise,
			stick_material: self.stick_material.clone(),
			__marker: PhantomData,
		};

		let mut out =
			StickRenderHelper::new(chain.clone(), stick_rule).spawn_render_items(commands, cascade_chunk, transform);

		let mut inner_ball = self.inner_leaf_surface_noise.build_scalar::<ChicoBall<InnerLeafM, InnerLeafS>>();
		inner_ball.material = self.inner_leaf_material.clone();
		let mut outer_splay = PlaneSplay::<OuterLeafM, OuterLeafS>::default();
		outer_splay.material = self.outer_leaf_material.clone();
		let foliage_rule = JungleStorybookFoliageRule {
			growth_spawn_fraction: self.construction.growth_spawn_fraction,
			inner_ball,
			outer_splay,
			leaf_radius_world: leaf_radius,
			body_noise: self.growth_body_noise,
			foliage_noise: self.growth_foliage_noise,
			body_material: self.growth_body_material.clone(),
			foliage_material: self.growth_foliage_material.clone(),
			__marker: PhantomData,
		};
		out.extend(BallRenderHelper::new(chain, foliage_rule).spawn_render_items(
			commands,
			cascade_chunk,
			transform,
		));

		out
	}
}
