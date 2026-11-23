//! Per-view retained grove submission, analogue of Bevy `BinnedInputBuffer` dirty uploads.

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

#[cfg(test)]
mod tests {
	use super::*;
	use crate::catalog::GroveId;
	use crate::grove_record::GroveVisibleSpan;

	fn grove(id: u64, lod: u32) -> VisibleGrove {
		VisibleGrove {
			id: GroveId(id),
			lod,
			span: GroveVisibleSpan { branch_off: 0, branch_count: 4, leaf_off: 0, leaf_count: 8 },
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
}
