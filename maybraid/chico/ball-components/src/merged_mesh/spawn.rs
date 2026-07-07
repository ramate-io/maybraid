//! Spawn merged mesh geometry on a single marker entity.
//!
//! Mesh vertices are authored at unit scale; sizing and placement use the entity [`Transform`]
//! so parent assemblies can reparent or reposition foliage without rebaking geometry.

use bevy::prelude::*;
use render_item::CascadeChunk;

/// Spawns `item` at `local_transform`, optionally as a child of `parent`.
///
/// `mesh` must already be built at unit scale (call [`MergedTuft::build_mesh`](crate::tuft::spawn::MergedTuft::build_mesh)
/// / [`MergedFrond::build_mesh`](crate::frond::spawn::MergedFrond::build_mesh) with `1.0`).
pub(crate) fn spawn_merged_mesh<I, M, S>(
	item: &I,
	mesh: Mesh,
	material: S,
	commands: &mut Commands,
	cascade_chunk: &CascadeChunk,
	local_transform: Transform,
	parent: Option<Entity>,
) -> Entity
where
	I: Component + Clone + Send + Sync + 'static,
	M: Material + Send + Sync + 'static,
	S: Clone + Into<MeshMaterial3d<M>> + Send + Sync + 'static,
{
	let item = item.clone();
	let bundle = (item, cascade_chunk.clone(), local_transform, Visibility::default());

	let root = match parent {
		Some(parent) => {
			let mut root = Entity::PLACEHOLDER;
			commands.entity(parent).with_children(|parent_cmd| {
				root = parent_cmd.spawn(bundle).id();
			});
			root
		}
		None => commands.spawn(bundle).id(),
	};

	commands.queue(move |world: &mut World| {
		let mesh_handle = world.resource_mut::<Assets<Mesh>>().add(mesh);
		let mesh_material: MeshMaterial3d<M> = material.into();
		world.entity_mut(root).insert((Mesh3d(mesh_handle), mesh_material));
	});

	root
}
