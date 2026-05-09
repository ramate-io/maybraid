use crate::{BallStickChain, BallStickNode, Hysteresis};
use bevy::prelude::*;
use procedural_common::FromScalarNoise;
use render_item::{CascadeChunk, RenderItem};
use std::marker::PhantomData;

pub trait BallRenderRule<R: RenderItem>: Clone {
	fn ball_render_item_for(&self, node: &BallStickNode, hysteresis: &Hysteresis) -> Option<R>;
}

/// A useful common helper for rendering balls.
/// The plan right not to have someone spawn this type directly,
/// but rather as an internal type used in a render item spawn tree.
#[derive(Clone)]
pub struct BallRenderHelper<Item: RenderItem, Rule: BallRenderRule<Item>> {
	rule: Rule,
	chain: BallStickChain,
	__marker: PhantomData<Item>,
}

impl<Item: RenderItem, Rule: BallRenderRule<Item>> BallRenderHelper<Item, Rule> {
	pub fn new(chain: BallStickChain, rule: Rule) -> Self {
		Self { chain, rule, __marker: PhantomData }
	}

	pub fn render_balls(&self) -> Vec<(Item, Transform)> {
		self.chain
			.nodes()
			.map(|node| {
				(
					self.rule.ball_render_item_for(node, &Hysteresis::default()),
					Transform::from_translation(node.position),
				)
			})
			.filter_map(|(item, transform)| item.map(|item| (item, transform)))
			.collect()
	}

	/// Often we'll want to construct this over a chain and then yield it back to the caller.
	pub fn spawn_render_items_and_yield(
		self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> BallStickChain {
		self.spawn_render_items(commands, cascade_chunk, transform);
		self.chain
	}
}

impl<Item: RenderItem, Rule: BallRenderRule<Item>> RenderItem for BallRenderHelper<Item, Rule> {
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		// For now, we typically assume the balls are in world space with respect to some origin.
		// Hence, the transform here is an offset on world space coordinates
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

impl<Item: RenderItem, Rule: BallRenderRule<Item> + FromScalarNoise> BallRenderHelper<Item, Rule> {
	/// Construct a ball renderer from a scalar noise value.
	///
	/// NOTE: this isn't pulled into the scalar hierarchy directly
	/// because BallRenderHelper is a helper, it still needs a particular BallStickChain
	pub fn new_from_noise(
		chain: BallStickChain,
		scalar: f32,
		amplitude: f32,
		frequency: f32,
		octaves: u32,
	) -> Self {
		let rule = Rule::from_scalar(scalar, frequency, amplitude, octaves);
		Self::new(chain, rule)
	}
}
