//! CPU grove metadata shared by woody fields (tree and bush).

use bevy::math::Vec3;

use crate::catalog::GroveId;

/// std430 grove slot for woody kinds. CPU cull reads this; GPU select never does.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WoodyGroveRecord {
	pub aabb_min: Vec3,
	pub branch_off: u32,
	pub aabb_max: Vec3,
	pub branch_count: u32,
	pub plant_off: u32,
	pub plant_count: u32,
	pub leaf_off: u32,
	pub leaf_count: u32,
	pub seed: u32,
	pub flags: u32,
}

/// WGSL `array<WoodyGroveRecord>` stride (std430 rounds a 56 B struct up to 16 B).
pub const WOODY_GROVE_RECORD_STRIDE: u64 = 64;

impl WoodyGroveRecord {
	pub const FLAG_ACTIVE: u32 = 1 << 0;

	pub fn is_active(self) -> bool {
		self.flags & Self::FLAG_ACTIVE != 0
	}

	pub fn center(self) -> Vec3 {
		(self.aabb_min + self.aabb_max) * 0.5
	}

	pub fn to_std430_bytes(self) -> [u8; WOODY_GROVE_RECORD_STRIDE as usize] {
		let mut out = [0u8; WOODY_GROVE_RECORD_STRIDE as usize];
		// Bytes 56..64 are array-element tail padding (must stay zero).
		pack_vec3(&mut out, 0, self.aabb_min);
		out[12..16].copy_from_slice(&self.branch_off.to_le_bytes());
		pack_vec3(&mut out, 16, self.aabb_max);
		out[28..32].copy_from_slice(&self.branch_count.to_le_bytes());
		out[32..36].copy_from_slice(&self.plant_off.to_le_bytes());
		out[36..40].copy_from_slice(&self.plant_count.to_le_bytes());
		out[40..44].copy_from_slice(&self.leaf_off.to_le_bytes());
		out[44..48].copy_from_slice(&self.leaf_count.to_le_bytes());
		out[48..52].copy_from_slice(&self.seed.to_le_bytes());
		out[52..56].copy_from_slice(&self.flags.to_le_bytes());
		out
	}
}

/// Visible-span output from CPU grove culling (one entry per visible grove).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroveVisibleSpan {
	pub branch_off: u32,
	pub branch_count: u32,
	pub leaf_off: u32,
	pub leaf_count: u32,
	pub lod: u32,
}

impl GroveVisibleSpan {
	/// std430 array stride: five u32s plus 12 B pad to a 16 B multiple.
	pub const STRIDE: u64 = 32;

	pub fn is_empty(self) -> bool {
		self.branch_count == 0 && self.leaf_count == 0
	}

	pub fn branch_range(self) -> std::ops::Range<u32> {
		self.branch_off..self.branch_off + self.branch_count
	}

	pub fn leaf_range(self) -> std::ops::Range<u32> {
		self.leaf_off..self.leaf_off + self.leaf_count
	}

	pub fn to_std430_bytes(self) -> [u8; 32] {
		let mut out = [0u8; 32];
		out[0..4].copy_from_slice(&self.branch_off.to_le_bytes());
		out[4..8].copy_from_slice(&self.branch_count.to_le_bytes());
		out[8..12].copy_from_slice(&self.leaf_off.to_le_bytes());
		out[12..16].copy_from_slice(&self.leaf_count.to_le_bytes());
		out[16..20].copy_from_slice(&self.lod.to_le_bytes());
		out
	}
}

/// One visible grove after CPU frustum + distance + band pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisibleGrove {
	pub id: GroveId,
	pub lod: u32,
	pub span: GroveVisibleSpan,
}

fn pack_vec3(out: &mut [u8], offset: usize, v: Vec3) {
	out[offset..offset + 4].copy_from_slice(&v.x.to_le_bytes());
	out[offset + 4..offset + 8].copy_from_slice(&v.y.to_le_bytes());
	out[offset + 8..offset + 12].copy_from_slice(&v.z.to_le_bytes());
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn storage_stride_matches_wgsl_array_element_size() {
		let bytes = WoodyGroveRecord::default().to_std430_bytes();
		assert_eq!(bytes.len(), WOODY_GROVE_RECORD_STRIDE as usize);
		assert_eq!(WOODY_GROVE_RECORD_STRIDE, 64);
		assert!(bytes[56..].iter().all(|&b| b == 0));
		let span =
			GroveVisibleSpan { branch_off: 1, branch_count: 2, leaf_off: 3, leaf_count: 4, lod: 2 }
				.to_std430_bytes();
		assert_eq!(span.len(), GroveVisibleSpan::STRIDE as usize);
		assert_eq!(u32::from_le_bytes(span[16..20].try_into().unwrap()), 2);
		assert!(span[20..].iter().all(|&b| b == 0));
	}
}
