//! **Sope's Banyan** — end-to-end tree assembly for Chico ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252)).
//!
//! # Intent
//!
//! Wire the vertical **vase banyan** recipe: [Banyan Trunk §3.1.6.5](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/05-banyan-trunk/README.md) stalk (`chico_sdf` / stalk height and radius fractions from the RFC), `chico-sbs-geometry` anchor rings plus [`chico_sbs_geometry::chain::sopes_banyan`](chico_sbs_geometry::chain::sopes_banyan) hysteresis, segment meshes via `chico-stick` ([noisy tapered cylinder](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/01-stick-and-stalk-components/README.md)), and canopy balls via `chico-ball` and `plane-splay` with RFC ball selection (foliage broadly in the rising crown; sparse on descenders unless tuning for denser mystique). Optional `tree-components` / jungle growth (tufts) comes later for dense variants per [§3.1.6.4 Jungle growths](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/04-jungle-growths/README.md).
//!
//! # Rendering split
//!
//! - **Stick material** — [`ChicoStick`] segments plus [`ChicoBall`] markers at **internal joints** (nodes with at least one child edge): bark-colored joints between sticks.
//! - **Leaf material** — [`ChicoBall`] at **terminal nodes** (no outgoing edges), scaled by [`Self::leaf_ball_radius_scale`] for canopy reads vs joint spheres.
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

/// Typical [`StandardMaterial`] tree using CLI-skipped handles for both bark (`stick_material`) and foliage (`leaf_material`).
pub type SopesBanyanStd = SopesBanyan<
	StandardMaterial,
	SkippedMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedMeshMaterial<StandardMaterial>,
>;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct SopesBanyan<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: SopesBanyanSbs,

	/// Bark / wood mesh material for [`ChicoStick`] segments and joint [`ChicoBall`] markers.
	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	/// Foliage mesh material for terminal canopy [`ChicoBall`] markers.
	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	/// Uniform scale on [`BallStickNode::radius`] for terminal leaf balls (joint balls stay at `1.0`).
	#[arg(long, default_value_t = 2.0)]
	#[arg(help_heading = "Canopy")]
	pub leaf_ball_radius_scale: f32,

	/// Stick surface noise as `seed,frequency,amplitude,octaves`.
	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Surface Noise"
	)]
	pub stick_surface_noise: NoiseParams,

	/// Leaf canopy ball surface noise as `seed,frequency,amplitude,octaves`.
	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES",
		help_heading = "Surface Noise"
	)]
	pub leaf_surface_noise: NoiseParams,

	#[arg(skip)]
	__marker: PhantomData<(fn() -> StickM, fn() -> LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS> Default for SopesBanyan<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Default,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Default,
{
	fn default() -> Self {
		Self {
			geometry: SopesBanyanSbs::default(),
			stick_material: StickS::default(),
			leaf_material: LeafS::default(),
			leaf_ball_radius_scale: 8.0,
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, LeafM, LeafS> SopesBanyan<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	pub fn build_chain(&self) -> BallStickChain<SopesBanyanChain> {
		self.geometry.build_chain()
	}
}

#[derive(Clone)]
struct SopesBanyanStickRule<StickM, StickS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>>,
{
	surface_noise: NoiseParams,
	stick_material: StickS,
	__marker: PhantomData<fn() -> StickM>,
}

impl<StickM, StickS> StickRenderRule<ChicoStick<StickM, StickS>, SopesBanyanChain>
	for SopesBanyanStickRule<StickM, StickS>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Default + Send + Sync + 'static,
{
	fn stick_render_item_for(
		&self,
		segment: &BallStickSegment<'_>,
		_parent_hysteresis: &SopesBanyanChain,
		_child_hysteresis: &SopesBanyanChain,
	) -> Option<ChicoStick<StickM, StickS>> {
		let seed = self.surface_noise.seed
			+ segment.start.position.length() as i32
			+ segment.end.position.length() as i32;

		let mut stick =
			self.surface_noise.with_seed(seed).build_scalar::<ChicoStick<StickM, StickS>>();
		stick.material = self.stick_material.clone();
		Some(stick)
	}
}

/// Joint [`ChicoBall`] using [`SopesBanyan::stick_material`] + [`SopesBanyan::stick_surface_noise`].
#[derive(Clone)]
struct SopesBanyanJointBallRule<StickM, StickS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>>,
{
	joint_ball: ChicoBall<StickM, StickS>,
}

impl<StickM, StickS> BallRenderRule<ChicoBall<StickM, StickS>, SopesBanyanChain>
	for SopesBanyanJointBallRule<StickM, StickS>
where
	StickM: Material + Send + Sync + 'static,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		node_idx: usize,
		_node: &BallStickNode,
		_hysteresis: &SopesBanyanChain,
		chain: &BallStickChain<SopesBanyanChain>,
	) -> Option<(ChicoBall<StickM, StickS>, f32)> {
		if chain.children[node_idx].is_empty() {
			return None;
		}
		Some((self.joint_ball.clone(), 1.0))
	}
}

/// Terminal canopy [`ChicoBall`] using [`SopesBanyan::leaf_material`] + [`SopesBanyan::leaf_surface_noise`].
#[derive(Clone)]
struct SopesBanyanLeafBallRule<LeafM, LeafS>
where
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>>,
{
	leaf_ball: ChicoBall<LeafM, LeafS>,
	min_height: f32,
}

impl<LeafM, LeafS> BallRenderRule<ChicoBall<LeafM, LeafS>, SopesBanyanChain>
	for SopesBanyanLeafBallRule<LeafM, LeafS>
where
	LeafM: Material + Send + Sync + 'static,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Send + Sync + 'static,
{
	fn ball_render_item_for(
		&self,
		_node_idx: usize,
		node: &BallStickNode,
		_hysteresis: &SopesBanyanChain,
		_chain: &BallStickChain<SopesBanyanChain>,
	) -> Option<(ChicoBall<LeafM, LeafS>, f32)> {
		const BALL_SIZE: f32 = 6.0;

		if node.position.y < self.min_height {
			return None;
		}

		let scale = BALL_SIZE / node.radius;

		Some((self.leaf_ball.clone(), scale))
	}
}

impl<StickM, StickS, LeafM, LeafS> RenderItem for SopesBanyan<StickM, StickS, LeafM, LeafS>
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

		let stick_rule = SopesBanyanStickRule::<StickM, StickS> {
			surface_noise: self.stick_surface_noise,
			stick_material: self.stick_material.clone(),
			__marker: PhantomData,
		};

		let mut out = StickRenderHelper::new(chain.clone(), stick_rule).spawn_render_items(
			commands,
			cascade_chunk,
			transform,
		);

		let mut joint_ball = self.stick_surface_noise.build_scalar::<ChicoBall<StickM, StickS>>();
		joint_ball.material = self.stick_material.clone();
		let joint_rule = SopesBanyanJointBallRule { joint_ball };

		out.extend(BallRenderHelper::new(chain.clone(), joint_rule).spawn_render_items(
			commands,
			cascade_chunk,
			transform,
		));

		let mut leaf_ball = self.leaf_surface_noise.build_scalar::<ChicoBall<LeafM, LeafS>>();
		leaf_ball.material = self.leaf_material.clone();
		let leaf_rule = SopesBanyanLeafBallRule {
			min_height: self.geometry.scale.base_anchor.y
				+ self.geometry.scale.stalk_height * self.geometry.rings.height_range.start,
			leaf_ball,
		};

		out.extend(BallRenderHelper::new(chain, leaf_rule).spawn_render_items(
			commands,
			cascade_chunk,
			transform,
		));

		out
	}
}
