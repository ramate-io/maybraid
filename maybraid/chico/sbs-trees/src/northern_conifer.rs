//! **Northern Conifer** — Liam's geometry with plane-splay foliage ([#232](https://github.com/ramate-io/maybraid/issues/232),
//! [RFC-183 §3.1.7.11](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/11-northern-conifer/README.md)).
//!
//! [`NorthernConiferParams::build`] applies the Northern preset, grows the ball-stick chain once
//! into [`NorthernConifer`], which implements [`VegetationComponents`].
//!
//! Foliage uses cheap-ball banding; Medium is ~30% fewer cells; Medium/Low share a thin
//! top-anchored full-height layered canopy proxy (Low emits it twice).

pub(crate) mod canopy;
pub mod render_item_plugin;
pub(crate) mod stick;

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, LiamsConiferChain, NorthernConiferSbs};
use chico_vegetation_components::{
	chico_leaf_material_ref, chico_stick_material_ref, FoliageNode, Layers, StickNode,
	VegetationComponents, StructuralLod,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use crate::conifer_canopy_apex::NORTHERN_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT;
use crate::torch_tree::structural_lod_for;
use canopy::{
	foliage_nodes_banded, foliage_nodes_low, foliage_nodes_medium, HIGH_FOLIAGE_BANDS,
};
use stick::{stick_nodes_high, stick_nodes_low, stick_nodes_medium};

pub use canopy::{
	NORTHERN_SPLAY_CORE_RADIUS, NORTHERN_SPLAY_LEAF_DISC_RADIUS,
	NORTHERN_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
};

/// Authoring / CLI parameters for Northern Conifer.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct NorthernConiferParams {
	/// Flattened [`LiamsConiferSbs`] (clap defaults are Liam's, not Northern).
	/// [`Self::build`] / [`NorthernConifer::from_params`] call [`NorthernConiferSbs::apply_northern_preset`].
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: NorthernConiferSbs,

	/// Plane-splay world radius as a fraction of stalk height.
	#[arg(
		long,
		default_value_t = NORTHERN_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
		help_heading = "Foliage"
	)]
	pub splay_radius_fraction_of_height: f32,

	/// Fraction of ball-stick joints that receive plane-splay foliage (1.0 = all joints).
	#[arg(long, default_value_t = 1.0, help_heading = "Foliage")]
	pub splay_spawn_fraction: f32,

	/// Fraction of trees that spawn one apex ball at the stalk crown.
	#[arg(long, default_value_t = 1.0, help_heading = "Foliage")]
	pub apex_canopy_spawn_fraction: f32,

	/// Apex ball world radius as a fraction of stalk height.
	#[arg(
		long,
		default_value_t = NORTHERN_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
		help_heading = "Foliage"
	)]
	pub apex_ball_radius_fraction_of_height: f32,
}

impl Default for NorthernConiferParams {
	fn default() -> Self {
		Self {
			geometry: NorthernConiferSbs::default(),
			splay_radius_fraction_of_height: NORTHERN_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
			splay_spawn_fraction: 1.0,
			apex_canopy_spawn_fraction: 1.0,
			apex_ball_radius_fraction_of_height: NORTHERN_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
		}
	}
}

impl NorthernConiferParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> NorthernConifer {
		NorthernConifer::from_params(self)
	}
}

/// Built Northern Conifer: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct NorthernConifer {
	pub geometry: NorthernConiferSbs,
	pub chain: BallStickChain<LiamsConiferChain>,
	pub splay_radius_fraction_of_height: f32,
	pub splay_spawn_fraction: f32,
	pub apex_canopy_spawn_fraction: f32,
	pub apex_ball_radius_fraction_of_height: f32,
}

impl NorthernConifer {
	pub fn from_params(params: &NorthernConiferParams) -> Self {
		let mut geometry = params.geometry.clone();
		geometry.apply_northern_preset();
		Self {
			chain: geometry.build_chain(),
			geometry,
			splay_radius_fraction_of_height: params.splay_radius_fraction_of_height,
			splay_spawn_fraction: params.splay_spawn_fraction,
			apex_canopy_spawn_fraction: params.apex_canopy_spawn_fraction,
			apex_ball_radius_fraction_of_height: params.apex_ball_radius_fraction_of_height,
		}
	}

	fn footprint_radius(&self) -> f32 {
		self.chain.footprint_radius_at_least(
			self.geometry.scale.stalk_base_radius_or_default().max(1e-3),
		)
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.geometry.height() * 0.5, 0.0)
	}

	fn splay_radius_world(&self) -> f32 {
		self.geometry.height() * self.splay_radius_fraction_of_height
	}

	fn apex_radius_world(&self) -> f32 {
		self.geometry.height() * self.apex_ball_radius_fraction_of_height
	}
}

impl VegetationComponents for NorthernConifer {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => stick_nodes_high(&self.chain),
			LodSceneLevel::Medium => stick_nodes_medium(&self.chain),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => stick_nodes_low(&self.chain),
		};
		Layers::from_free(nodes).map(|n| n.with_material(chico_stick_material_ref()))
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let splay_r = self.splay_radius_world();
		let apex_r = self.apex_radius_world();
		let nodes = match level {
			LodSceneLevel::High => foliage_nodes_banded(
				&self.chain,
				HIGH_FOLIAGE_BANDS,
				splay_r,
				self.splay_spawn_fraction,
				self.apex_canopy_spawn_fraction,
				apex_r,
			),
			LodSceneLevel::Medium => foliage_nodes_medium(
				&self.chain,
				splay_r,
				self.splay_spawn_fraction,
				self.apex_canopy_spawn_fraction,
				apex_r,
			),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => foliage_nodes_low(
				&self.chain,
				splay_r,
				self.splay_spawn_fraction,
				self.apex_canopy_spawn_fraction,
				apex_r,
			),
		};
		Layers::from_free(nodes).map(|n| n.with_material(chico_leaf_material_ref()))
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(structural_lod_for(
			self.structural_center(),
			self.footprint_radius(),
			self.geometry.height(),
		))
	}
}
