//! **Sope's Banyan** — end-to-end tree assembly for Chico ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252)).
//!
//! # Intent
//!
//! Wire the vertical **vase banyan** recipe: [Banyan Trunk §3.1.6.5](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/05-banyan-trunk/README.md) stalk (`chico_sdf` / stalk height and radius fractions from the RFC), `chico-sbs-geometry` anchor rings plus [`chico_sbs_geometry::chain::sopes_banyan`](chico_sbs_geometry::chain::sopes_banyan) hysteresis, segment meshes via `chico-stick` ([noisy tapered cylinder](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/01-stick-and-stalk-components/README.md)), and canopy balls via `chico-ball` and `plane-splay` with RFC ball selection (foliage broadly in the rising crown; sparse on descenders unless tuning for denser mystique). Optional `tree-components` / jungle growth (tufts) comes later for dense variants per [§3.1.6.4 Jungle growths](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/04-jungle-growths/README.md).
//!
//! # Playground / CLI
//!
//! Everything that parameterizes height, rings, chain phases, materials, and optional fruiting ([§3.1.6.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/07-fruiting-bodies/README.md)) should be exposed **under feature flags** as **`clap`-parseable** types so a future playground can drive the same recipe as production.

pub mod render_item_plugin;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_sbs_geometry::render::ball::{BallRenderHelper, BallRenderRule};
use chico_sbs_geometry::render::stick::{StickRenderHelper, StickRenderRule};
use chico_sbs_geometry::{
	BallStickChain, BallStickNode, BallStickSegment, SopesBanyanChain, SopesBanyanSbs,
};
use chico_stick_components::chico_stick::ChicoStick;
use clap::Args;
use procedural_common::noise_params_from_scalar_str;
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::SkippedMeshMaterial;

/// Typical [`StandardMaterial`] tree using CLI-skipped material handles (defaults until real flags land).
pub type SopesBanyanStd = SopesBanyan<
	StandardMaterial,
	SkippedMeshMaterial<StandardMaterial>,
	SkippedMeshMaterial<StandardMaterial>,
>;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct SopesBanyan<M, SS, BS>
where
	M: Material,
	SS: Clone + Into<MeshMaterial3d<M>> + Args,
	BS: Clone + Into<MeshMaterial3d<M>> + Args,
{
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: SopesBanyanSbs,

	/// Stick mesh material (embedded on each [`ChicoStick`]).
	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: SS,

	/// Ball mesh material (embedded on each [`ChicoBall`]).
	#[command(flatten, next_help_heading = "Ball Material")]
	pub ball_material: BS,

	/// Stick surface noise as `seed,frequency,amplitude,octaves`.
	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Surface Noise"
	)]
	pub stick_surface_noise: NoiseParams,

	/// Ball surface noise as `seed,frequency,amplitude,octaves`.
	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Surface Noise"
	)]
	pub ball_surface_noise: NoiseParams,

	#[arg(skip)]
	__marker: PhantomData<fn() -> M>,
}

impl<M, SS, BS> Default for SopesBanyan<M, SS, BS>
where
	M: Material,
	SS: Clone + Into<MeshMaterial3d<M>> + Args + Default,
	BS: Clone + Into<MeshMaterial3d<M>> + Args + Default,
{
	fn default() -> Self {
		Self {
			geometry: SopesBanyanSbs::default(),
			stick_material: SS::default(),
			ball_material: BS::default(),
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			ball_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			__marker: PhantomData,
		}
	}
}

impl<M, SS, BS> SopesBanyan<M, SS, BS>
where
	M: Material,
	SS: Clone + Into<MeshMaterial3d<M>> + Args,
	BS: Clone + Into<MeshMaterial3d<M>> + Args,
{
	pub fn build_chain(&self) -> BallStickChain<SopesBanyanChain> {
		self.geometry.build_chain()
	}
}

#[derive(Clone)]
struct SopesBanyanStickRule<M, SS>
where
	M: Material,
	SS: Clone + Into<MeshMaterial3d<M>>,
{
	surface_noise: NoiseParams,
	stick_material: SS,
	__marker: PhantomData<fn() -> M>,
}

impl<M, SS> StickRenderRule<ChicoStick<M, SS>, SopesBanyanChain> for SopesBanyanStickRule<M, SS>
where
	M: Material + Send + Sync + 'static,
	SS: Clone + Into<MeshMaterial3d<M>> + Default + Send + Sync + 'static,
{
	fn stick_render_item_for(
		&self,
		segment: &BallStickSegment<'_>,
		_parent_hysteresis: &SopesBanyanChain,
		_child_hysteresis: &SopesBanyanChain,
	) -> Option<ChicoStick<M, SS>> {
		let seed = self.surface_noise.seed
			+ segment.start.position.length() as i32
			+ segment.end.position.length() as i32;

		let mut stick = self.surface_noise.with_seed(seed).build_scalar::<ChicoStick<M, SS>>();
		stick.material = self.stick_material.clone();
		Some(stick)
	}
}

#[derive(Clone)]
struct SopesBanyanBallRule<M, BS>
where
	M: Material,
	BS: Clone + Into<MeshMaterial3d<M>>,
{
	ball: ChicoBall<M, BS>,
}

impl<M, BS> BallRenderRule<ChicoBall<M, BS>, SopesBanyanChain> for SopesBanyanBallRule<M, BS>
where
	M: Material + Send + Sync + 'static,
	BS: Clone + Into<MeshMaterial3d<M>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		_node: &BallStickNode,
		_hysteresis: &SopesBanyanChain,
	) -> Option<ChicoBall<M, BS>> {
		Some(self.ball.clone())
	}
}

impl<M, SS, BS> RenderItem for SopesBanyan<M, SS, BS>
where
	M: Material + Send + Sync + 'static,
	SS: Clone + Into<MeshMaterial3d<M>> + Args + Send + Sync + 'static + Default,
	BS: Clone + Into<MeshMaterial3d<M>> + Args + Send + Sync + 'static + Default,
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		let chain = self.build_chain();

		let stick_rule = SopesBanyanStickRule::<M, SS> {
			surface_noise: self.stick_surface_noise,
			stick_material: self.stick_material.clone(),
			__marker: PhantomData,
		};

		let mut out = StickRenderHelper::new(chain.clone(), stick_rule).spawn_render_items(
			commands,
			cascade_chunk,
			transform,
		);

		let mut ball = self.ball_surface_noise.build_scalar::<ChicoBall<M, BS>>();
		ball.material = self.ball_material.clone();
		let ball_rule = SopesBanyanBallRule { ball };

		out.extend(BallRenderHelper::new(chain, ball_rule).spawn_render_items(
			commands,
			cascade_chunk,
			transform,
		));

		out
	}
}
