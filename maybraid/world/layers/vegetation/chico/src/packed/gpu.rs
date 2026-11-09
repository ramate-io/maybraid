//! Resident vertex-instance payloads, uploaded only when a revision changes.

use crate::packed::cache::{TilePayload, TileSignal, READY_BUFFER};
use crate::packed::instances::PackedInstance;
use bevy::ecs::query::QueryItem;
use bevy::prelude::*;
use bevy::render::extract_component::ExtractComponent;
use bevy::render::render_resource::encase::StorageBuffer;
use bevy::render::render_resource::{Buffer, BufferInitDescriptor, BufferUsages, ShaderType};
use bevy::render::renderer::RenderDevice;
use bevy::render::sync_component::SyncComponent;
use bevy::render::RenderApp;
use lod::LodSceneLevel;
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
	pub cell: Entity,
	pub mesh: AssetId<Mesh>,
	pub payload: Arc<TilePayload>,
	pub signal: Arc<TileSignal>,
	/// Extracted with the ordinary scene visibility; never read live main-world
	/// activation state from a pipelined render frame.
	pub extracted_phase: u8,
	pub ordinal: usize,
	pub level: LodSceneLevel,
	pub bytes: Arc<[u8]>,
	pub instances: u32,
	pub shadow_only: bool,
	/// Optional diagnostic target only: actual main-pass commands, not selected bins.
	#[cfg(feature = "packed-diagnostics")]
	pub diagnostic_main_draws: Arc<AtomicU64>,
}

impl PackedBatchInstance {
	pub fn lod_index(&self) -> usize {
		crate::packed::cache::TilePayload::level_index(self.level)
	}
	pub fn is_active(&self) -> bool {
		self.extracted_phase == crate::packed::cache::ACTIVE
	}

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
		let mut extracted = item.clone();
		extracted.extracted_phase = item.signal.phase.load(Ordering::Acquire);
		Some(extracted)
	}
}

#[derive(Component)]
pub struct GpuPackedInstances {
	pub buffer: Buffer,
	pub source: Arc<[u8]>,
}

#[derive(Default)]
pub struct CounterValues {
	pub submitted_payload_bytes: AtomicU64,
	pub allocated_payload_bytes: AtomicU64,
	pub upload_calls: AtomicU64,
	pub issued_main_draws: AtomicU64,
	pub issued_prepass_draws: AtomicU64,
	pub issued_shadow_draws: AtomicU64,
	pub issued_main_instances: AtomicU64,
}

#[derive(Resource, Clone, Default)]
pub struct PackedCounters(pub Arc<CounterValues>);

pub fn prepare_packed_gpu(
	mut commands: Commands,
	device: Res<RenderDevice>,
	instances: Query<(Entity, &PackedBatchInstance, Option<&GpuPackedInstances>)>,
	counters: Res<PackedCounters>,
) {
	let _span =
		bevy::log::tracing::trace_span!(target: "maybraid.packed.profile", "packed_prepare_gpu")
			.entered();
	for (entity, instance, gpu) in &instances {
		if instance.extracted_phase == crate::packed::cache::RETIRED {
			continue;
		}
		if gpu.is_some_and(|gpu| Arc::ptr_eq(&gpu.source, &instance.bytes)) {
			instance.signal.acknowledge(instance.ordinal, READY_BUFFER);
			continue;
		}
		let buffer = device.create_buffer_with_data(&BufferInitDescriptor {
			label: Some("packed woody resident instances"),
			contents: &instance.bytes,
			usage: BufferUsages::VERTEX,
		});
		let bytes = instance.bytes.len() as u64;
		counters.0.submitted_payload_bytes.fetch_add(bytes, Ordering::Relaxed);
		counters.0.allocated_payload_bytes.fetch_add(bytes, Ordering::Relaxed);
		counters.0.upload_calls.fetch_add(1, Ordering::Relaxed);
		commands
			.entity(entity)
			.insert(GpuPackedInstances { buffer, source: Arc::clone(&instance.bytes) });
		instance.signal.acknowledge(instance.ordinal, READY_BUFFER);
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
