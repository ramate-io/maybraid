//! Immutable selected-band payloads; one upload per newly loaded batch.
use crate::persistent::payload::PackedInstance;
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::encase::StorageBuffer;
use bevy::render::render_resource::{Buffer, BufferInitDescriptor, BufferUsages, ShaderType};
use bevy::render::renderer::RenderDevice;
use bevy::render::sync_component::SyncComponent;
use bevy::render::sync_world::{MainEntity, MainEntityHashMap};
use bevy::render::RenderApp;
#[cfg(feature = "persistent-vegetation-diagnostics")]
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
#[derive(Clone, Copy, ShaderType)]
struct GpuInstance {
	model: Mat4,
	normal0: Vec4,
	normal1: Vec4,
	normal2: Vec4,
	root0: Vec4,
	root1: Vec4,
	root2: Vec4,
	plant_origin: Vec4,
}

impl GpuInstance {
	fn from_instance(instance: &PackedInstance) -> Option<Self> {
		let model = instance.world_from_local;
		let linear = Mat3::from_mat4(model);
		if !model.is_finite() || linear.determinant() < 1e-12 {
			return None;
		}
		let normal = linear.inverse().transpose();
		Some(Self {
			model,
			normal0: normal.x_axis.extend(instance.part_scale),
			normal1: normal.y_axis.extend(if instance.merged_leaf { 1.0 } else { 0.0 }),
			normal2: normal.z_axis.extend(0.0),
			root0: instance.collection_root.x_axis,
			root1: instance.collection_root.y_axis,
			root2: instance.collection_root.z_axis,
			plant_origin: instance.plant_origin.extend(1.0),
		})
	}
}

#[derive(Component, Clone)]
pub struct PackedBatchInstance {
	pub mesh: AssetId<Mesh>,
	pub bytes: Arc<[u8]>,
	pub instances: u32,
}
impl PackedBatchInstance {
	pub fn encode(instances: &[PackedInstance]) -> Option<Arc<[u8]>> {
		let values =
			instances.iter().map(GpuInstance::from_instance).collect::<Option<Vec<_>>>()?;
		let mut buffer = StorageBuffer::new(Vec::new());
		buffer.write(&values).ok()?;
		Some(buffer.into_inner().into())
	}
}
impl SyncComponent<RenderApp> for PackedBatchInstance {
	type Target = (Self, GpuPackedInstances);
}
impl ExtractComponent<RenderApp> for PackedBatchInstance {
	type QueryData = &'static Self;
	type QueryFilter = ();
	type Out = Self;
	fn extract_component(item: QueryItem<'_, '_, Self::QueryData>) -> Option<Self> {
		Some(item.clone())
	}
}
#[derive(Component)]
pub struct GpuPackedInstances {
	pub buffer: Buffer,
	pub source: Arc<[u8]>,
}
/// Opaque phases may use PLACEHOLDER render entities, so resolve canonical main identity.
#[derive(Resource, Default)]
pub struct PackedDrawEntities(pub MainEntityHashMap<Entity>);
impl PackedDrawEntities {
	pub fn get(&self, main: MainEntity) -> Option<Entity> {
		self.0.get(&main).copied()
	}
}
#[cfg(feature = "persistent-vegetation-diagnostics")]
#[derive(Default)]
pub struct CounterValues {
	pub mesh_misses: AtomicU64,
	pub upload_bytes: AtomicU64,
	pub upload_calls: AtomicU64,
	pub issued_main_draws: AtomicU64,
	pub issued_prepass_draws: AtomicU64,
	pub issued_shadow_draws: AtomicU64,
	pub issued_main_instances: AtomicU64,
}
#[cfg(feature = "persistent-vegetation-diagnostics")]
#[derive(Resource, Clone, Default)]
pub struct PersistentCounters(pub Arc<CounterValues>);
pub fn prepare_packed_gpu(
	mut commands: Commands,
	device: Res<RenderDevice>,
	instances: Query<(Entity, &MainEntity, &PackedBatchInstance, Option<&GpuPackedInstances>)>,
	mut draw_entities: ResMut<PackedDrawEntities>,
	#[cfg(feature = "persistent-vegetation-diagnostics")] counters: Res<PersistentCounters>,
) {
	draw_entities.0.clear();
	for (entity, main, instance, gpu) in &instances {
		draw_entities.0.insert(*main, entity);
		if gpu.is_some_and(|gpu| Arc::ptr_eq(&gpu.source, &instance.bytes)) {
			continue;
		}
		let buffer = device.create_buffer_with_data(&BufferInitDescriptor {
			label: Some("persistent vegetation selected band"),
			contents: &instance.bytes,
			usage: BufferUsages::VERTEX,
		});
		#[cfg(feature = "persistent-vegetation-diagnostics")]
		counters
			.0
			.upload_bytes
			.fetch_add(instance.bytes.len() as u64, Ordering::Relaxed);
		#[cfg(feature = "persistent-vegetation-diagnostics")]
		counters.0.upload_calls.fetch_add(1, Ordering::Relaxed);
		commands
			.entity(entity)
			.insert(GpuPackedInstances { buffer, source: Arc::clone(&instance.bytes) });
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn packed_leaf_adapter_matches_rotated_nonuniform_merged_reference() -> anyhow::Result<()> {
		let root = Mat4::from_scale_rotation_translation(
			Vec3::new(2.0, 3.0, 1.5),
			Quat::from_rotation_y(0.7),
			Vec3::new(11.0, 8.0, -4.0),
		);
		let child = Mat4::from_scale_rotation_translation(
			Vec3::new(0.8, 1.4, 0.5),
			Quat::from_rotation_z(0.4),
			Vec3::new(2.0, 5.0, 1.0),
		);
		let part_scale = 1.4;
		let instance = PackedInstance {
			world_from_local: root * child,
			plant_origin: root.w_axis.truncate(),
			collection_root: root,
			part_scale,
			merged_leaf: true,
		};
		let gpu = GpuInstance::from_instance(&instance)
			.ok_or_else(|| anyhow::anyhow!("valid resident instance rejected"))?;
		let raw = Vec3::new(0.3, -0.6, 0.2);
		let baked = child.transform_point3(raw);
		let expected_centroid = root.transform_point3(baked - raw * part_scale);
		let root_linear =
			Mat3::from_cols(gpu.root0.truncate(), gpu.root1.truncate(), gpu.root2.truncate());
		let actual_centroid = gpu.model.transform_point3(raw) - root_linear * (raw * gpu.normal0.w);
		assert!((expected_centroid - actual_centroid).length() < 1e-4);
		assert_eq!(gpu.plant_origin.truncate(), root.w_axis.truncate());
		assert!(gpu.normal1.w > 0.5);
		assert!((actual_centroid - gpu.model.w_axis.truncate()).length() > 0.01);
		let tangent = Vec3::X;
		let normal = Vec3::Y;
		let transformed_tangent = Mat3::from_mat4(root * child) * tangent;
		let transformed_normal =
			Mat3::from_cols(gpu.normal0.truncate(), gpu.normal1.truncate(), gpu.normal2.truncate())
				* normal;
		assert!(transformed_normal.dot(transformed_tangent).abs() < 1e-4);
		let encoded = PackedBatchInstance::encode(&[instance])
			.ok_or_else(|| anyhow::anyhow!("encoding failed"))?;
		assert_eq!(encoded.len(), 176);
		Ok(())
	}
}
