use crate::{BallStickChain, BallStickNode, Hysteresis};
use bevy::prelude::*;
use procedural_common::FromScalarNoise;
use render_item::{CascadeChunk, RenderItem};
use std::marker::PhantomData;

pub trait BallRenderRule<R: RenderItem, H: Hysteresis>: Clone {
	fn ball_render_item_for(&self, node: &BallStickNode, hysteresis: &H) -> Option<R>;
}

/// A useful common helper for rendering balls.
/// The plan right not to have someone spawn this type directly,
/// but rather as an internal type used in a render item spawn tree.
#[derive(Clone)]
pub struct BallRenderHelper<Item: RenderItem, Rule: BallRenderRule<Item, H>, H: Hysteresis> {
	rule: Rule,
	chain: BallStickChain<H>,
	__marker: PhantomData<Item>,
}

impl<Item: RenderItem, Rule: BallRenderRule<Item, H>, H: Hysteresis> BallRenderHelper<Item, Rule, H> {
	pub fn new(chain: BallStickChain<H>, rule: Rule) -> Self {
		Self { chain, rule, __marker: PhantomData }
	}

	pub fn render_balls(&self) -> Vec<(Item, Transform)> {
		self.chain
			.nodes_with_hysteresis()
			.filter_map(|(node, h)| {
				self.rule.ball_render_item_for(node, h).map(|item| {
					(item, Transform::from_translation(node.position))
				})
			})
			.collect()
	}

	/// Often we'll want to construct this over a chain and then yield it back to the caller.
	pub fn spawn_render_items_and_yield(
		self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> BallStickChain<H> {
		self.spawn_render_items(commands, cascade_chunk, transform);
		self.chain
	}
}

impl<Item: RenderItem, Rule: BallRenderRule<Item, H>, H: Hysteresis> RenderItem
	for BallRenderHelper<Item, Rule, H>
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		self.render_balls()
			.into_iter()
			.map(|(item, inner_transform)| {
				item.spawn_render_items(
					commands,
					cascade_chunk,
					Transform::from_translation(
						transform.translation + inner_transform.translation,
					),
				)
			})
			.flatten()
			.collect()
	}
}

impl<Item: RenderItem, Rule: BallRenderRule<Item, H> + FromScalarNoise, H: Hysteresis>
	BallRenderHelper<Item, Rule, H>
{
	/// Construct a ball renderer from a scalar noise value.
	///
	/// NOTE: this isn't pulled into the scalar hierarchy directly
	/// because BallRenderHelper is a helper, it still needs a particular BallStickChain
	pub fn new_from_noise(
		chain: BallStickChain<H>,
		scalar: f32,
		amplitude: f32,
		frequency: f32,
		octaves: u32,
	) -> Self {
		let rule = Rule::from_scalar(scalar, frequency, amplitude, octaves);
		Self::new(chain, rule)
	}
}
