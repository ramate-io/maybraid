//! GPU/CPU grove metadata shared by woody fields (tree and bush).

use bevy::math::Vec3;

/// std430 grove slot for woody kinds. Upload once per admit; clear flags on retire.
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

/// WGSL `array<WoodyGroveRecord>` stride (std430 rounds 56 B struct up to 16 B alignment).
pub const WOODY_GROVE_RECORD_STRIDE: u64 = 64;

impl WoodyGroveRecord {
	pub const FLAG_ACTIVE: u32 = 1 << 0;

	pub fn is_active(self) -> bool {
		self.flags & Self::FLAG_ACTIVE != 0
	}

	pub fn center(self) -> Vec3 {
		(self.aabb_min + self.aabb_max) * 0.5
	}

	pub fn extent_radius(self) -> f32 {
		(self.aabb_max - self.aabb_min).length() * 0.5
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

/// Visible-span output from pass-1 grove culling (one entry per visible grove).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GroveVisibleSpan {
	pub branch_off: u32,
	pub branch_count: u32,
	pub leaf_off: u32,
	pub leaf_count: u32,
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
	}
}

fn pack_vec3(out: &mut [u8], offset: usize, v: Vec3) {
	out[offset..offset + 4].copy_from_slice(&v.x.to_le_bytes());
	out[offset + 4..offset + 8].copy_from_slice(&v.y.to_le_bytes());
	out[offset + 8..offset + 12].copy_from_slice(&v.z.to_le_bytes());
}

impl GroveVisibleSpan {
	pub const STRIDE: u64 = 16;

	pub fn to_std430_bytes(self) -> [u8; 16] {
		let mut out = [0u8; 16];
		out[0..4].copy_from_slice(&self.branch_off.to_le_bytes());
		out[4..8].copy_from_slice(&self.branch_count.to_le_bytes());
		out[8..12].copy_from_slice(&self.leaf_off.to_le_bytes());
		out[12..16].copy_from_slice(&self.leaf_count.to_le_bytes());
		out
	}
}