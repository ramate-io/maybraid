//! Incremental storage-buffer growth and span uploads for grove catalogs.

use bevy::render::render_resource::{Buffer, BufferDescriptor, BufferUsages};
use bevy::render::renderer::{RenderDevice, RenderQueue};

pub const GROVE_COUNTER_VISIBLE_SPANS: u32 = 9;
pub const GROVE_COUNTER_VISIBLE_BRANCHES: u32 = 10;
pub const GROVE_COUNTER_VISIBLE_LEAVES: u32 = 11;
pub const GROVE_COMPUTE_INDIRECT_STRIDE: u64 = 12;

/// Grow a storage buffer geometrically; copy old contents when reallocating.
pub fn grow_storage_buffer(
	device: &RenderDevice,
	queue: &RenderQueue,
	label: &'static str,
	current: &mut Option<Buffer>,
	min_bytes: u64,
	stride: u64,
) -> bool {
	let need = min_bytes.max(stride);
	let Some(existing) = current.as_ref() else {
		*current = Some(create_storage(device, label, need));
		return true;
	};
	if existing.size() >= need {
		return false;
	}
	let mut new_cap = existing.size().max(stride);
	while new_cap < need {
		new_cap = new_cap.saturating_mul(2);
	}
	let new_buf = create_storage(device, label, new_cap);
	let mut encoder = device.create_command_encoder(&bevy::render::render_resource::CommandEncoderDescriptor {
		label: Some("grove_buffer_grow"),
	});
	encoder.copy_buffer_to_buffer(existing, 0, &new_buf, 0, existing.size());
	queue.submit(Some(encoder.finish()));
	*current = Some(new_buf);
	true
}

fn create_storage(device: &RenderDevice, label: &'static str, bytes: u64) -> Buffer {
	device.create_buffer(&BufferDescriptor {
		label: Some(label),
		size: bytes,
		usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
		mapped_at_creation: false,
	})
}

pub fn ensure_compute_indirect(device: &RenderDevice, slot: &mut Option<Buffer>) -> Buffer {
	if let Some(buf) = slot {
		return buf.clone();
	}
	let buf = device.create_buffer(&BufferDescriptor {
		label: Some("grove_compute_indirect"),
		size: GROVE_COMPUTE_INDIRECT_STRIDE * 2,
		usage: BufferUsages::STORAGE | BufferUsages::INDIRECT | BufferUsages::COPY_DST,
		mapped_at_creation: false,
	});
	let _ = slot.insert(buf.clone());
	buf
}