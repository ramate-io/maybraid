//! **Friend's Conifer** — log-profile conifer with plane-splay foliage ([#236](https://github.com/ramate-io/maybraid/issues/236),
//! [RFC-183 §3.1.7.14](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/14-friend-s-conifer/README.md)).
//!
//! Geometry from [`FriendsConiferSbs`](chico_sbs_geometry::FriendsConiferSbs); [`PlaneSplay`](chico_ball_components::plane_splay::PlaneSplay) at every joint.

mod canopy;
mod stick;

use crate::conifer_canopy_apex::{
	spawn_apex_chico_ball, DEFAULT_APEX_CANOPY_SPAWN_FRACTION,
	FRIENDS_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
};
pub mod render_item_plugin;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::render::ball::BallRenderHelper;
use chico_sbs_geometry::render::stick::StickRenderHelper;
use chico_sbs_geometry::{BallStickChain, FriendsConiferChain, FriendsConiferSbs};
use clap::Args;
use procedural_common::noise_params_from_scalar_str;
use procedural_common::{FromScalarNoise, NoiseParams};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};
use canopy::{
	FriendsConiferCanopyRule, FRIENDS_SPLAY_CORE_RADIUS, FRIENDS_SPLAY_LEAF_DISC_RADIUS,
	FRIENDS_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
};
use stick::FriendsConiferStickRule;

pub type FriendsConiferStd = FriendsConifer<
	StandardMaterial,
	SkippedStickMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
>;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct FriendsConifer<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: FriendsConiferSbs,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	/// Plane-splay world radius as a fraction of stalk height (playground default [`FRIENDS_SPLAY_RADIUS_FRACTION_OF_HEIGHT`]).
	#[arg(
		long,
		default_value_t = FRIENDS_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
		help_heading = "Foliage"
	)]
	pub splay_radius_fraction_of_height: f32,

	/// Fraction of trees that spawn one [`chico_ball_components::chico_ball::ChicoBall`] at the stalk crown (noise-gated).
	#[arg(long, default_value_t = DEFAULT_APEX_CANOPY_SPAWN_FRACTION, help_heading = "Foliage")]
	pub apex_canopy_spawn_fraction: f32,

	/// Apex ball world radius as a fraction of stalk height.
	#[arg(
		long,
		default_value_t = FRIENDS_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
		help_heading = "Foliage"
	)]
	pub apex_ball_radius_fraction_of_height: f32,

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

impl<StickM, StickS, LeafM, LeafS> Default for FriendsConifer<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Default,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Default,
{
	fn default() -> Self {
		Self {
			geometry: FriendsConiferSbs::default(),
			stick_material: StickS::default(),
			leaf_material: LeafS::default(),
			splay_radius_fraction_of_height: FRIENDS_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
			apex_canopy_spawn_fraction: DEFAULT_APEX_CANOPY_SPAWN_FRACTION,
			apex_ball_radius_fraction_of_height: FRIENDS_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, LeafM, LeafS> FriendsConifer<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	pub fn build_chain(&self) -> BallStickChain<FriendsConiferChain> {
		self.geometry.build_chain()
	}

	pub fn splay_radius_world(&self) -> f32 {
		self.geometry.height() * self.splay_radius_fraction_of_height
	}
}

impl<StickM, StickS, LeafM, LeafS> RenderItem for FriendsConifer<StickM, StickS, LeafM, LeafS>
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

		let stick_rule = FriendsConiferStickRule::<StickM, StickS> {
			surface_noise: self.stick_surface_noise,
			stick_material: self.stick_material.clone(),
			__marker: PhantomData,
		};

		let mut out = StickRenderHelper::new(chain.clone(), stick_rule).spawn_render_items(
			commands,
			cascade_chunk,
			transform,
		);

		let mut leaf_splay = PlaneSplay::<LeafM, LeafS>::default();
		leaf_splay.core_radius = FRIENDS_SPLAY_CORE_RADIUS;
		leaf_splay.leaf_disc_radius = FRIENDS_SPLAY_LEAF_DISC_RADIUS;
		leaf_splay.icosphere_subdivisions = 1;
		leaf_splay.material = self.leaf_material.clone();
		let leaf_rule = FriendsConiferCanopyRule {
			leaf_splay,
			splay_radius_world: self.splay_radius_world(),
		};

		out.extend(BallRenderHelper::new(chain.clone(), leaf_rule).spawn_render_items(
			commands,
			cascade_chunk,
			transform,
		));

		out.extend(spawn_apex_chico_ball::<LeafM, _>(
			&self.geometry,
			&chain,
			commands,
			cascade_chunk,
			transform,
			&self.leaf_surface_noise,
			self.apex_canopy_spawn_fraction,
			self.apex_ball_radius_fraction_of_height,
			self.leaf_material.clone(),
		));

		out
	}
}
