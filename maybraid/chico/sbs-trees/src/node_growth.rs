//! Spawn [`RenderItem`] instances at ball-stick graph nodes.

use bevy::prelude::*;
use chico_sbs_geometry::{BallStickChain, BallStickNode, Hysteresis};
use render_item::{CascadeChunk, RenderItem};
use std::marker::PhantomData;

pub trait NodeGrowthRule<R: RenderItem, H: Hysteresis>: Clone {
	fn growth_at_node(
		&self,
		node_idx: usize,
		node: &BallStickNode,
		hysteresis: &H,
		chain: &BallStickChain<H>,
	) -> Option<(R, Transform)>;
}

#[derive(Clone)]
pub struct NodeGrowthRenderHelper<Item: RenderItem, Rule: NodeGrowthRule<Item, H>, H: Hysteresis> {
	rule: Rule,
	chain: BallStickChain<H>,
	__marker: PhantomData<Item>,
}

impl<Item: RenderItem, Rule: NodeGrowthRule<Item, H>, H: Hysteresis>
	NodeGrowthRenderHelper<Item, Rule, H>
{
	pub fn new(chain: BallStickChain<H>, rule: Rule) -> Self {
		Self { chain, rule, __marker: PhantomData }
	}
}

impl<Item: RenderItem, Rule: NodeGrowthRule<Item, H>, H: Hysteresis> RenderItem
	for NodeGrowthRenderHelper<Item, Rule, H>
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		self.chain
			.nodes_with_hysteresis_enumerated()
			.filter_map(|(node_idx, node, h)| {
				self.rule.growth_at_node(node_idx, node, h, &self.chain).map(|(item, inner)| {
					item.spawn_render_items(commands, cascade_chunk, inner.mul_transform(transform))
				})
			})
			.flatten()
			.collect()
	}
}
