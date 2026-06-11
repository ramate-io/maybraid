//! **Northern Conifer** — Liam's geometry with plane-splay foliage ([#232](https://github.com/ramate-io/maybraid/issues/232),
//! [RFC-183 §3.1.7.11](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/11-northern-conifer/README.md)).
//!
//! Geometry from [`NorthernConiferSbs`](chico_sbs_geometry::NorthernConiferSbs) (flattened clap uses Liam defaults;
//! [`Self::geometry_for_render`] applies the Northern preset). [`PlaneSplay`](chico_ball_components::plane_splay::PlaneSplay) at joints and an optional crown [`ChicoBall`](chico_ball_components::chico_ball::ChicoBall).

mod canopy;
pub mod render_item_plugin;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::render::ball::BallRenderHelper;
use chico_sbs_geometry::render::stick::StickRenderHelper;
use chico_sbs_geometry::{BallStickChain, LiamsConiferChain, NorthernConiferSbs};
use clap::Args;
use procedural_common::noise_params_from_scalar_str;
use procedural_common::NoiseParams;
use render_item::{CascadeChunk, RenderItem};

use crate::conifer_canopy_apex::{
	spawn_apex_chico_ball_northern, NORTHERN_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
};
use crate::liams_conifer::stick::LiamsConiferStickRule;
use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};
use canopy::{
	NorthernConiferCanopyRule, NORTHERN_SPLAY_CORE_RADIUS, NORTHERN_SPLAY_LEAF_DISC_RADIUS,
	NORTHERN_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
};

pub type NorthernConiferStd = NorthernConifer<
	StandardMaterial,
	SkippedStickMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
>;

#[derive(Component, Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct NorthernConifer<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	/// Flattened [`LiamsConiferSbs`] (clap defaults are Liam's, not Northern). [`Self::geometry_for_render`] calls [`NorthernConiferSbs::apply_northern_preset`].
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: NorthernConiferSbs,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	/// Plane-splay world radius as a fraction of stalk height (RFC [`NORTHERN_SPLAY_RADIUS_FRACTION_OF_HEIGHT`]).
	#[arg(
		long,
		default_value_t = NORTHERN_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
		help_heading = "Foliage"
	)]
	pub splay_radius_fraction_of_height: f32,

	/// Fraction of ball-stick joints that receive plane-splay foliage (1.0 = all joints).
	#[arg(long, default_value_t = 1.0, help_heading = "Foliage")]
	pub splay_spawn_fraction: f32,

	/// Fraction of trees that spawn one [`chico_ball_components::chico_ball::ChicoBall`] at the stalk crown.
	#[arg(long, default_value_t = 1.0, help_heading = "Foliage")]
	pub apex_canopy_spawn_fraction: f32,

	/// Apex ball world radius as a fraction of stalk height.
	#[arg(
		long,
		default_value_t = NORTHERN_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
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

impl<StickM, StickS, LeafM, LeafS> Default for NorthernConifer<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Default,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Default,
{
	fn default() -> Self {
		Self {
			geometry: NorthernConiferSbs::default(),
			stick_material: StickS::default(),
			leaf_material: LeafS::default(),
			splay_radius_fraction_of_height: NORTHERN_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
			splay_spawn_fraction: 1.0,
			apex_canopy_spawn_fraction: 1.0,
			apex_ball_radius_fraction_of_height: NORTHERN_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, LeafM, LeafS> NorthernConifer<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	/// SBS snapshot with Northern preset applied (safe to call after CLI parse).
	pub fn geometry_for_render(&self) -> NorthernConiferSbs {
		let mut geometry = self.geometry.clone();
		geometry.apply_northern_preset();
		geometry
	}

	pub fn build_chain(&self) -> BallStickChain<LiamsConiferChain> {
		self.geometry_for_render().build_chain()
	}

	pub fn splay_radius_world(&self) -> f32 {
		self.geometry_for_render().height() * self.splay_radius_fraction_of_height
	}
}

impl<StickM, StickS, LeafM, LeafS> RenderItem for NorthernConifer<StickM, StickS, LeafM, LeafS>
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
		let geometry = self.geometry_for_render();
		let chain = geometry.build_chain();

		let stick_rule = LiamsConiferStickRule::<StickM, StickS> {
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

		let mut leaf_splay = PlaneSplay::<LeafM, LeafS>::default();
		leaf_splay.core_radius = NORTHERN_SPLAY_CORE_RADIUS;
		leaf_splay.leaf_disc_radius = NORTHERN_SPLAY_LEAF_DISC_RADIUS;
		leaf_splay.icosphere_subdivisions = 2;
		leaf_splay.material = self.leaf_material.clone();
		let leaf_rule = NorthernConiferCanopyRule {
			leaf_splay,
			splay_radius_world: self.splay_radius_world(),
			splay_spawn_fraction: self.splay_spawn_fraction,
		};

		BallRenderHelper::new(chain.clone(), leaf_rule).spawn_render_items_under(
			commands,
			cascade_chunk,
			Transform::IDENTITY,
			Some(root),
		);

		spawn_apex_chico_ball_northern::<LeafM, _>(
			&geometry,
			&chain,
			commands,
			cascade_chunk,
			root,
			&self.leaf_surface_noise,
			self.apex_canopy_spawn_fraction,
			self.apex_ball_radius_fraction_of_height,
			self.leaf_material.clone(),
		);

		vec![root]
	}
}
