//! Geometric GPU arena growth. Copies old contents instead of rebuilding.

use bevy::render::render_resource::{
	Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor,
};
use bevy::render::renderer::{RenderDevice, RenderQueue};

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

/// Grow `current` geometrically to hold `needed` bytes. Copies prior contents
/// GPU-side and never rebuilds from a CPU dump of the whole arena.
pub fn grow_storage_buffer(
	device: &RenderDevice,
	queue: &RenderQueue,
	current: &mut Option<Buffer>,
	needed: u64,
	min_stride: u64,
	label: &'static str,
) -> bool {
	let mut plan =
		ArenaBufferPlan { cap: current.as_ref().map(|buffer| buffer.size()).unwrap_or(0) };
	match plan.ensure(needed, min_stride) {
		ArenaGrow::Fit => false,
		ArenaGrow::Grew { new_cap, .. } => {
			let new_buf = device.create_buffer(&BufferDescriptor {
				label: Some(label),
				size: new_cap,
				usage: BufferUsages::STORAGE | BufferUsages::COPY_DST | BufferUsages::COPY_SRC,
				mapped_at_creation: false,
			});
			if let Some(old) = current.take() {
				let copy = old.size().min(new_cap);
				if copy > 0 {
					let mut encoder = device.create_command_encoder(&CommandEncoderDescriptor {
						label: Some("vegetation_arena_grow"),
					});
					encoder.copy_buffer_to_buffer(&old, 0, &new_buf, 0, copy);
					queue.submit([encoder.finish()]);
				}
			}
			*current = Some(new_buf);
			true
		}
	}
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
}
