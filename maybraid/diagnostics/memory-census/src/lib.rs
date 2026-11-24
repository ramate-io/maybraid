//! Explicit, read-only CPU payload census. No plugin, recurring system or logging.
//!
//! Counts selected owned Vec payloads, not allocator overhead, GPU data or RSS.

use bevy::asset::AssetId;
use bevy::mesh::{Indices, VertexAttributeValues};
use bevy::prelude::*;
use serde::Serialize;
use std::collections::{HashMap, HashSet};

/// Initialized bytes and reserved Vec element storage, excluding allocator overhead.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PayloadBytes {
	pub logical_bytes: usize,
	pub capacity_bytes: usize,
}

impl PayloadBytes {
	fn of_vec<T>(values: &Vec<T>) -> Self {
		Self {
			logical_bytes: values.len() * size_of::<T>(),
			capacity_bytes: values.capacity() * size_of::<T>(),
		}
	}

	fn add(&mut self, other: Self) {
		self.logical_bytes += other.logical_bytes;
		self.capacity_bytes += other.capacity_bytes;
	}

	fn of_attribute(values: &VertexAttributeValues) -> Self {
		match values {
			VertexAttributeValues::Uint8(values) => Self::of_vec(values),
			VertexAttributeValues::Uint8x2(values) => Self::of_vec(values),
			VertexAttributeValues::Uint8x4(values) => Self::of_vec(values),
			VertexAttributeValues::Sint8(values) => Self::of_vec(values),
			VertexAttributeValues::Sint8x2(values) => Self::of_vec(values),
			VertexAttributeValues::Sint8x4(values) => Self::of_vec(values),
			VertexAttributeValues::Unorm8(values) => Self::of_vec(values),
			VertexAttributeValues::Unorm8x2(values) => Self::of_vec(values),
			VertexAttributeValues::Unorm8x4(values) => Self::of_vec(values),
			VertexAttributeValues::Snorm8(values) => Self::of_vec(values),
			VertexAttributeValues::Snorm8x2(values) => Self::of_vec(values),
			VertexAttributeValues::Snorm8x4(values) => Self::of_vec(values),
			VertexAttributeValues::Uint16(values) => Self::of_vec(values),
			VertexAttributeValues::Uint16x2(values) => Self::of_vec(values),
			VertexAttributeValues::Uint16x4(values) => Self::of_vec(values),
			VertexAttributeValues::Sint16(values) => Self::of_vec(values),
			VertexAttributeValues::Sint16x2(values) => Self::of_vec(values),
			VertexAttributeValues::Sint16x4(values) => Self::of_vec(values),
			VertexAttributeValues::Unorm16(values) => Self::of_vec(values),
			VertexAttributeValues::Unorm16x2(values) => Self::of_vec(values),
			VertexAttributeValues::Unorm16x4(values) => Self::of_vec(values),
			VertexAttributeValues::Snorm16(values) => Self::of_vec(values),
			VertexAttributeValues::Snorm16x2(values) => Self::of_vec(values),
			VertexAttributeValues::Snorm16x4(values) => Self::of_vec(values),
			VertexAttributeValues::Float16(values) => Self::of_vec(values),
			VertexAttributeValues::Float16x2(values) => Self::of_vec(values),
			VertexAttributeValues::Float16x4(values) => Self::of_vec(values),
			VertexAttributeValues::Float32(values) => Self::of_vec(values),
			VertexAttributeValues::Float32x2(values) => Self::of_vec(values),
			VertexAttributeValues::Float32x3(values) => Self::of_vec(values),
			VertexAttributeValues::Float32x4(values) => Self::of_vec(values),
			VertexAttributeValues::Uint32(values) => Self::of_vec(values),
			VertexAttributeValues::Uint32x2(values) => Self::of_vec(values),
			VertexAttributeValues::Uint32x3(values) => Self::of_vec(values),
			VertexAttributeValues::Uint32x4(values) => Self::of_vec(values),
			VertexAttributeValues::Sint32(values) => Self::of_vec(values),
			VertexAttributeValues::Sint32x2(values) => Self::of_vec(values),
			VertexAttributeValues::Sint32x3(values) => Self::of_vec(values),
			VertexAttributeValues::Sint32x4(values) => Self::of_vec(values),
			VertexAttributeValues::Float64(values) => Self::of_vec(values),
			VertexAttributeValues::Float64x2(values) => Self::of_vec(values),
			VertexAttributeValues::Float64x3(values) => Self::of_vec(values),
			VertexAttributeValues::Float64x4(values) => Self::of_vec(values),
			VertexAttributeValues::Unorm10_10_10_2(values) => Self::of_vec(values),
			VertexAttributeValues::Unorm8x4Bgra(values) => Self::of_vec(values),
		}
	}

	fn of_mesh(mesh: &Mesh) -> Self {
		let mut bytes = Self::default();
		for (_, values) in mesh.attributes() {
			bytes.add(Self::of_attribute(values));
		}
		if let Some(indices) = mesh.indices() {
			bytes.add(match indices {
				Indices::U16(values) => Self::of_vec(values),
				Indices::U32(values) => Self::of_vec(values),
			});
		}
		bytes
	}
}

#[derive(Debug, Default, Serialize)]
pub struct EcsCensus {
	pub live_entities: usize,
	/// Includes empty archetypes retained by Bevy.
	pub archetypes: usize,
	pub nonempty_archetypes: usize,
	pub tables: usize,
	/// Entity slots in dense tables; not bytes or archetype capacity.
	pub table_entity_capacity: usize,
	/// Inline initialized component storage only, excluding nested heap allocations.
	pub component_logical_inline_bytes: usize,
	pub largest_components: Vec<ComponentCensus>,
}

#[derive(Debug, Serialize)]
pub struct ComponentCensus {
	/// Identity within this World only; not stable across runs.
	pub component_id: usize,
	pub name: String,
	pub instances: usize,
	pub inline_bytes_per_instance: usize,
	pub logical_inline_bytes: usize,
}

#[derive(Debug, Default, Serialize)]
pub struct MeshCensus {
	/// Unique AssetIds in main-world Assets<Mesh>, including assets without live Mesh3d.
	pub assets: usize,
	pub attribute_and_index_payload: PayloadBytes,
	pub mesh3d_references: usize,
	pub unique_mesh3d_ids: usize,
	pub mesh3d_ids_with_cpu_asset: usize,
	pub mesh3d_referenced_payload: PayloadBytes,
}

#[derive(Debug, Default, Serialize)]
pub struct ImageCensus {
	pub assets: usize,
	pub assets_with_cpu_data: usize,
	pub data_payload: PayloadBytes,
}

#[derive(Debug, Serialize)]
pub struct MemoryCensus {
	pub schema_version: u32,
	pub scope: &'static str,
	pub ecs: EcsCensus,
	/// None means the Assets resource is absent, rather than zero assets.
	pub meshes: Option<MeshCensus>,
	pub images: Option<ImageCensus>,
	pub handle_caches: Vec<HandleCacheCensus>,
}

#[derive(Debug, Serialize)]
pub struct HandleCacheCensus {
	pub label: String,
	pub status: &'static str,
	pub entries: Option<usize>,
	pub entry_capacity: Option<usize>,
	pub strong_handle_entries: Option<usize>,
	pub unique_mesh_ids: Option<usize>,
	pub ids_with_cpu_asset: usize,
	pub cpu_payload: PayloadBytes,
	/// Absence from Mesh3d is not proof that only the cache owns the mesh.
	pub ids_without_mesh3d: usize,
}

impl MemoryCensus {
	/// Add one distinct mailbox, not each Arc alias of that mailbox. This does not
	/// infer other owners from handle Arc counts or claim that assets are evictable.
	pub fn add_handle_cache(
		&mut self,
		world: &World,
		label: impl Into<String>,
		snapshot: Option<render_item::mesh::cache::handle::map::HandleMapSnapshot>,
	) {
		let mut cache = HandleCacheCensus {
			label: label.into(),
			status: "busy_or_poisoned",
			entries: None,
			entry_capacity: None,
			strong_handle_entries: None,
			unique_mesh_ids: None,
			ids_with_cpu_asset: 0,
			cpu_payload: PayloadBytes::default(),
			ids_without_mesh3d: 0,
		};
		if let Some(snapshot) = snapshot {
			cache.status = "sampled";
			cache.entries = Some(snapshot.entries);
			cache.entry_capacity = Some(snapshot.entry_capacity);
			cache.strong_handle_entries = Some(snapshot.strong_handle_entries);
			cache.unique_mesh_ids = Some(snapshot.unique_mesh_ids.len());
			let live_ids: HashSet<_> = world
				.iter_entities()
				.filter_map(|e| e.get::<Mesh3d>().map(|m| m.0.id()))
				.collect();
			cache.ids_without_mesh3d = snapshot.unique_mesh_ids.difference(&live_ids).count();
			if let Some(assets) = world.get_resource::<Assets<Mesh>>() {
				for id in snapshot.unique_mesh_ids {
					if let Some(mesh) = assets.get(id) {
						cache.ids_with_cpu_asset += 1;
						cache.cpu_payload.add(PayloadBytes::of_mesh(mesh));
					}
				}
			}
		}
		self.handle_caches.push(cache);
	}

	/// O(entities + assets + attributes). Call only at an explicit diagnostic boundary.
	/// Shared mesh handles are counted once per AssetId, never once per owner.
	pub fn capture(world: &World) -> Self {
		let archetypes = world.archetypes();
		let tables = &world.storages().tables;
		let mut component_counts = HashMap::new();
		for archetype in archetypes.iter() {
			for id in archetype.iter_components() {
				*component_counts.entry(id).or_insert(0_usize) += archetype.len() as usize;
			}
		}
		let mut largest_components: Vec<_> = component_counts
			.into_iter()
			.filter_map(|(id, instances)| {
				let info = world.components().get_info(id)?;
				let inline_bytes_per_instance = info.layout().size();
				Some(ComponentCensus {
					component_id: id.index(),
					name: info.name().to_string(),
					instances,
					inline_bytes_per_instance,
					logical_inline_bytes: instances * inline_bytes_per_instance,
				})
			})
			.collect();
		let component_logical_inline_bytes =
			largest_components.iter().map(|c| c.logical_inline_bytes).sum();
		largest_components.sort_by(|a, b| {
			b.logical_inline_bytes
				.cmp(&a.logical_inline_bytes)
				.then_with(|| a.name.cmp(&b.name))
				.then_with(|| a.component_id.cmp(&b.component_id))
		});
		largest_components.truncate(32);
		let ecs = EcsCensus {
			live_entities: archetypes.iter().map(|a| a.len() as usize).sum(),
			archetypes: archetypes.len(),
			nonempty_archetypes: archetypes.iter().filter(|a| !a.is_empty()).count(),
			tables: tables.iter().count(),
			table_entity_capacity: tables.iter().map(|t| t.capacity()).sum(),
			component_logical_inline_bytes,
			largest_components,
		};
		let meshes = world.get_resource::<Assets<Mesh>>().map(|assets| {
			let mut census = MeshCensus::default();
			let mut live_ids = HashSet::<AssetId<Mesh>>::new();
			for entity in world.iter_entities() {
				if let Some(mesh) = entity.get::<Mesh3d>() {
					census.mesh3d_references += 1;
					live_ids.insert(mesh.0.id());
				}
			}
			census.unique_mesh3d_ids = live_ids.len();
			// Assets::iter visits each AssetId once, regardless of strong-handle aliases.
			for (id, mesh) in assets.iter() {
				let bytes = PayloadBytes::of_mesh(mesh);
				census.assets += 1;
				census.attribute_and_index_payload.add(bytes);
				if live_ids.contains(&id) {
					census.mesh3d_ids_with_cpu_asset += 1;
					census.mesh3d_referenced_payload.add(bytes);
				}
			}
			census
		});
		let images = world.get_resource::<Assets<Image>>().map(|assets| {
			let mut census = ImageCensus::default();
			for (_, image) in assets.iter() {
				census.assets += 1;
				if let Some(data) = &image.data {
					census.assets_with_cpu_data += 1;
					census.data_payload.add(PayloadBytes::of_vec(data));
				}
			}
			census
		});
		Self {
			schema_version: 1,
			scope:
				"selected main-world CPU Vec payloads; not heap totals, GPU bytes, RSS or footprint",
			ecs,
			meshes,
			images,
			handle_caches: Vec::new(),
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::*;
	use bevy::asset::RenderAssetUsages;
	use bevy::mesh::PrimitiveTopology;

	#[test]
	fn shared_mesh_is_counted_once_and_capacity_is_not_logical_length() -> anyhow::Result<()> {
		let mut world = World::new();
		let mut meshes = Assets::<Mesh>::default();
		let mut positions = Vec::with_capacity(16);
		positions.extend([[0.0_f32; 3], [1.0; 3], [2.0; 3]]);
		let position_capacity = positions.capacity() * size_of::<[f32; 3]>();
		let mut indices = Vec::with_capacity(12);
		indices.extend([0_u16, 1, 2]);
		let index_capacity = indices.capacity() * size_of::<u16>();
		let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD);
		mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
		mesh.insert_indices(Indices::U16(indices));
		let handle = meshes.add(mesh);
		world.insert_resource(meshes);
		world.spawn(Mesh3d(handle.clone()));
		world.spawn(Mesh3d(handle));
		let census = MemoryCensus::capture(&world);
		let meshes = census.meshes.ok_or_else(|| anyhow::anyhow!("missing mesh census"))?;
		assert_eq!(meshes.assets, 1);
		assert_eq!(meshes.mesh3d_references, 2);
		assert_eq!(meshes.unique_mesh3d_ids, 1);
		assert_eq!(meshes.attribute_and_index_payload.logical_bytes, 42);
		assert_eq!(
			meshes.attribute_and_index_payload.capacity_bytes,
			position_capacity + index_capacity
		);
		assert_eq!(meshes.mesh3d_referenced_payload, meshes.attribute_and_index_payload);
		// Bevy also owns entities for world infrastructure; count all spawned
		// entities independently, while the Mesh3d ownership assertions stay exact.
		assert_eq!(census.ecs.live_entities, world.entities().count_spawned() as usize);
		assert!(census.ecs.table_entity_capacity >= 2);
		let mesh_component = census
			.ecs
			.largest_components
			.iter()
			.find(|component| component.name.ends_with("::Mesh3d"))
			.ok_or_else(|| anyhow::anyhow!("Mesh3d component census missing"))?;
		assert_eq!(mesh_component.instances, 2);
		assert_eq!(mesh_component.logical_inline_bytes, 2 * size_of::<Mesh3d>());
		Ok(())
	}

	#[test]
	fn images_distinguish_absent_cpu_data_and_reserved_bytes() -> anyhow::Result<()> {
		let mut world = World::new();
		assert!(MemoryCensus::capture(&world).images.is_none());
		let mut images = Assets::<Image>::default();
		let mut data = Vec::with_capacity(64);
		data.extend([1_u8, 2, 3, 4]);
		let capacity = data.capacity();
		let cpu = images.add(Image { data: Some(data), ..default() });
		let gpu = images.add(Image { data: None, ..default() });
		world.insert_resource(images);
		let census = MemoryCensus::capture(&world);
		let images = census.images.ok_or_else(|| anyhow::anyhow!("missing image census"))?;
		assert_eq!(images.assets, 2);
		assert_eq!(images.assets_with_cpu_data, 1);
		assert_eq!(images.data_payload.logical_bytes, 4);
		assert_eq!(images.data_payload.capacity_bytes, capacity);
		drop((cpu, gpu));
		Ok(())
	}
}
