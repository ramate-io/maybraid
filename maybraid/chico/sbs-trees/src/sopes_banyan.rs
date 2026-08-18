//! **Sope's Banyan** — end-to-end tree assembly for Chico ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252)).
//!
//! [`SopesBanyanParams::build`] grows the ball-stick chain once into [`SopesBanyan`],
//! which implements [`VegetationComponents`].
//!
//! Structural LOD (tree-radius bands):
//! - **High** — within `8 ×` tree radius: full sticks; dense azimuth×height layered canopy
//! - **Medium** — `8…20 ×` radius: trunk + band-sampled sticks; layered outer foliage + mid proxy
//! - **Low** — `20…32 ×` radius: trunk + ~1/4 descenders; cheap-ball outer foliage + mid proxy

mod canopy;
mod stick;

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, SopesBanyanChain, SopesBanyanSbs};
use chico_vegetation_components::{
	chico_leaf_material_ref, chico_stick_material_ref, FoliageNode, Layers, StickNode,
	VegetationComponents, StructuralLod,
};
use clap::Args;
use lod::gen::LodSceneLevel;

use canopy::{
	banded_outer_canopy_balls, banded_outer_canopy_with_proxy, CanopyBallKit, HIGH_FOLIAGE_BANDS,
	LOW_FOLIAGE_BANDS, MEDIUM_FOLIAGE_BANDS,
};
use stick::{
	keep_stick_on_low, stick_node_for_segment, stick_nodes_medium_banded, stick_role_for_segment,
};

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 20.0;
const STRUCTURAL_LOW_FACTOR: f32 = 32.0;

/// Authoring / CLI parameters for Sope's Banyan.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct SopesBanyanParams {
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: SopesBanyanSbs,
}

impl Default for SopesBanyanParams {
	fn default() -> Self {
		Self { geometry: SopesBanyanSbs::default() }
	}
}

impl SopesBanyanParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> SopesBanyan {
		SopesBanyan::from_params(self)
	}
}

/// Built Sope's Banyan: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct SopesBanyan {
	pub geometry: SopesBanyanSbs,
	pub chain: BallStickChain<SopesBanyanChain>,
}

impl SopesBanyan {
	pub fn from_params(params: &SopesBanyanParams) -> Self {
		Self {
			geometry: params.geometry.clone(),
			chain: params.geometry.build_chain(),
		}
	}

	fn footprint_radius(&self) -> f32 {
		self.chain
			.footprint_radius_at_least(self.geometry.scale.stalk_base_radius.max(1e-3))
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.geometry.scale.stalk_height * 0.5, 0.0)
	}

	fn stick_nodes_high(&self) -> Vec<StickNode> {
		self.chain
			.segments_with_hysteresis()
			.filter_map(|(segment, parent, _)| stick_node_for_segment(&segment, parent))
			.collect()
	}

	fn stick_nodes_medium(&self) -> Vec<StickNode> {
		stick_nodes_medium_banded(
			self.chain
				.segments_with_hysteresis()
				.map(|(segment, parent, _)| (segment, parent)),
		)
	}

	fn stick_nodes_low(&self) -> Vec<StickNode> {
		let mut descender_index = 0usize;
		self.chain
			.segments_with_hysteresis()
			.filter_map(|(segment, parent, _)| {
				let role = stick_role_for_segment(&segment, parent);
				if !keep_stick_on_low(role, &mut descender_index) {
					return None;
				}
				stick_node_for_segment(&segment, parent)
			})
			.collect()
	}

	fn foliage_nodes_high(&self) -> Vec<FoliageNode> {
		banded_outer_canopy_balls(
			&self.chain,
			HIGH_FOLIAGE_BANDS,
			self.geometry.crown_floor_world_y(),
			self.geometry.leaf_ball_size(),
			CanopyBallKit::Layered,
		)
	}

	fn foliage_nodes_medium(&self) -> Vec<FoliageNode> {
		banded_outer_canopy_with_proxy(
			&self.chain,
			MEDIUM_FOLIAGE_BANDS,
			self.geometry.crown_floor_world_y(),
			self.geometry.leaf_ball_size(),
			CanopyBallKit::Layered,
		)
	}

	fn foliage_nodes_low(&self) -> Vec<FoliageNode> {
		banded_outer_canopy_with_proxy(
			&self.chain,
			LOW_FOLIAGE_BANDS,
			self.geometry.crown_floor_world_y(),
			self.geometry.leaf_ball_size(),
			CanopyBallKit::Cheap,
		)
	}
}

impl VegetationComponents for SopesBanyan {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => self.stick_nodes_high(),
			LodSceneLevel::Medium => self.stick_nodes_medium(),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => self.stick_nodes_low(),
		};
		Layers::from_free(nodes).map(|n| n.with_material(chico_stick_material_ref()))
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let nodes = match level {
			LodSceneLevel::High => self.foliage_nodes_high(),
			LodSceneLevel::Medium => self.foliage_nodes_medium(),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => self.foliage_nodes_low(),
		};
		Layers::from_free(nodes).map(|n| n.with_material(chico_leaf_material_ref()))
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(
			StructuralLod::new(self.structural_center(), self.footprint_radius()).with_factors(
				STRUCTURAL_HIGH_FACTOR,
				STRUCTURAL_MEDIUM_FACTOR,
				STRUCTURAL_LOW_FACTOR,
			),
		)
	}
}
