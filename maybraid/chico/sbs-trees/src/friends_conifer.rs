//! **Friend's Conifer** — log-profile conifer with plane-splay foliage ([#236](https://github.com/ramate-io/maybraid/issues/236),
//! [RFC-183 §3.1.7.14](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/14-friend-s-conifer/README.md)).
//!
//! [`FriendsConiferParams::build`] grows the ball-stick chain once into [`FriendsConifer`], which
//! implements [`VegetationComponents`]. Sticks/foliage reuse Northern banding (`FriendsConiferChain`
//! is [`LiamsConiferChain`](chico_sbs_geometry::LiamsConiferChain)); High/Medium/Low emit cheap-ball
//! joint clusters sized by splay radius (historical plane-splay mesh is not used under VC).

pub mod canopy;
pub mod render_item_plugin;
#[allow(dead_code)]
mod stick;

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, FriendsConiferChain, FriendsConiferSbs};
use chico_vegetation_components::{
	chico_leaf_material_ref, chico_stick_material_ref, FoliageNode, Layers, StickNode,
	StructuralLod, VegetationComponents,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use crate::conifer_canopy_apex::{
	DEFAULT_APEX_CANOPY_SPAWN_FRACTION, FRIENDS_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
};
use crate::northern_conifer::canopy::{
	foliage_nodes_banded, foliage_nodes_low, foliage_nodes_medium, HIGH_FOLIAGE_BANDS,
};
use crate::northern_conifer::stick::{stick_nodes_high, stick_nodes_low, stick_nodes_medium};
use crate::torch_tree::structural_lod_for;

pub use canopy::{
	FRIENDS_SPLAY_CORE_RADIUS, FRIENDS_SPLAY_LEAF_DISC_RADIUS, FRIENDS_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
};

/// Authoring / CLI parameters for Friend's Conifer.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct FriendsConiferParams {
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: FriendsConiferSbs,

	/// Plane-splay world radius as a fraction of stalk height.
	#[arg(
		long,
		default_value_t = FRIENDS_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
		help_heading = "Foliage"
	)]
	pub splay_radius_fraction_of_height: f32,

	/// Fraction of ball-stick joints that receive foliage (1.0 = all joints).
	#[arg(long, default_value_t = 1.0, help_heading = "Foliage")]
	pub splay_spawn_fraction: f32,

	/// Fraction of trees that spawn one apex ball at the stalk crown.
	#[arg(long, default_value_t = DEFAULT_APEX_CANOPY_SPAWN_FRACTION, help_heading = "Foliage")]
	pub apex_canopy_spawn_fraction: f32,

	/// Apex ball world radius as a fraction of stalk height.
	#[arg(
		long,
		default_value_t = FRIENDS_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
		help_heading = "Foliage"
	)]
	pub apex_ball_radius_fraction_of_height: f32,
}

impl Default for FriendsConiferParams {
	fn default() -> Self {
		Self {
			geometry: FriendsConiferSbs::default(),
			splay_radius_fraction_of_height: FRIENDS_SPLAY_RADIUS_FRACTION_OF_HEIGHT,
			splay_spawn_fraction: 1.0,
			apex_canopy_spawn_fraction: DEFAULT_APEX_CANOPY_SPAWN_FRACTION,
			apex_ball_radius_fraction_of_height: FRIENDS_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
		}
	}
}

impl FriendsConiferParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> FriendsConifer {
		FriendsConifer::from_params(self)
	}
}

/// Built Friend's Conifer: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct FriendsConifer {
	pub geometry: FriendsConiferSbs,
	pub chain: BallStickChain<FriendsConiferChain>,
	pub splay_radius_fraction_of_height: f32,
	pub splay_spawn_fraction: f32,
	pub apex_canopy_spawn_fraction: f32,
	pub apex_ball_radius_fraction_of_height: f32,
}

impl FriendsConifer {
	pub fn from_params(params: &FriendsConiferParams) -> Self {
		Self {
			chain: params.geometry.build_chain(),
			geometry: params.geometry.clone(),
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

impl VegetationComponents for FriendsConifer {
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
