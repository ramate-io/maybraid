//! Axis-mirror rebuild for [`SceneRef`] meshes / worlds.

use bevy::asset::{AssetId, Handle};
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::mesh::Mesh;
use bevy::platform::collections::HashMap;
use bevy::prelude::{Assets, Mesh3d, Transform};
use bevy::world_serialization::WorldAsset;

use crate::scene_ref::MirrorAxis;

/// Clone `mesh`, negate `axis` on positions/normals/tangents, and reverse winding.
pub fn mirror_mesh(mesh: &Mesh, axis: MirrorAxis) -> Mesh {
	let mut out = mesh.clone();
	out.transform_by(Transform::from_scale(axis.scale()));
	// Odd negative scale reverses winding; restore front-face orientation.
	let _ = out.invert_winding();
	out
}

/// Clone `source` and rewrite every `Mesh3d` to a newly registered mirrored mesh.
///
/// Caller must ensure the source handle is
/// [`AssetServer::is_loaded_with_dependencies`] so mesh bytes are in `Assets<Mesh>`.
pub(crate) fn mirror_world_asset(
	source: &WorldAsset,
	axis: MirrorAxis,
	meshes: &mut Assets<Mesh>,
	type_registry: &AppTypeRegistry,
) -> Option<WorldAsset> {
	let mut cloned = source.clone_with(type_registry).ok()?;

	let mut entities = Vec::new();
	for entity in cloned.world.iter_entities() {
		if let Some(mesh3d) = entity.get::<Mesh3d>() {
			entities.push((entity.id(), mesh3d.0.clone()));
		}
	}

	let mut remap: HashMap<AssetId<Mesh>, Handle<Mesh>> = HashMap::default();
	for (entity, old_handle) in entities {
		let new_handle = if let Some(h) = remap.get(&old_handle.id()) {
			h.clone()
		} else {
			let mirrored = mirror_mesh(meshes.get(&old_handle)?, axis);
			let h = meshes.add(mirrored);
			remap.insert(old_handle.id(), h.clone());
			h
		};
		if let Some(mut mesh3d) = cloned.world.get_mut::<Mesh3d>(entity) {
			*mesh3d = Mesh3d(new_handle);
		}
	}

	Some(cloned)
}
