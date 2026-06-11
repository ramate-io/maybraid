pub mod lod;

pub mod mesh;
// Early development caches to be reused by RenderItem developers.
pub mod sdf;

use bevy::prelude::*;
pub use chunk::cascade::CascadeChunk;

/// A logical item that can spawn its constituents into the world.
///
/// # Placement contract
///
/// `transform` is the item's placement **in the caller's space**: world space when spawned
/// top-level, or parent-local when spawned via [`RenderItem::spawn_render_items_under`].
/// Composite items (tree assemblies and similar) should spawn **one root entity** carrying the
/// item as a `Component` plus `transform`, attach their constituents as children with
/// item-local transforms, and return only the root. Bevy's transform propagation then owns
/// world placement — implementations must not bake world offsets into child transforms.
///
/// Per-instance variation (e.g. across a grove) comes from the **seeds** the caller sets on the
/// item's noise parameters, not from spatial offsets baked into the geometry.
pub trait RenderItem: Clone {
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity>;

	/// Spawn with `local_transform` relative to `parent` (when given), attaching the returned
	/// entities as its children.
	fn spawn_render_items_under(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		local_transform: Transform,
		parent: Option<Entity>,
	) -> Vec<Entity> {
		let entities = self.spawn_render_items(commands, cascade_chunk, local_transform);
		if let Some(parent) = parent {
			for entity in &entities {
				commands.entity(*entity).insert(ChildOf(parent));
			}
		}
		entities
	}
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
