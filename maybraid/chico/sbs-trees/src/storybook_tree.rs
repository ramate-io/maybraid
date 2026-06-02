//! **Storybook Tree** — default broadleaf ball-stick assembly ([#230](https://github.com/ramate-io/maybraid/issues/230), [RFC §3.1.7.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/01-storybook-tree/README.md)).

//! Narrow tapered stalk, dome-profile radial limbs, and [`PlaneSplay`](chico_ball_components::plane_splay::PlaneSplay) on outer/terminal joints.

mod canopy;
pub mod render_item_plugin;
mod stick;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::render::ball::BallRenderHelper;
use chico_sbs_geometry::render::stick::StickRenderHelper;
use chico_sbs_geometry::{BallStickChain, StorybookTreeChain, StorybookTreeSbs};
use clap::Args;
use procedural_common::noise_params_from_scalar_str;
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};
use canopy::StorybookTreeLeafCanopyRule;
use stick::StorybookTreeStickRule;

/// Typical [`StandardMaterial`] tree using CLI-skipped handles.
pub type StorybookTreeStd = StorybookTree<
	StandardMaterial,
	SkippedStickMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
>;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct StorybookTree<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: StorybookTreeSbs,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

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
		help_heading = "Foliage Surface Noise"
	)]
	pub leaf_surface_noise: NoiseParams,

	#[arg(skip)]
	__marker: PhantomData<(fn() -> StickM, fn() -> LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS> Default for StorybookTree<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Default,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Default,
{
	fn default() -> Self {
		Self {
			geometry: StorybookTreeSbs::default(),
			stick_material: StickS::default(),
			leaf_material: LeafS::default(),
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, LeafM, LeafS> StorybookTree<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	pub fn build_chain(&self) -> BallStickChain<StorybookTreeChain> {
		self.geometry.build_chain()
	}
}

impl<StickM, StickS, LeafM, LeafS> RenderItem for StorybookTree<StickM, StickS, LeafM, LeafS>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Send + Sync + 'static + Default,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let chain = self.build_chain();
		let stick_rule = StorybookTreeStickRule::<StickM, StickS> {
			surface_noise: self.stick_surface_noise,
			stick_material: self.stick_material.clone(),
			__marker: PhantomData,
		};

		let mut out =
			StickRenderHelper::new(chain.clone(), stick_rule).spawn_render_items(commands, cascade_chunk, transform);

		let mut leaf_splay = PlaneSplay::<LeafM, LeafS>::default();
		leaf_splay.material = self.leaf_material.clone();
		let leaf_rule = StorybookTreeLeafCanopyRule {
			leaf_splay,
			leaf_radius_world: self.geometry.leaf_radius_world(),
		};

		out.extend(BallRenderHelper::new(chain, leaf_rule).spawn_render_items(
			commands,
			cascade_chunk,
			transform,
		));

		out
	}
}
