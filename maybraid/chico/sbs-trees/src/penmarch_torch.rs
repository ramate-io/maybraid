//! **Penmarch Torch** — vase-profile upward flame tree ([#248](https://github.com/ramate-io/maybraid/issues/248), [RFC §3.1.7.4](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/04-penmarch-torch/README.md)).

mod canopy;
pub mod render_item_plugin;
mod stick;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::render::ball::BallRenderHelper;
use chico_sbs_geometry::render::stick::StickRenderHelper;
use chico_sbs_geometry::{BallStickChain, PenmarchTorchChain, PenmarchTorchSbs};
use clap::Args;
use procedural_common::noise_params_from_scalar_str;
use procedural_common::NoiseParams;
use render_item::{CascadeChunk, RenderItem};

use crate::layered_canopy::LayeredTerminalCanopy;
use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};
use canopy::PenmarchTorchLeafCanopyRule;
use stick::PenmarchTorchStickRule;

pub type PenmarchTorchStd = PenmarchTorch<
	StandardMaterial,
	SkippedStickMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
>;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct PenmarchTorch<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: PenmarchTorchSbs,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Surface Noise"
	)]
	pub stick_surface_noise: NoiseParams,

	#[arg(
		long,
		default_value = "0,1,0.06,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Surface Noise"
	)]
	pub leaf_surface_noise: NoiseParams,

	#[arg(skip)]
	__marker: PhantomData<(fn() -> StickM, fn() -> LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS> Default for PenmarchTorch<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Default,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Default,
{
	fn default() -> Self {
		Self {
			geometry: PenmarchTorchSbs::default(),
			stick_material: StickS::default(),
			leaf_material: LeafS::default(),
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, LeafM, LeafS> PenmarchTorch<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	pub fn build_chain(&self) -> BallStickChain<PenmarchTorchChain> {
		self.geometry.build_chain()
	}
}

impl<StickM, StickS, LeafM, LeafS> RenderItem for PenmarchTorch<StickM, StickS, LeafM, LeafS>
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
		let stick_rule = PenmarchTorchStickRule::<StickM, StickS> {
			surface_noise: self.stick_surface_noise,
			stick_material: self.stick_material.clone(),
			__marker: PhantomData,
		};

		let mut out =
			StickRenderHelper::new(chain.clone(), stick_rule).spawn_render_items(commands, cascade_chunk, transform);

		let mut leaf_ball = self.leaf_surface_noise.build_scalar::<ChicoBall<LeafM, LeafS>>();
		leaf_ball.material = self.leaf_material.clone();
		let mut leaf_splay = PlaneSplay::<LeafM, LeafS>::default();
		leaf_splay.core_radius = 0.75;
		leaf_splay.leaf_disc_radius = 0.95;
		leaf_splay.icosphere_subdivisions = 1;
		leaf_splay.material = self.leaf_material.clone();
		let leaf_rule = PenmarchTorchLeafCanopyRule {
			canopy: LayeredTerminalCanopy::new(leaf_ball, leaf_splay),
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
