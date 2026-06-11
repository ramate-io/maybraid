//! **Date Palm** — columnar trunk + stacked frond crown ([#256](https://github.com/ramate-io/maybraid/issues/256), [RFC §3.1.7.9](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/09-date-palm/README.md)).

//! # Intent
//!
//! Tight vertical [`DatePalmChain`](chico_sbs_geometry::DatePalmChain) trunk with inverted palm-trunk stick taper;
//! stacked [`FrondCrown`](chico_ball_components::frond::FrondCrown) rings and a small concealment tuft at the tip.

mod crown;
pub mod render_item_plugin;
mod stick;
mod tuft;

use std::marker::PhantomData;

use bevy::prelude::*;
use chico_sbs_geometry::render::stick::StickRenderHelper;
use chico_sbs_geometry::DatePalmSbs;
use clap::Args;
use procedural_common::noise_params_from_scalar_str;
use procedural_common::NoiseParams;
use render_item::{CascadeChunk, RenderItem};

use crate::skipped_mesh_material::{SkippedLeafMeshMaterial, SkippedStickMeshMaterial};
use crown::spawn_crown_rings;
use stick::DatePalmStickRule;
use tuft::spawn_crown_tuft;

/// Typical [`StandardMaterial`] Date Palm using CLI-skipped handles.
pub type DatePalmStd = DatePalm<
	StandardMaterial,
	SkippedStickMeshMaterial<StandardMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
>;

#[derive(Component, Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct DatePalm<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: DatePalmSbs,

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
	pub foliage_noise: NoiseParams,

	#[arg(skip)]
	__marker: PhantomData<(fn() -> StickM, fn() -> LeafM)>,
}

impl<StickM, StickS, LeafM, LeafS> Default for DatePalm<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args + Default,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args + Default,
{
	fn default() -> Self {
		Self {
			geometry: DatePalmSbs::default(),
			stick_material: StickS::default(),
			leaf_material: LeafS::default(),
			stick_surface_noise: NoiseParams::from_scalar(0.0, 1.0, 0.05, 1),
			foliage_noise: NoiseParams::from_scalar(0.0, 1.0, 0.06, 1),
			__marker: PhantomData,
		}
	}
}

impl<StickM, StickS, LeafM, LeafS> DatePalm<StickM, StickS, LeafM, LeafS>
where
	StickM: Material,
	StickS: Clone + Into<MeshMaterial3d<StickM>> + Args,
	LeafM: Material,
	LeafS: Clone + Into<MeshMaterial3d<LeafM>> + Args,
{
	pub fn build_chain(&self) -> chico_sbs_geometry::BallStickChain<chico_sbs_geometry::DatePalmChain> {
		self.geometry.build_chain()
	}
}

impl<StickM, StickS, LeafM, LeafS> RenderItem for DatePalm<StickM, StickS, LeafM, LeafS>
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

		let stick_rule = DatePalmStickRule::<StickM, StickS> {
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

		spawn_crown_rings(
			&self.geometry,
			&chain,
			commands,
			cascade_chunk,
			root,
			&self.foliage_noise,
			self.leaf_material.clone(),
		);

		spawn_crown_tuft(
			&self.geometry,
			&chain,
			commands,
			cascade_chunk,
			root,
			&self.foliage_noise,
			self.leaf_material.clone(),
		);

		vec![root]
	}
}
