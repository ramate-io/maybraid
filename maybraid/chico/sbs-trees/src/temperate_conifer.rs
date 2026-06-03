//! **Temperate Conifer** — sparse fronded Friend's Conifer variant ([#238](https://github.com/ramate-io/maybraid/issues/238),
//! [RFC-183 §3.1.7.15](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/15-temperate-conifer/README.md)).
//!
//! Geometry from [`FriendsConiferSbs`](chico_sbs_geometry::FriendsConiferSbs); joint foliage uses
//! [`FrondCrown`](chico_ball_components::frond::FrondCrown) aligned to branch direction.

mod foliage;
mod stick;
pub mod render_item_plugin;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::render::stick::StickRenderHelper;
use chico_sbs_geometry::FriendsConiferSbs;
use clap::Args;

/// [`FriendsConiferSbs`] with Temperate Conifer limb/ray defaults (clap `flatten` base).
#[derive(Clone, Debug, PartialEq, Args)]
#[command(rename_all = "kebab-case")]
pub struct TemperateConiferGeometry {
	#[command(flatten)]
	pub inner: FriendsConiferSbs,
}

impl Default for TemperateConiferGeometry {
	fn default() -> Self {
		let mut inner = FriendsConiferSbs::default();
		inner.apply_temperate_preset();
		Self { inner }
	}
}

impl std::ops::Deref for TemperateConiferGeometry {
	type Target = FriendsConiferSbs;
	fn deref(&self) -> &Self::Target {
		&self.inner
	}
}
use procedural_common::noise_params_from_scalar_str;
use procedural_common::parse_unit_range;
use procedural_common::{FromScalarNoise, NoiseParams, UnitRange};
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};
use foliage::spawn_joint_fronds;
use stick::TemperateConiferStickRule;

/// Typical [`StandardMaterial`] tree using CLI-skipped handles for bark and foliage.
pub type TemperateConiferStd = TemperateConifer<
	StandardMaterial,
	SkippedStickMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
>;

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct TemperateConifer<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: TemperateConiferGeometry,

	#[command(flatten, next_help_heading = "Stick Material")]
	pub stick_material: StickS,

	#[command(flatten, next_help_heading = "Leaf Material")]
	pub leaf_material: LeafS,

	/// Uniform world scale applied to each joint [`FrondCrown`] mesh.
	#[arg(long, default_value_t = 1.0, help_heading = "Foliage")]
	pub frond_world_scale: f32,

	/// Fronds placed per ball-stick joint (RFC `1..2`).
	#[arg(
		long = "fronds-per-joint",
		default_value = "1..2",
		value_parser = parse_unit_range,
		value_name = "MIN..MAX",
		help_heading = "Foliage"
	)]
	pub fronds_per_joint: UnitRange,

	/// Frond spine length as a fraction of stalk height (RFC `0.035..0.07`).
	#[arg(
		long = "frond-length-fraction",
		default_value = "0.035..0.07",
		value_parser = parse_unit_range,
		value_name = "MIN..MAX",
		help_heading = "Foliage"
	)]
	pub frond_length_fraction: UnitRange,

	/// Fraction of joints that receive fronds (sparse dryland &lt; 1.0).
	#[arg(long, default_value_t = 1.0, help_heading = "Foliage")]
	pub frond_spawn_fraction: f32,

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

impl<StickM, StickS, LeafM, LeafS> Default for TemperateConifer<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Default,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Default,
{
	fn default() -> Self {
		Self {
			geometry: TemperateConiferGeometry::default(),
			stick_material: StickS::default(),
			leaf_material: LeafS::default(),
			frond_world_scale: 1.0,
			fronds_per_joint: UnitRange::new(1.0, 2.0),
			frond_length_fraction: UnitRange::new(0.035, 0.07),
			frond_spawn_fraction: 1.0,
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			leaf_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, LeafM, LeafS> TemperateConifer<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	pub fn build_chain(&self) -> chico_sbs_geometry::BallStickChain<chico_sbs_geometry::FriendsConiferChain> {
		let mut geometry = self.geometry.inner.clone();
		geometry.apply_temperate_preset();
		geometry.build_chain()
	}
}

impl<StickM, StickS, LeafM, LeafS> RenderItem for TemperateConifer<StickM, StickS, LeafM, LeafS>
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

		let stick_rule = TemperateConiferStickRule::<StickM, StickS> {
			surface_noise: self.stick_surface_noise,
			stick_material: self.stick_material.clone(),
			__marker: PhantomData,
		};

		let mut out = StickRenderHelper::new(chain.clone(), stick_rule).spawn_render_items(
			commands,
			cascade_chunk,
			transform,
		);

		let mut geometry = self.geometry.inner.clone();
		geometry.apply_temperate_preset();
		out.extend(spawn_joint_fronds::<LeafM, _>(
			&geometry,
			self.frond_world_scale,
			&chain,
			commands,
			cascade_chunk,
			transform,
			&self.fronds_per_joint,
			&self.frond_length_fraction,
			self.frond_spawn_fraction,
			self.leaf_material.clone(),
		));

		out
	}
}
