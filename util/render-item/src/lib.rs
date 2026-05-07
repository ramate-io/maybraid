//! Legacy render dispatch tied to `chunk::CascadeChunk` and optional LOD (`lod` module).
//!
//! New work should use the **`renderit`** crate (`maybraid/rendering/renderit`): generic
//! `RenderItem<Ctx>`, `RenderDispatchSource`, and child-spawning under the dispatch entity.

pub mod lod;

pub mod mesh;
// Early development caches to be reused by RenderItem developers.
pub mod sdf;

use bevy::prelude::*;
use chunk::cascade::CascadeChunk;

/// Used for logical items that can will spawn their constituens into the world.
pub trait RenderItem: Clone {
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity>;
}

/// Signals an intent to render an item into the world.
#[derive(Component, Clone)]
pub struct DispatchRenderItem<T: RenderItem> {
	item: T,
}

/// Spawns the render item to the world.
impl<T: RenderItem> DispatchRenderItem<T> {
	pub fn new(item: T) -> Self {
		Self { item }
	}

	pub fn item(&self) -> &T {
		&self.item
	}

	pub fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		self.item.spawn_render_items(commands, cascade_chunk, transform)
	}
}

/// Handles the render items for a given cascade chunk, assigning them a material by type.
///
/// NOTE: this is not procedural contract for all produce all items of the type.
/// Rather, when a render item is dispatched, this begins the process of rendering said item.
///
/// TODO: this needs to be made event-based.
pub fn render_items<T: RenderItem + Send + Sync + 'static>(
	mut commands: Commands,
	query: Query<
		(Entity, &DispatchRenderItem<T>, &CascadeChunk, &Transform),
		Added<DispatchRenderItem<T>>,
	>,
) {
	for (_entity, dispatch, chunk, transform) in &query {
		dispatch.spawn_render_items(&mut commands, chunk, *transform);
	}
}

pub trait NormalizeChunk {
	/// Normalizes the cascaded chunk to the mesh space.
	///
	/// Some reusable meshes may normalize the chunk space to something like the origin,
	/// then rely on transforms to position the mesh in the world.
	///
	/// Higher order systems are responsible for accounting for whether the mesh is normalized or not.
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		cascade_chunk.clone()
	}
}
