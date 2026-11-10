//! Shared original meshes; unsupported scene layouts never lose ordinary visuals.

use crate::packed::instances::SHADOW_KIT;
use bevy::gltf::{Gltf, GltfMesh, GltfNode};
use bevy::mesh::{PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;
use bevy::shape::Aabb3d;
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
	pub local_bounds: HashMap<&'static str, Aabb3d>,
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

	/// Exact POSITION AABB for the accepted CPU mesh, cached per kit revision.
	pub fn bounds_for_mesh(mesh: &Mesh) -> Option<Aabb3d> {
		let Some(VertexAttributeValues::Float32x3(positions)) =
			mesh.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			return None;
		};
		if positions.is_empty() {
			return None;
		}
		let mut min = Vec3::splat(f32::INFINITY);
		let mut max = Vec3::splat(f32::NEG_INFINITY);
		for position in positions {
			let position = Vec3::from_array(*position);
			if !position.is_finite() {
				return None;
			}
			min = min.min(position);
			max = max.max(position);
		}
		Some(Aabb3d::from_min_max(min, max))
	}

	pub fn ready_bounds(&self, kit: &'static str) -> Option<Aabb3d> {
		self.mesh(kit)?;
		self.local_bounds.get(kit).copied()
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
		// Supported shared kits have one identity node and primitive.
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
	let kits = &mut *kits;
	for (&kit, state) in &mut kits.states {
		if kits.local_bounds.contains_key(kit) {
			continue;
		}
		let KitState::Ready(handle) = state else { continue };
		let Some(mesh) = meshes.get(handle.id()) else { continue };
		match PackedKitMeshes::bounds_for_mesh(mesh) {
			Some(bounds) => {
				kits.local_bounds.insert(kit, bounds);
			}
			None => {
				*state = KitState::Unsupported;
			}
		}
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
	let mut removed_meshes = std::collections::HashSet::new();
	let mesh_changes = mesh_events
		.read()
		.filter_map(|event| match event {
			AssetEvent::Modified { id } => Some(*id),
			AssetEvent::Removed { id } => {
				removed_meshes.insert(*id);
				Some(*id)
			}
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
		kits.local_bounds.remove(kit);
		if let Some(source) = kits.sources.get(kit).cloned() {
			kits.states.insert(kit, KitState::Loading(source));
		} else if matches!(kits.states.get(kit),Some(KitState::Ready(mesh)) if removed_meshes.contains(&mesh.id()))
		{
			kits.states.insert(kit, KitState::Unsupported);
		}
	}
}
