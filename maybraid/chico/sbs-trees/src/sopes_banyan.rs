//! **Sope's Banyan** — end-to-end tree assembly for Chico ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252)).
//!
//! # Intent
//!
//! Wire the vertical **vase banyan** recipe: [Banyan Trunk §3.1.6.5](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/05-banyan-trunk/README.md) stalk (`chico_sdf` / stalk height and radius fractions from the RFC), `chico-sbs-geometry` anchor rings plus [`chico_sbs_geometry::chain::sopes_banyan`](chico_sbs_geometry::chain::sopes_banyan) hysteresis, segment meshes via `chico-stick` ([noisy tapered cylinder](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/01-stick-and-stalk-components/README.md)), and canopy via `chico-ball` **noisy balls** plus [`chico_ball_components::plane_splay`](chico_ball_components::plane_splay) **plane splays** with RFC-style mixing (dense variegation in the rising crown; sparser splays on descenders). Optional `tree-components` / jungle growth (tufts) comes later for dense variants per [§3.1.6.4 Jungle growths](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/04-jungle-growths/README.md).
//!
//! # Rendering split
//!
//! - **Stick material** — [`ChicoStick`] segments plus [`ChicoBall`] markers at **internal joints** (nodes with at least one child edge): bark-colored joints between sticks.
//! - **Leaf material** — At terminals above the crown floor: **[`ChicoBall`]** (noisy sphere) or **[`PlaneSplay`](chico_ball_components::plane_splay::PlaneSplay)** (radial blade meshes), selected per-node for silhouette variegation; uniform scale uses [`SopesBanyanSbs::leaf_ball_size`] (from [`SopesBanyan::geometry`]) / [`BallStickNode::radius`].
//!
//! # Playground / CLI
//!
//! Everything that parameterizes height, rings, chain phases, materials, and optional fruiting ([§3.1.6.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/07-fruiting-bodies/README.md)) should be exposed **under feature flags** as **`clap`-parseable** types so a future playground can drive the same recipe as production.

mod canopy;
mod joint_ball;
pub mod render_item_plugin;
mod stick;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::chico_ball::ChicoBall;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::render::ball::BallRenderHelper;
use chico_sbs_geometry::render::stick::StickRenderHelper;
use chico_sbs_geometry::{BallStickChain, SopesBanyanChain, SopesBanyanSbs};
use clap::Args;
use procedural_common::noise_params_from_scalar_str;
use procedural_common::NoiseParams;
use render_item::{CascadeChunk, RenderItem};

use crate::layered_canopy::LayeredTerminalCanopy;
use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};
use canopy::SopesBanyanLeafCanopyRule;
use joint_ball::SopesBanyanJointBallRule;
use stick::SopesBanyanStickRule;

/// Typical [`StandardMaterial`] tree using CLI-skipped handles for both bark (`stick_material`) and foliage (`leaf_material`).
pub type SopesBanyanStd = SopesBanyan<
	StandardMaterial,
	SkippedStickMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
>;

#[derive(Component, Clone, Args)]
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

	/// Foliage mesh material for terminal canopy ([`ChicoBall`] and [`PlaneSplay`]).
	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	/// Stick surface noise as `seed,frequency,amplitude,octaves`.
	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Surface Noise"
	)]
	pub stick_surface_noise: NoiseParams,

	/// Leaf canopy ball surface noise as `seed,frequency,amplitude,octaves` (used for [`ChicoBall`] terminals; plane splays use the same material handle).
	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
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
		let root = commands
			.spawn((self.clone(), cascade_chunk.clone(), transform, Visibility::default()))
			.id();
		let chain = self.build_chain();

		let stick_rule = SopesBanyanStickRule::<StickM, StickS> {
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

		let mut joint_ball = self.stick_surface_noise.build_scalar::<ChicoBall<StickM, StickS>>();
		joint_ball.material = self.stick_material.clone();
		let joint_rule = SopesBanyanJointBallRule { joint_ball };

		BallRenderHelper::new(chain.clone(), joint_rule).spawn_render_items_under(
			commands,
			cascade_chunk,
			Transform::IDENTITY,
			Some(root),
		);

		let mut leaf_ball = self.leaf_surface_noise.build_scalar::<ChicoBall<LeafM, LeafS>>();
		leaf_ball.material = self.leaf_material.clone();
		let mut leaf_splay = PlaneSplay::<LeafM, LeafS>::default();
		leaf_splay.material = self.leaf_material.clone();
		let leaf_rule = SopesBanyanLeafCanopyRule {
			min_height: self.geometry.crown_floor_world_y(),
			canopy: LayeredTerminalCanopy::new(leaf_ball, leaf_splay),
			leaf_radius_world: self.geometry.leaf_ball_size(),
		};

		BallRenderHelper::new(chain, leaf_rule).spawn_render_items_under(
			commands,
			cascade_chunk,
			Transform::IDENTITY,
			Some(root),
		);

		vec![root]
	}
}
