//! Shared original meshes; unsupported scene layouts never lose ordinary visuals.

use crate::packed::instances::SHADOW_KIT;
use bevy::gltf::{Gltf, GltfMesh, GltfNode};
use bevy::mesh::{PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use std::collections::HashMap;
use vegetation_components::VegetationProceduralAssets;

#[derive(Clone)]
pub enum KitState {
	Loading(Handle<Gltf>),
	Ready(Handle<Mesh>),
	Unsupported,
}

#[derive(Resource, Default)]
pub struct PackedKitMeshes {
	pub states: HashMap<&'static str, KitState>,
	pub epoch: u64,
	pub sources: HashMap<&'static str, Handle<Gltf>>,
}

impl PackedKitMeshes {
	pub fn request(&mut self, assets: &AssetServer, kit: &'static str) {
		if kit != SHADOW_KIT {
			self.sources.entry(kit).or_insert_with(|| assets.load(kit));
		}
		self.states.entry(kit).or_insert_with(|| {
			if kit == SHADOW_KIT {
				VegetationProceduralAssets::try_get()
					.map(|assets| KitState::Ready(assets.foliage_ball.clone()))
					.unwrap_or(KitState::Unsupported)
			} else {
				KitState::Loading(assets.load(kit))
			}
		});
	}

	pub fn mesh(&self, kit: &'static str) -> Option<Handle<Mesh>> {
		match self.states.get(kit)? {
			KitState::Ready(mesh) => Some(mesh.clone()),
			_ => None,
		}
	}

	pub fn supports_mesh(mesh: &Mesh) -> bool {
		if mesh.primitive_topology() != PrimitiveTopology::TriangleList
			|| mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_some()
			|| mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).is_some()
			|| mesh.attribute(Mesh::ATTRIBUTE_COLOR).is_some()
			|| mesh.morph_targets().is_some()
		{
			return false;
		}
		matches!(
			mesh.attribute(Mesh::ATTRIBUTE_POSITION),
			Some(VertexAttributeValues::Float32x3(_))
		) && matches!(
			mesh.attribute(Mesh::ATTRIBUTE_NORMAL),
			Some(VertexAttributeValues::Float32x3(_))
		) && mesh.attributes().all(|(_, values)| {
			matches!(
				values,
				VertexAttributeValues::Float32x2(_)
					| VertexAttributeValues::Float32x3(_)
					| VertexAttributeValues::Float32x4(_)
			)
		})
	}
}

pub fn resolve_kit_meshes(
	mut kits: ResMut<PackedKitMeshes>,
	assets: Res<AssetServer>,
	gltfs: Res<Assets<Gltf>>,
	nodes: Res<Assets<GltfNode>>,
	gltf_meshes: Res<Assets<GltfMesh>>,
	meshes: Res<Assets<Mesh>>,
) {
	for state in kits.states.values_mut() {
		let KitState::Loading(handle) = state else { continue };
		let Some(gltf) = gltfs.get(handle.id()) else {
			if assets.load_state(handle.id()).is_failed() {
				*state = KitState::Unsupported;
			}
			continue;
		};
		// The representative orchard kits have one identity node and primitive.
		// Anything richer stays on SceneRef rather than dropping a scene part.
		if gltf.nodes.len() != 1 || gltf.meshes.len() != 1 || gltf.scenes.len() != 1 {
			*state = KitState::Unsupported;
			continue;
		}
		let Some(node) = nodes.get(&gltf.nodes[0]) else { continue };
		if !node.children.is_empty() || node.skin.is_some() || node.transform != Transform::IDENTITY
		{
			*state = KitState::Unsupported;
			continue;
		}
		let Some(gltf_mesh) = gltf_meshes.get(&gltf.meshes[0]) else { continue };
		if gltf_mesh.primitives.len() != 1 || node.mesh.as_ref() != Some(&gltf.meshes[0]) {
			*state = KitState::Unsupported;
			continue;
		}
		let mesh_handle = gltf_mesh.primitives[0].mesh.clone();
		let Some(mesh) = meshes.get(&mesh_handle) else { continue };
		*state = if PackedKitMeshes::supports_mesh(mesh) {
			KitState::Ready(mesh_handle)
		} else {
			KitState::Unsupported
		};
	}
}

/// Hot-reloaded kit content retires the old tile revision before replacement.
pub fn invalidate_changed_kits(
	mut kits: ResMut<PackedKitMeshes>,
	mut gltf_events: MessageReader<AssetEvent<Gltf>>,
	mut mesh_events: MessageReader<AssetEvent<Mesh>>,
) {
	let gltf_changes = gltf_events
		.read()
		.filter_map(|event| match event {
			AssetEvent::Modified { id } | AssetEvent::Removed { id } => Some(*id),
			_ => None,
		})
		.collect::<Vec<_>>();
	let mesh_changes = mesh_events
		.read()
		.filter_map(|event| match event {
			AssetEvent::Modified { id } | AssetEvent::Removed { id } => Some(*id),
			_ => None,
		})
		.collect::<Vec<_>>();
	let dirty = kits
		.states
		.iter()
		.filter(|(kit, state)| {
			kits.sources.get(*kit).is_some_and(|source| gltf_changes.contains(&source.id()))
				|| matches!(state,KitState::Ready(mesh) if mesh_changes.contains(&mesh.id()))
		})
		.map(|(&kit, _)| kit)
		.collect::<Vec<_>>();
	if dirty.is_empty() {
		return;
	}
	kits.epoch = kits.epoch.wrapping_add(1);
	for kit in dirty {
		if let Some(source) = kits.sources.get(kit).cloned() {
			kits.states.insert(kit, KitState::Loading(source));
		}
	}
}
