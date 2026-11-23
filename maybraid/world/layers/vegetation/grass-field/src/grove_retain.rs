//! Per-view retained grove submission, analogue of Bevy `BinnedInputBuffer` dirty uploads.

use std::collections::HashMap;

use crate::catalog::{GroveEncodeStats, GroveId};
use crate::grove_record::VisibleGrove;

/// `MAYBRAID_GROVE_RETAIN=force-rebuild` disables skipping select/span uploads.
pub const GROVE_RETAIN_ENV: &str = "MAYBRAID_GROVE_RETAIN";
pub const GROVE_RETAIN_FORCE_REBUILD: &str = "force-rebuild";

pub fn grove_retain_force_rebuild() -> bool {
	std::env::var(GROVE_RETAIN_ENV).ok().as_deref() == Some(GROVE_RETAIN_FORCE_REBUILD)
}

/// Last visible (grove_id, lod, span) set plus the view/policy fingerprint.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RetainedGroveFrame {
	pub generation: u64,
	pub viewport_w: u32,
	pub viewport_h: u32,
	pub fov_bits: u32,
	pub card_m_bits: u32,
	pub groves: Vec<VisibleGrove>,
}

impl RetainedGroveFrame {
	pub fn capture(
		generation: u64,
		viewport_w: u32,
		viewport_h: u32,
		fov_y: f32,
		card_m: f32,
		groves: Vec<VisibleGrove>,
	) -> Self {
		Self {
			generation,
			viewport_w,
			viewport_h,
			fov_bits: fov_y.to_bits(),
			card_m_bits: card_m.to_bits(),
			groves,
		}
	}

	pub fn is_clean(&self, next: &Self) -> bool {
		!grove_retain_force_rebuild() && self == next
	}

	pub fn work_counts(&self) -> (u32, u32) {
		let mut branches = 0u32;
		let mut leaves = 0u32;
		for grove in &self.groves {
			branches += grove.span.branch_count;
			leaves += grove.span.leaf_count;
		}
		(branches, leaves)
	}
}

/// One view's retained submit, shared by tree/bush/grass prepare.
#[derive(Clone, Debug, PartialEq)]
pub struct GroveSubmit {
	pub frame: RetainedGroveFrame,
	pub dirty: bool,
	pub work_branches: u32,
	pub work_leaves: u32,
	pub span_count: u32,
}

impl GroveSubmit {
	/// Grove residency can skip select when the visible set is unchanged.
	/// The non-grove path is always dirty: GPU select still uses the live camera.
	pub fn plan(
		previous: &RetainedGroveFrame,
		generation: u64,
		viewport_w: u32,
		viewport_h: u32,
		fov_y: f32,
		card_m: f32,
		groves: Vec<VisibleGrove>,
		grove_residency: bool,
		fallback_branches: u32,
		fallback_leaves: u32,
	) -> Self {
		let frame =
			RetainedGroveFrame::capture(generation, viewport_w, viewport_h, fov_y, card_m, groves);
		if !grove_residency {
			return Self {
				frame,
				dirty: true,
				work_branches: fallback_branches,
				work_leaves: fallback_leaves,
				span_count: 0,
			};
		}
		let dirty = !previous.is_clean(&frame);
		let (work_branches, work_leaves) = frame.work_counts();
		let span_count = frame.groves.len() as u32;
		Self { frame, dirty, work_branches, work_leaves, span_count }
	}

	/// Apply this plan: write uniforms always; upload spans and replace the
	/// retained frame only when dirty. `upload` returns whether the span buffer grew.
	pub fn stamp(
		self,
		retained: &mut RetainedGroveFrame,
		encode: &mut GroveEncodeStats,
		mut upload: impl FnMut(&[VisibleGrove]) -> bool,
	) -> StampedView {
		encode.uniform_writes += 1;
		let mut span_grew = false;
		if self.dirty {
			if self.span_count > 0 {
				span_grew = upload(&self.frame.groves);
				encode.span_uploads += 1;
			}
			*retained = self.frame;
		} else {
			encode.retained_clean += 1;
		}
		StampedView {
			dirty: self.dirty,
			work_branches: self.work_branches,
			work_leaves: self.work_leaves,
			span_count: self.span_count,
			span_grew,
		}
	}

	/// Compute-pass counters. Clean frames issue no select/finalize dispatches.
	pub fn record_select(
		encode: &mut GroveEncodeStats,
		dirty: bool,
		woody: bool,
		work_branches: u32,
		work_leaves: u32,
	) {
		if !dirty {
			return;
		}
		encode.passes += 1;
		encode.dispatches += if woody {
			1 + u32::from(work_branches > 0) + u32::from(work_leaves > 0)
		} else {
			1 + u32::from(work_branches > 0)
		};
	}

	/// Encode counters matching tree/bush/grass compute + queue on this view.
	pub fn encode_stats(&self, woody: bool) -> GroveEncodeStats {
		let mut stats = GroveEncodeStats { uniform_writes: 1, ..GroveEncodeStats::default() };
		stats.draws = if woody { 6 } else { 3 };
		if !self.dirty {
			stats.retained_clean = 1;
			return stats;
		}
		if self.span_count > 0 {
			stats.span_uploads = 1;
		}
		stats.passes = 1;
		if woody {
			stats.dispatches =
				1 + u32::from(self.work_branches > 0) + u32::from(self.work_leaves > 0);
		} else {
			stats.dispatches = 1 + u32::from(self.work_branches > 0);
		}
		stats
	}
}

/// Result of [`GroveSubmit::stamp`], shared by tree/bush/grass prepare.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StampedView {
	pub dirty: bool,
	pub work_branches: u32,
	pub work_leaves: u32,
	pub span_count: u32,
	pub span_grew: bool,
}

/// Record the last culled band per grove for hysteresis on the next frame.
pub fn remember_bands(last: &mut HashMap<GroveId, u32>, groves: &[VisibleGrove]) {
	last.clear();
	for grove in groves {
		last.insert(grove.id, grove.lod);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove_record::GroveVisibleSpan;

	fn grove(id: u64, lod: u32) -> VisibleGrove {
		VisibleGrove {
			id: GroveId(id),
			lod,
			span: GroveVisibleSpan {
				branch_off: 0,
				branch_count: 4,
				leaf_off: 0,
				leaf_count: 8,
				lod,
			},
		}
	}

	#[test]
	fn still_frame_is_clean() {
		let frame = RetainedGroveFrame {
			generation: 3,
			viewport_w: 1280,
			viewport_h: 720,
			fov_bits: 1.0f32.to_bits(),
			card_m_bits: 1000.0f32.to_bits(),
			groves: vec![grove(1, 0)],
		};
		assert!(frame.is_clean(&frame));
	}

	#[test]
	fn admit_or_frustum_entry_dirties() {
		let mut a = RetainedGroveFrame {
			generation: 1,
			viewport_w: 1280,
			viewport_h: 720,
			fov_bits: 1,
			card_m_bits: 2,
			groves: vec![grove(1, 0)],
		};
		let mut b = a.clone();
		b.generation = 2;
		assert!(!a.is_clean(&b));
		b = a.clone();
		b.groves.push(grove(2, 1));
		assert!(!a.is_clean(&b));
		a.groves[0].lod = 2;
		assert!(!a.is_clean(&b));
	}

	#[test]
	fn viewport_or_fov_dirties() {
		let a = RetainedGroveFrame {
			generation: 1,
			viewport_w: 1280,
			viewport_h: 720,
			fov_bits: 1,
			card_m_bits: 2,
			groves: vec![grove(1, 0)],
		};
		let mut b = a.clone();
		b.viewport_w = 1920;
		assert!(!a.is_clean(&b));
		b = a.clone();
		b.fov_bits = 99;
		assert!(!a.is_clean(&b));
		b = a.clone();
		b.card_m_bits = 3;
		assert!(!a.is_clean(&b));
	}

	#[test]
	fn non_grove_path_is_always_dirty() {
		let previous = RetainedGroveFrame::default();
		let first =
			GroveSubmit::plan(&previous, 1, 1280, 720, 1.0, 220.0, Vec::new(), false, 8, 16);
		let second =
			GroveSubmit::plan(&first.frame, 1, 1280, 720, 1.0, 220.0, Vec::new(), false, 8, 16);
		assert!(first.dirty);
		assert!(second.dirty);
		assert_eq!(first.span_count, 0);
		assert_eq!(second.encode_stats(true).dispatches, 3);
	}

	#[test]
	fn still_camera_second_frame_skips_select_and_span_uploads() {
		let groves = vec![grove(1, 0)];
		let previous = RetainedGroveFrame::default();
		let frame1 =
			GroveSubmit::plan(&previous, 4, 1280, 720, 1.0, 1000.0, groves.clone(), true, 0, 0);
		let stats1 = frame1.encode_stats(true);
		assert!(frame1.dirty);
		assert_eq!(stats1.span_uploads, 1);
		assert!(stats1.dispatches > 0);
		assert_eq!(stats1.draws, 6);

		let frame2 =
			GroveSubmit::plan(&frame1.frame, 4, 1280, 720, 1.0, 1000.0, groves, true, 0, 0);
		let stats2 = frame2.encode_stats(true);
		assert!(!frame2.dirty);
		assert_eq!(stats2.dispatches, 0);
		assert_eq!(stats2.span_uploads, 0);
		assert_eq!(stats2.draws, 6);
		assert_eq!(stats2.retained_clean, 1);
		assert_eq!(stats2.uniform_writes, 1);
	}

	#[test]
	fn camera_move_that_changes_visible_set_redirties() {
		let previous = RetainedGroveFrame::default();
		let visible =
			GroveSubmit::plan(&previous, 1, 1280, 720, 1.0, 1000.0, vec![grove(1, 0)], true, 0, 0);
		let empty =
			GroveSubmit::plan(&visible.frame, 1, 1280, 720, 1.0, 1000.0, Vec::new(), true, 0, 0);
		assert!(empty.dirty);
		assert_eq!(empty.encode_stats(true).dispatches, 1);
	}
}
