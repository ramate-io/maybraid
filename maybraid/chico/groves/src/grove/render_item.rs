//! Generic grove [`RenderItem`] helpers ([RFC-183 §3.4.2]).

use std::marker::PhantomData;

use bevy::prelude::*;
use render_item::{CascadeChunk, RenderItem};

use super::GrovePlacedCell;

/// Maps one [`GrovePlacedCell`] to a renderable item and local transform.
pub trait GroveRenderRule<Item, V>: Clone {
	fn render_item_for(&self, placed: &GrovePlacedCell<V>) -> Option<(Item, Transform)>;
}

/// Iterates grove placements and spawns child [`RenderItem`] instances.
#[derive(Clone)]
pub struct GroveRenderHelper<Item: RenderItem, V: Clone, Rule: GroveRenderRule<Item, V>> {
	rule: Rule,
	placements: Vec<GrovePlacedCell<V>>,
	__marker: PhantomData<Item>,
}

impl<Item: RenderItem, V: Clone, Rule: GroveRenderRule<Item, V>> GroveRenderHelper<Item, V, Rule> {
	pub fn new(placements: Vec<GrovePlacedCell<V>>, rule: Rule) -> Self {
		Self { rule, placements, __marker: PhantomData }
	}

	pub fn placements(&self) -> &[GrovePlacedCell<V>] {
		&self.placements
	}

	pub fn render_placements(&self) -> impl Iterator<Item = (Item, Transform)> + '_ {
		self.placements.iter().filter_map(|placed| self.rule.render_item_for(placed))
	}
}

impl<Item: RenderItem, V: Clone, Rule: GroveRenderRule<Item, V>> RenderItem
	for GroveRenderHelper<Item, V, Rule>
{
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		self.render_placements()
			.flat_map(|(item, inner_transform)| {
				item.spawn_render_items(
					commands,
					cascade_chunk,
					transform.mul_transform(inner_transform),
				)
			})
			.collect()
	}
}
