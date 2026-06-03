//! **Northern Conifer** — Liam's geometry with plane-splay foliage ([#232](https://github.com/ramate-io/maybraid/issues/232),
//! [RFC-183 §3.1.7.11](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/11-northern-conifer/README.md)).
//!
//! Geometry from [`LiamsConiferSbs`](chico_sbs_geometry::LiamsConiferSbs) with Northern ring/projection presets;
//! [`PlaneSplay`](chico_ball_components::plane_splay::PlaneSplay) at joints and an optional crown [`ChicoBall`](chico_ball_components::chico_ball::ChicoBall).

mod canopy;
pub mod render_item_plugin;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_ball_components::plane_splay::PlaneSplay;
use chico_sbs_geometry::render::ball::BallRenderHelper;
use chico_sbs_geometry::render::stick::StickRenderHelper;
use chico_sbs_geometry::{BallStickChain, LiamsConiferChain, LiamsConiferSbs};
use clap::Args;
use procedural_common::noise_params_from_scalar_str;
use procedural_common::{FromScalarNoise, NoiseParams, UnitRange};
use render_item::{CascadeChunk, RenderItem};

use crate::conifer_canopy_apex::{
	spawn_apex_chico_ball_liams, NORTHERN_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
};
use crate::liams_conifer::stick::LiamsConiferStickRule;
use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};
use canopy::{
	NorthernConiferCanopyRule, NORTHERN_SPLAY_CORE_RADIUS, NORTHERN_SPLAY_LEAF_DISC_RADIUS,
	NORTHERN_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
};

/// Default ring band: higher start, rings through the crown.
pub const NORTHERN_DEFAULT_RING_HEIGHTS_START: f32 = 0.38;
pub const NORTHERN_DEFAULT_RING_HEIGHTS_END: f32 = 1.0;

/// Downward tilt on radial seeds (stronger than Liam's default 2°).
pub const NORTHERN_DEFAULT_DOWNWARD_BIAS_DEGREES: f32 = 10.0;

/// [`LiamsConiferSbs`] with Northern Conifer geometry defaults (clap `flatten` base).
#[derive(Clone, Debug, PartialEq, Args)]
#[command(rename_all = "kebab-case")]
pub struct NorthernConiferGeometry {
	#[command(flatten)]
	pub inner: LiamsConiferSbs,
}

impl NorthernConiferGeometry {
	pub fn apply_northern_preset(inner: &mut LiamsConiferSbs) {
		inner.rings.height_range =
			UnitRange::new(NORTHERN_DEFAULT_RING_HEIGHTS_START, NORTHERN_DEFAULT_RING_HEIGHTS_END);
		inner.rings.spacing = 0.035;
		// Keep limb length near max through the crown (linear taper floor near 1.0).
		inner.projection.length_fraction_of_height = UnitRange::new(0.15, 0.95);
		inner.growth.downward_bias_degrees = NORTHERN_DEFAULT_DOWNWARD_BIAS_DEGREES;
	}
}

impl Default for NorthernConiferGeometry {
	fn default() -> Self {
		let mut inner = LiamsConiferSbs::default();
		Self::apply_northern_preset(&mut inner);
		Self { inner }
	}
}

impl std::ops::Deref for NorthernConiferGeometry {
	type Target = LiamsConiferSbs;
	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}

pub type NorthernConiferStd = NorthernConifer<
	StandardMaterial,
	SkippedStickMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
>;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct NorthernConifer<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: NorthernConiferGeometry,

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
			geometry: NorthernConiferGeometry::default(),
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
	pub fn build_chain(&self) -> BallStickChain<LiamsConiferChain> {
		self.geometry.inner.build_chain()
	}

	pub fn splay_radius_world(&self) -> f32 {
		self.geometry.inner.scale.stalk_height * self.splay_radius_fraction_of_height
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
		let chain = self.build_chain();

		let stick_rule = LiamsConiferStickRule::<StickM, StickS> {
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
		leaf_splay.core_radius = NORTHERN_SPLAY_CORE_RADIUS;
		leaf_splay.leaf_disc_radius = NORTHERN_SPLAY_LEAF_DISC_RADIUS;
		leaf_splay.icosphere_subdivisions = 2;
		leaf_splay.material = self.leaf_material.clone();
		let leaf_rule = NorthernConiferCanopyRule {
			leaf_splay,
			splay_radius_world: self.splay_radius_world(),
			splay_spawn_fraction: self.splay_spawn_fraction,
		};

		out.extend(BallRenderHelper::new(chain.clone(), leaf_rule).spawn_render_items(
			commands,
			cascade_chunk,
			transform,
		));

		out.extend(spawn_apex_chico_ball_liams::<LeafM, _>(
			&self.geometry.inner,
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
