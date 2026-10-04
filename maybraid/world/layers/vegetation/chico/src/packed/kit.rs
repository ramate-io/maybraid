//! Resolve shared kit GLBs to one [`Mesh`] handle each.

use std::collections::HashMap;

use bevy::gltf::{Gltf, GltfMesh};
use bevy::prelude::*;

/// Hidden kit loads so packed draws reuse the same mesh assets as SceneRef.
#[derive(Resource, Default)]
pub struct PackedKitMeshes {
	pending: HashMap<&'static str, Handle<Gltf>>,
	ready: HashMap<&'static str, Handle<Mesh>>,
}

impl PackedKitMeshes {
	pub fn request(&mut self, asset_server: &AssetServer, kit: &'static str) {
		if self.ready.contains_key(kit) || self.pending.contains_key(kit) {
			return;
		}
		self.pending.insert(kit, asset_server.load(kit));
	}

	pub fn mesh(&self, kit: &'static str) -> Option<Handle<Mesh>> {
		self.ready.get(kit).cloned()
	}

	pub fn ready_count(&self) -> usize {
		self.ready.len()
	}
}

pub fn resolve_kit_meshes(
	mut kits: ResMut<PackedKitMeshes>,
	selection: Res<crate::packed::select::PackedGroveSelection>,
	asset_server: Res<AssetServer>,
	gltfs: Res<Assets<Gltf>>,
	gltf_meshes: Res<Assets<GltfMesh>>,
) {
	for tile in &selection.tiles {
		for batch in &tile.batches {
			kits.request(&asset_server, batch.key.kit);
		}
	}
	let pending: Vec<_> = kits.pending.iter().map(|(kit, handle)| (*kit, handle.clone())).collect();
	for (kit, handle) in pending {
		let Some(gltf) = gltfs.get(&handle) else {
			continue;
		};
		let Some(mesh) = first_primitive_mesh(gltf, &gltf_meshes) else {
			continue;
		};
		kits.pending.remove(kit);
		kits.ready.insert(kit, mesh);
	}
}

fn first_primitive_mesh(gltf: &Gltf, gltf_meshes: &Assets<GltfMesh>) -> Option<Handle<Mesh>> {
	for mesh in &gltf.meshes {
		let gltf_mesh = gltf_meshes.get(mesh)?;
		if let Some(primitive) = gltf_mesh.primitives.first() {
			return Some(primitive.mesh.clone());
		}
	}
	None
}
