//! Generic grove [`RenderItem`] helpers ([RFC-183 §3.4.2]).

use std::marker::PhantomData;

use bevy::prelude::*;
use render_item::{CascadeChunk, RenderItem};

use super::{CellGrove, Grove, GroveCellOutcome, GroveExtent, GroveOverspillPolicy, TerrainSample};
use gimme_gen::Cell;

/// One placed grove cell ready for materialization.
#[derive(Debug, Clone, PartialEq)]
pub struct GrovePlacedCell<V> {
	pub variant: V,
	pub position: Vec3,
	pub scale: f32,
}

impl<V: Clone> GrovePlacedCell<V> {
	pub fn new(variant: V, position: Vec3, scale: f32) -> Self {
		Self { variant, position, scale }
	}
}

impl<V: Clone> From<GroveCellOutcome<V>> for Option<GrovePlacedCell<V>> {
	fn from(outcome: GroveCellOutcome<V>) -> Self {
		match outcome {
			GroveCellOutcome::Placed { variant, position, scale } => {
				Some(GrovePlacedCell { variant, position, scale })
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => None,
		}
	}
}

impl<G: CellGrove> Grove<G> {
	/// Run selection on each cell and collect placed outcomes only.
	///
	/// Grove extent is the union of all `cells`, so per-cell overspill stays inside the LOD unit.
	pub fn select_placements(
		&self,
		cells: &[Cell],
		terrain: &impl TerrainSample,
	) -> Vec<GrovePlacedCell<G::Variant>>
	where
		G::Variant: Clone,
	{
		self.select_placements_with_policy(cells, terrain, GroveOverspillPolicy::Discard)
	}

	/// Like [`Self::select_placements`], with an explicit overspill policy.
	pub fn select_placements_with_policy(
		&self,
		cells: &[Cell],
		terrain: &impl TerrainSample,
		overspill_policy: GroveOverspillPolicy,
	) -> Vec<GrovePlacedCell<G::Variant>>
	where
		G::Variant: Clone,
	{
		let grove_extent = GroveExtent::from_cells(cells);
		cells
			.iter()
			.filter_map(|cell| {
				Option::<GrovePlacedCell<G::Variant>>::from(self.select_cell_with_policy(
					cell,
					grove_extent.as_ref(),
					overspill_policy,
					terrain,
				))
			})
			.collect()
	}
}

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
