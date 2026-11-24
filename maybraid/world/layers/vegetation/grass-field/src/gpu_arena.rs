//! Geometric GPU arena growth. Copies old contents instead of rebuilding.

use bevy::render::render_resource::{
	Buffer, BufferDescriptor, BufferInitDescriptor, BufferUsages, CommandEncoder,
};
use bevy::render::renderer::{RenderDevice, RenderQueue};

use crate::grove_record::{GroveVisibleSpan, VisibleGrove};

/// CPU-side capacity plan matching GPU buffer growth.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ArenaBufferPlan {
	pub cap: u64,
}

/// Result of [`ArenaBufferPlan::ensure`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArenaGrow {
	Fit,
	Grew { previous_cap: u64, new_cap: u64 },
}

impl ArenaBufferPlan {
	pub fn ensure(&mut self, needed: u64, min_stride: u64) -> ArenaGrow {
		let needed = needed.max(min_stride);
		if self.cap >= needed {
			return ArenaGrow::Fit;
		}
		let previous_cap = self.cap;
		let mut cap = self.cap.max(min_stride);
		if cap == 0 {
			cap = min_stride;
		}
		while cap < needed {
			cap = cap.saturating_mul(2).max(needed);
		}
		self.cap = cap;
		ArenaGrow::Grew { previous_cap, new_cap: cap }
	}
}

/// Grow copy recorded on the frame encoder instead of a private `queue.submit`.
#[derive(Clone, Debug)]
pub struct PendingBufferCopy {
	src: Buffer,
	dst: Buffer,
	size: u64,
}

impl PendingBufferCopy {
	pub fn encode(&self, encoder: &mut CommandEncoder) {
		if self.size > 0 {
			encoder.copy_buffer_to_buffer(&self.src, 0, &self.dst, 0, self.size);
		}
	}
}

/// Staging write applied after grow copies so free-list reuse cannot be overwritten.
#[derive(Clone, Debug)]
pub struct PendingBufferWrite {
	src: Buffer,
	dst: Buffer,
	offset: u64,
	size: u64,
}

impl PendingBufferWrite {
	pub fn encode(&self, encoder: &mut CommandEncoder) {
		if self.size > 0 {
			encoder.copy_buffer_to_buffer(&self.src, 0, &self.dst, self.offset, self.size);
		}
	}
}

/// Encode grow copies then span writes onto the frame's command encoder.
pub fn encode_arena_copies(
	encoder: &mut CommandEncoder,
	copies: &mut Vec<PendingBufferCopy>,
	writes: &mut Vec<PendingBufferWrite>,
) {
	for copy in copies.drain(..) {
		copy.encode(encoder);
	}
	for write in writes.drain(..) {
		write.encode(encoder);
	}
}

fn storage_buffer(device: &RenderDevice, size: u64, label: &'static str) -> Buffer {
	device.create_buffer(&BufferDescriptor {
		label: Some(label),
		size,
		usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
		mapped_at_creation: false,
	})
}

/// Grow `current` geometrically. The copy is recorded in `pending` for the frame encoder.
pub fn grow_storage_buffer(
	device: &RenderDevice,
	current: &mut Option<Buffer>,
	needed: u64,
	min_stride: u64,
	label: &'static str,
	pending: &mut Vec<PendingBufferCopy>,
) -> bool {
	let mut plan =
		ArenaBufferPlan { cap: current.as_ref().map(|buffer| buffer.size()).unwrap_or(0) };
	match plan.ensure(needed, min_stride) {
		ArenaGrow::Fit => false,
		ArenaGrow::Grew { new_cap, .. } => {
			let new_buf = storage_buffer(device, new_cap, label);
			if let Some(old) = current.take() {
				let copy = old.size().min(new_cap);
				if copy > 0 {
					pending.push(PendingBufferCopy { src: old, dst: new_buf.clone(), size: copy });
				}
			}
			*current = Some(new_buf);
			true
		}
	}
}

/// First per-view instance-list length. Grows geometrically with visible work.
pub const INITIAL_LIST_ITEMS: u32 = 64;

/// Grow every instance-list buffer to `needed_items` without recreating other views.
pub fn grow_list_buffers<'a>(
	device: &RenderDevice,
	lists: impl IntoIterator<Item = &'a mut Buffer>,
	needed_items: u32,
	label: &'static str,
	pending: &mut Vec<PendingBufferCopy>,
) -> bool {
	let needed = u64::from(needed_items.max(1)) * 4;
	let mut grew = false;
	for list in lists {
		grew |= grow_buffer_in_place(device, list, needed, 4, label, pending);
	}
	grew
}

/// Grow an existing storage buffer geometrically, copying prior contents in-place.
pub fn grow_buffer_in_place(
	device: &RenderDevice,
	current: &mut Buffer,
	needed: u64,
	min_stride: u64,
	label: &'static str,
	pending: &mut Vec<PendingBufferCopy>,
) -> bool {
	let mut plan = ArenaBufferPlan { cap: current.size() };
	match plan.ensure(needed, min_stride) {
		ArenaGrow::Fit => false,
		ArenaGrow::Grew { new_cap, .. } => {
			let new_buf = storage_buffer(device, new_cap, label);
			let copy = current.size().min(new_cap);
			if copy > 0 {
				pending.push(PendingBufferCopy {
					src: current.clone(),
					dst: new_buf.clone(),
					size: copy,
				});
			}
			*current = new_buf;
			true
		}
	}
}

/// Stage a span write that runs after any grow copies on the frame encoder.
pub fn stage_buffer_write(
	device: &RenderDevice,
	dst: &Buffer,
	offset: u64,
	bytes: &[u8],
	pending: &mut Vec<PendingBufferWrite>,
) {
	if bytes.is_empty() {
		return;
	}
	let src = device.create_buffer_with_data(&BufferInitDescriptor {
		label: Some("vegetation_arena_write"),
		contents: bytes,
		usage: BufferUsages::COPY_SRC,
	});
	pending.push(PendingBufferWrite { src, dst: dst.clone(), offset, size: bytes.len() as u64 });
}

/// Grow without copying when the caller is about to rewrite the whole buffer.
pub fn resize_storage_buffer(
	device: &RenderDevice,
	current: &mut Buffer,
	needed: u64,
	min_stride: u64,
	label: &'static str,
) -> bool {
	let mut plan = ArenaBufferPlan { cap: current.size() };
	match plan.ensure(needed, min_stride) {
		ArenaGrow::Fit => false,
		ArenaGrow::Grew { new_cap, .. } => {
			*current = storage_buffer(device, new_cap, label);
			true
		}
	}
}

/// Rewrite the visible-span list. Grows this view's buffer only.
pub fn upload_visible_spans(
	device: &RenderDevice,
	queue: &RenderQueue,
	current: &mut Buffer,
	groves: &[VisibleGrove],
	label: &'static str,
) -> bool {
	if groves.is_empty() {
		return false;
	}
	let bytes: Vec<u8> = groves.iter().flat_map(|grove| grove.span.to_std430_bytes()).collect();
	let grew = resize_storage_buffer(
		device,
		current,
		(bytes.len() as u64).max(GroveVisibleSpan::STRIDE),
		GroveVisibleSpan::STRIDE,
		label,
	);
	queue.write_buffer(current, 0, &bytes);
	grew
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn admit_after_growth_keeps_prior_capacity_contents() {
		let mut plan = ArenaBufferPlan { cap: 64 };
		assert_eq!(plan.ensure(64, 16), ArenaGrow::Fit);
		let grew = plan.ensure(80, 16);
		assert_eq!(grew, ArenaGrow::Grew { previous_cap: 64, new_cap: 128 });
		assert_eq!(plan.ensure(100, 16), ArenaGrow::Fit);
	}

	#[test]
	fn empty_plan_starts_at_min_stride() {
		let mut plan = ArenaBufferPlan::default();
		assert_eq!(plan.ensure(16, 16), ArenaGrow::Grew { previous_cap: 0, new_cap: 16 });
	}

	#[test]
	fn list_capacity_follows_visible_work_not_catalog_len() {
		let mut plan = ArenaBufferPlan { cap: u64::from(INITIAL_LIST_ITEMS) * 4 };
		let visible = 100u64 * 4;
		let catalog = 1_000_000u64 * 4;
		assert_eq!(plan.ensure(visible, 4), ArenaGrow::Grew { previous_cap: 256, new_cap: 512 });
		assert!(plan.cap < catalog);
		assert_eq!(plan.ensure(visible, 4), ArenaGrow::Fit);
	}
}
