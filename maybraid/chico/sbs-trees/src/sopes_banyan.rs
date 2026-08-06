//! **Sope's Banyan** — end-to-end tree assembly for Chico ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252)).
//!
//! Emits [`StickNode`] / [`FoliageNode`] via [`VegetationComponents`]. Present with
//! [`ComponentsOnly`](chico_vegetation_components::ComponentsOnly). The legacy
//! [`RenderItem`] path is unimplemented — use the vegetation LodScene adapter or `/show`.
//!
//! Structural LOD (tree-radius bands):
//! - **High** — within `3 ×` tree radius: full sticks + full canopy
//! - **Medium** — `3…12 ×` radius: trunk + outer-half sticks; azimuth×height outer foliage
//! - **Low** — `12…24 ×` radius: trunk + ~1/4 descenders; coarser azimuth×height foliage

mod canopy;
pub mod render_item_plugin;
mod stick;

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, SopesBanyanChain, SopesBanyanSbs};
use chico_vegetation_components::{
	FoliageNode, Layers, StickNode, VegetationComponents, VegetationStructuralLodProbe,
};
use clap::Args;
use lod::gen::LodSceneLevel;
use render_item::{CascadeChunk, RenderItem};

use canopy::{
	banded_outer_canopy_balls, foliage_node_for_terminal, LOW_FOLIAGE_BANDS, MEDIUM_FOLIAGE_BANDS,
};
use stick::{
	keep_stick_on_low, keep_stick_on_medium, stick_node_for_segment, stick_role_for_segment,
};

/// Typical Sope's Banyan (geometry-only; materials are patched externally later).
pub type SopesBanyanStd = SopesBanyan;

#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct SopesBanyan {
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: SopesBanyanSbs,
}

impl Default for SopesBanyan {
	fn default() -> Self {
		Self { geometry: SopesBanyanSbs::default() }
	}
}

impl SopesBanyan {
	pub fn build_chain(&self) -> BallStickChain<SopesBanyanChain> {
		self.geometry.build_chain()
	}

	/// Footprint radius: max horizontal distance of any chain node from the trunk axis.
	fn tree_radius(&self, chain: &BallStickChain<SopesBanyanChain>) -> f32 {
		let mut r = self.geometry.scale.stalk_base_radius.max(1e-3);
		for node in &chain.nodes {
			let horiz = Vec2::new(node.position.x, node.position.z).length();
			r = r.max(horiz);
		}
		r
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.geometry.scale.stalk_height * 0.5, 0.0)
	}

	fn stick_nodes_high(&self, chain: &BallStickChain<SopesBanyanChain>) -> Vec<StickNode> {
		chain
			.segments_with_hysteresis()
			.filter_map(|(segment, _, _)| stick_node_for_segment(&segment))
			.collect()
	}

	fn stick_nodes_medium(&self, chain: &BallStickChain<SopesBanyanChain>) -> Vec<StickNode> {
		let tree_radius = self.tree_radius(chain);
		chain
			.segments_with_hysteresis()
			.filter_map(|(segment, parent, _)| {
				let role = stick_role_for_segment(&segment, parent);
				if !keep_stick_on_medium(role, &segment, tree_radius) {
					return None;
				}
				stick_node_for_segment(&segment)
			})
			.collect()
	}

	fn stick_nodes_low(&self, chain: &BallStickChain<SopesBanyanChain>) -> Vec<StickNode> {
		let mut descender_index = 0usize;
		chain
			.segments_with_hysteresis()
			.filter_map(|(segment, parent, _)| {
				let role = stick_role_for_segment(&segment, parent);
				if !keep_stick_on_low(role, &mut descender_index) {
					return None;
				}
				stick_node_for_segment(&segment)
			})
			.collect()
	}

	fn foliage_nodes_high(&self, chain: &BallStickChain<SopesBanyanChain>) -> Vec<FoliageNode> {
		let min_height = self.geometry.crown_floor_world_y();
		let leaf_radius_world = self.geometry.leaf_ball_size();
		chain
			.nodes_with_hysteresis_enumerated()
			.filter_map(|(node_idx, node, h)| {
				foliage_node_for_terminal(node_idx, node, h, min_height, leaf_radius_world)
			})
			.collect()
	}

	fn foliage_nodes_medium(&self, chain: &BallStickChain<SopesBanyanChain>) -> Vec<FoliageNode> {
		let high = self.foliage_nodes_high(chain);
		banded_outer_canopy_balls(&high, MEDIUM_FOLIAGE_BANDS)
	}

	fn foliage_nodes_low(&self, chain: &BallStickChain<SopesBanyanChain>) -> Vec<FoliageNode> {
		let high = self.foliage_nodes_high(chain);
		banded_outer_canopy_balls(&high, LOW_FOLIAGE_BANDS)
	}
}

impl VegetationComponents for SopesBanyan {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let chain = self.build_chain();
		let nodes = match level {
			LodSceneLevel::High => self.stick_nodes_high(&chain),
			LodSceneLevel::Medium => self.stick_nodes_medium(&chain),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => self.stick_nodes_low(&chain),
		};
		Layers::from_free(nodes)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let chain = self.build_chain();
		let nodes = match level {
			LodSceneLevel::High => self.foliage_nodes_high(&chain),
			LodSceneLevel::Medium => self.foliage_nodes_medium(&chain),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => self.foliage_nodes_low(&chain),
		};
		Layers::from_free(nodes)
	}

	fn structural_lod_probe(&self) -> Option<VegetationStructuralLodProbe> {
		let chain = self.build_chain();
		Some(VegetationStructuralLodProbe::new(
			self.structural_center(),
			self.tree_radius(&chain),
		))
	}
}

impl RenderItem for SopesBanyan {
	fn spawn_render_items(
		&self,
		_commands: &mut Commands,
		_cascade_chunk: &CascadeChunk,
		_transform: Transform,
	) -> Vec<Entity> {
		unimplemented!(
			"SopesBanyan RenderItem removed — use VegetationComponents / ComponentsOnly / spawn_vegetation_components (playground: /show sopes-banyan)"
		);
	}
}
