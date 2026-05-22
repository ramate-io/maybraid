//! **Liam's Conifer** — sparse dry conifer assembly for Chico ([RFC-183 §3.1.7.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/02-liam-s-conifer/README.md), [#244](https://github.com/ramate-io/maybraid/issues/244)).
//!
//! # Intent
//!
//! Narrow vertical stalk plus dense anchor rings and three-segment sparse canopy chains from [`chico_sbs_geometry`](chico_sbs_geometry). Segment meshes use [`ChicoStick`](chico_stick_components::chico_stick::ChicoStick). [Tufts](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/02-ball-components/06-tufts/README.md) at ball-stick joints land in a follow-up once `chico-ball-components` exposes them.
//!
//! # Rendering split (current)
//!
//! - **Stick material** — all graph segments.
//! - **Tuft material** — (planned) every joint via [`LiamsConiferSbs::tuft_world_scale`].

mod stick;
pub mod render_item_plugin;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::render::stick::StickRenderHelper;
use chico_sbs_geometry::{BallStickChain, LiamsConiferChain, LiamsConiferSbs};
use clap::Args;
use procedural_common::noise_params_from_scalar_str;
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::SkippedMeshMaterial;
use stick::LiamsConiferStickRule;

/// Typical [`StandardMaterial`] tree using CLI-skipped handles for bark (`stick_material`).
pub type LiamsConiferStd = LiamsConifer<
	StandardMaterial,
	SkippedMeshMaterial<StandardMaterial>,
>;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct LiamsConifer<StickM, StickS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
{
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: LiamsConiferSbs,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Surface Noise"
	)]
	pub stick_surface_noise: NoiseParams,

	#[arg(skip)]
	__marker: PhantomData<fn() -> StickM>,
}

impl<StickM, StickS> Default for LiamsConifer<StickM, StickS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Default,
{
	fn default() -> Self {
		Self {
			geometry: LiamsConiferSbs::default(),
			stick_material: StickS::default(),
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS> LiamsConifer<StickM, StickS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
{
	pub fn build_chain(&self) -> BallStickChain<LiamsConiferChain> {
		self.geometry.build_chain()
	}
}

impl<StickM, StickS> RenderItem for LiamsConifer<StickM, StickS>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Send + Sync + 'static + Default,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let chain = self.build_chain();

		let stick_rule = LiamsConiferStickRule::<StickM, StickS> {
			surface_noise: self.stick_surface_noise,
			stick_material: self.stick_material.clone(),
			__marker: PhantomData,
		};

		StickRenderHelper::new(chain, stick_rule).spawn_render_items(commands, cascade_chunk, transform)
	}
}
