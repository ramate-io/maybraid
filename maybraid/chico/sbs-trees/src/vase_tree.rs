//! **Vase Tree** — upward-opening vase-profile broadleaf ([#246](https://github.com/ramate-io/maybraid/issues/246), [RFC §3.1.7.3](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/03-vase-tree/README.md)).
//!
//! [`VaseTreeParams::build`] grows the ball-stick chain once into [`VaseTree`],
//! which implements [`VegetationComponents`].
//!
//! Structural / stick LOD matches Penmarch Torch (`torch_tree`); foliage uses cheap-ball
//! banding on upper / outer joints, a stalk-tip apex, and a Low mid-canopy layered proxy.

mod canopy;
pub mod render_item_plugin;

use bevy::prelude::*;
use chico_sbs_geometry::{
	BallStickChain, StorybookTreeChain, VaseTreeSbs, DEFAULT_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
};
use chico_vegetation_components::{
	chico_leaf_material_ref, chico_stick_material_ref, FoliageNode, Layers, StickNode,
	VegetationComponents, StructuralLod,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use crate::torch_tree::{stick_nodes_high, stick_nodes_low, stick_nodes_medium};
use canopy::{
	foliage_nodes_banded, foliage_nodes_low, foliage_nodes_medium, HIGH_FOLIAGE_BANDS,
};

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 5.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 15.0;
const STRUCTURAL_LOW_FACTOR: f32 = 24.0;

/// Authoring / CLI parameters for Vase Tree.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct VaseTreeParams {
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: VaseTreeSbs,

	/// Crown ball world radius as a fraction of tree height `H`.
	#[arg(
		long,
		default_value_t = DEFAULT_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
		help_heading = "Foliage"
	)]
	pub apex_ball_radius_fraction_of_height: f32,
}

impl Default for VaseTreeParams {
	fn default() -> Self {
		Self {
			geometry: VaseTreeSbs::default(),
			apex_ball_radius_fraction_of_height: DEFAULT_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT,
		}
	}
}

impl VaseTreeParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> VaseTree {
		VaseTree::from_params(self)
	}

	/// RFC bush / grape-vine preset (shorter stalk, wider spread).
	pub fn apply_bush_preset(&mut self) {
		self.geometry.apply_bush_preset();
	}
}

/// Built Vase Tree: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct VaseTree {
	pub geometry: VaseTreeSbs,
	pub chain: BallStickChain<StorybookTreeChain>,
	pub apex_ball_radius_fraction_of_height: f32,
}

impl VaseTree {
	pub fn from_params(params: &VaseTreeParams) -> Self {
		Self {
			geometry: params.geometry.clone(),
			chain: params.geometry.build_chain(),
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

	fn leaf_radius_world(&self) -> f32 {
		self.geometry.leaf_radius_world()
	}

	fn apex_radius_world(&self) -> f32 {
		self.geometry
			.apex_radius_world(self.apex_ball_radius_fraction_of_height)
	}

	fn upper_foliage_ring_u(&self) -> f32 {
		self.geometry.canopy.upper_foliage_ring_u
	}
}

impl VegetationComponents for VaseTree {
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
		let leaf_r = self.leaf_radius_world();
		let upper_u = self.upper_foliage_ring_u();
		let apex_r = self.apex_radius_world();
		let nodes = match level {
			LodSceneLevel::High => foliage_nodes_banded(
				&self.chain,
				HIGH_FOLIAGE_BANDS,
				leaf_r,
				upper_u,
				apex_r,
			),
			LodSceneLevel::Medium => {
				foliage_nodes_medium(&self.chain, leaf_r, upper_u, apex_r)
			}
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => {
				foliage_nodes_low(&self.chain, leaf_r, upper_u, apex_r)
			}
		};
		Layers::from_free(nodes).map(|n| n.with_material(chico_leaf_material_ref()))
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(
			StructuralLod::from_extent(
				self.structural_center(),
				self.footprint_radius(),
				self.geometry.height(),
			)
			.with_factors(
				STRUCTURAL_HIGH_FACTOR,
				STRUCTURAL_MEDIUM_FACTOR,
				STRUCTURAL_LOW_FACTOR,
			),
		)
	}
}
