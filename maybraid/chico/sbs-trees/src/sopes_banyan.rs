//! **Sope's Banyan** — end-to-end tree assembly for Chico ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252)).
//!
//! Emits [`StickNode`] / [`FoliageNode`] via [`VegetationComponents`]. Present with
//! [`ComponentsOnly`](chico_vegetation_components::ComponentsOnly). The legacy
//! [`RenderItem`] path is unimplemented — use the vegetation LodScene adapter or `/show`.

mod canopy;
pub mod render_item_plugin;
mod stick;

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, SopesBanyanChain, SopesBanyanSbs};
use chico_vegetation_components::{
	FoliageNode, Layers, StickNode, VegetationComponents,
};
use clap::Args;
use lod::gen::LodSceneLevel;
use render_item::{CascadeChunk, RenderItem};

use canopy::foliage_node_for_terminal;
use stick::stick_node_for_segment;

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

	fn stick_nodes(&self) -> Vec<StickNode> {
		let chain = self.build_chain();
		chain
			.segments_with_hysteresis()
			.filter_map(|(segment, _, _)| stick_node_for_segment(&segment))
			.collect()
	}

	fn foliage_nodes(&self) -> Vec<FoliageNode> {
		let chain = self.build_chain();
		let min_height = self.geometry.crown_floor_world_y();
		let leaf_radius_world = self.geometry.leaf_ball_size();
		chain
			.nodes_with_hysteresis_enumerated()
			.filter_map(|(node_idx, node, h)| {
				foliage_node_for_terminal(node_idx, node, h, min_height, leaf_radius_world)
			})
			.collect()
	}
}

impl VegetationComponents for SopesBanyan {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::from_free(self.stick_nodes())
	}

	fn foliage_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FoliageNode> {
		Layers::from_free(self.foliage_nodes())
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
