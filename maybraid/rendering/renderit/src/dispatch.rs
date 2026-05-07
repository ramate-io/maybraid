//! [`DispatchRenderItem`] and [`RenderDispatchSource`]: generic dispatch handling (same query-shape
//! idea as `CascadeProductionSource` in the `lod-cascade-system` crate).

use bevy::ecs::query::{QueryData, QueryFilter};
use bevy::prelude::*;

/// Logical render payload: spawns **response entities as children** of the dispatching entity.
///
/// `Ctx` is whatever sibling (or joined) query data you attach to the same row—see
/// [`RenderDispatchSource`].
pub trait RenderItem<Ctx>: Clone + Send + Sync + 'static {
	/// Spawn all follow-up entities as **children** of `dispatch_entity`.
	fn spawn_render_items(&self, commands: &mut Commands, dispatch_entity: Entity, ctx: &Ctx);
}

/// Starts a render handling chain when this component is added (or matched by your
/// [`RenderDispatchSource::QueryFilter`](RenderDispatchSource::QueryFilter)).
#[derive(Component, Clone)]
pub struct DispatchRenderItem<T> {
	item: T,
}

impl<T> DispatchRenderItem<T> {
	pub fn new(item: T) -> Self {
		Self { item }
	}

	pub fn item(&self) -> &T {
		&self.item
	}

	pub fn into_inner(self) -> T {
		self.item
	}

	/// Runs [`RenderItem::spawn_render_items`] so all spawned entities are parented under
	/// `dispatch_entity`.
	pub fn spawn_children<Ctx>(&self, commands: &mut Commands, dispatch_entity: Entity, ctx: &Ctx)
	where
		T: RenderItem<Ctx>,
	{
		self.item.spawn_render_items(commands, dispatch_entity, ctx);
	}
}

/// Describes **one query row** that carries a dispatch plus the context type `Ctx` used by
/// [`RenderItem::spawn_render_items`].
///
/// This mirrors `CascadeProductionSource` in `lod-cascade-system`: you choose `QueryData` instead
/// of hard-coding `CascadeChunk` or `Transform`.
///
/// Cloning accessors avoid long-lived borrows into the query item when driving [`Commands`].
pub trait RenderDispatchSource: Send + Sync + 'static {
	type Item: RenderItem<Self::Context> + Clone + Send + Sync + 'static;
	type Context: Clone + Send + Sync + 'static;
	type QueryData: QueryData;
	type QueryFilter: QueryFilter + Send + Sync + 'static;

	fn dispatch_entity(
		item: &<<Self::QueryData as QueryData>::ReadOnly as QueryData>::Item<'_, '_>,
	) -> Entity;

	fn dispatch_clone(
		item: &<<Self::QueryData as QueryData>::ReadOnly as QueryData>::Item<'_, '_>,
	) -> DispatchRenderItem<Self::Item>;

	fn context_clone(
		item: &<<Self::QueryData as QueryData>::ReadOnly as QueryData>::Item<'_, '_>,
	) -> Self::Context;
}

/// Runs [`RenderDispatchSource`] rows once per matching query tick and parents spawns under the
/// dispatch entity.
pub fn process_render_dispatches<S: RenderDispatchSource>(
	mut commands: Commands,
	query: Query<S::QueryData, S::QueryFilter>,
) {
	for row in &query {
		let parent = S::dispatch_entity(&row);
		let dispatch = S::dispatch_clone(&row);
		let ctx = S::context_clone(&row);
		dispatch.spawn_children(&mut commands, parent, &ctx);
	}
}

/// Convenience when the dispatching entity is [`RenderDispatchSource::dispatch_entity`] and the
/// row is `(Entity, &DispatchRenderItem<Item>, &Ctx)` with default `Added` filter.
pub fn process_render_dispatches_simple<Item, Ctx>(
	mut commands: Commands,
	query: Query<(Entity, &DispatchRenderItem<Item>, &Ctx), Added<DispatchRenderItem<Item>>>,
) where
	Item: RenderItem<Ctx> + Clone + Send + Sync + 'static,
	Ctx: Component + Send + Sync + 'static,
{
	for (entity, dispatch, ctx) in &query {
		dispatch.spawn_children(&mut commands, entity, ctx);
	}
}
