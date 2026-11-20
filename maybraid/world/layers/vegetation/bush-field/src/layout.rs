//! Compact plant / branch / leaf GPU structs. Extra bush fields stay in this crate
//! (shared `layout.wesl` keeps grass `BladeDescriptor`).

use bevy::math::Vec3;
use bevy::render::render_resource::ShaderType;

/// Sentinel parent index: this branch is rooted on the plant, not another branch.
pub const PARENT_NONE: u32 = u32::MAX;

pub const BUSH_LOD_NEAR: u32 = 0;
pub const BUSH_LOD_MID: u32 = 1;
pub const BUSH_LOD_FAR: u32 = 2;
pub const BUSH_LOD_COUNT: usize = 3;

pub const PLANT_DESCRIPTOR_STRIDE: u64 = 96;
pub const BRANCH_DESCRIPTOR_STRIDE: u64 = 80;
pub const LEAF_DESCRIPTOR_STRIDE: u64 = 64;

/// CPU-owned patch slice into the persistent plant buffer.
#[derive(Clone, Copy, Debug, ShaderType, PartialEq)]
pub struct BushPatchHeader {
	pub aabb_min: Vec3,
	pub plant_offset: u32,
	pub aabb_max: Vec3,
	pub plant_count: u32,
	pub seed: u32,
	pub coverage: f32,
	pub species: u32,
	pub flags: u32,
}

impl BushPatchHeader {
	pub const FLAG_ACTIVE: u32 = 1 << 0;

	pub fn is_active(self) -> bool {
		self.flags & Self::FLAG_ACTIVE != 0
	}

	pub fn plant_range(self) -> std::ops::Range<u32> {
		self.plant_offset..self.plant_offset.saturating_add(self.plant_count)
	}
}

/// One woody plant. World metres. std430, 96 bytes.
#[derive(Clone, Copy, Debug, ShaderType, PartialEq)]
pub struct PlantDescriptor {
	pub root: Vec3,
	pub height_m: f32,
	pub up: Vec3,
	pub plant_id: u32,
	pub aabb_min: Vec3,
	pub branch_offset: u32,
	pub aabb_max: Vec3,
	pub branch_count: u32,
	pub leaf_offset: u32,
	pub leaf_count: u32,
	pub seed: u32,
	pub flags: u32,
	pub crown_radius_m: f32,
	pub wood_stiffness: f32,
	pub foliage_density: f32,
	pub species: f32,
}

impl PlantDescriptor {
	pub const FLAG_ACTIVE: u32 = 1 << 0;

	pub fn is_active(self) -> bool {
		self.flags & Self::FLAG_ACTIVE != 0
	}

	pub fn branch_range(self) -> std::ops::Range<u32> {
		self.branch_offset..self.branch_offset.saturating_add(self.branch_count)
	}

	pub fn leaf_range(self) -> std::ops::Range<u32> {
		self.leaf_offset..self.leaf_offset.saturating_add(self.leaf_count)
	}

	pub fn conservative_extent_m(self, wind_strength_m: f32) -> f32 {
		let wind = vegetation_shaders::wesl::max_wind_displacement_m(
			self.height_m,
			wind_strength_m,
			self.wood_stiffness,
		);
		self.height_m.max(self.crown_radius_m) + wind + self.crown_radius_m * 0.35
	}

	pub fn to_std430_bytes(self) -> [u8; PLANT_DESCRIPTOR_STRIDE as usize] {
		let mut out = [0u8; PLANT_DESCRIPTOR_STRIDE as usize];
		pack_vec3(&mut out, 0, self.root);
		pack_f32(&mut out, 12, self.height_m);
		pack_vec3(&mut out, 16, self.up);
		pack_u32(&mut out, 28, self.plant_id);
		pack_vec3(&mut out, 32, self.aabb_min);
		pack_u32(&mut out, 44, self.branch_offset);
		pack_vec3(&mut out, 48, self.aabb_max);
		pack_u32(&mut out, 60, self.branch_count);
		pack_u32(&mut out, 64, self.leaf_offset);
		pack_u32(&mut out, 68, self.leaf_count);
		pack_u32(&mut out, 72, self.seed);
		pack_u32(&mut out, 76, self.flags);
		pack_f32(&mut out, 80, self.crown_radius_m);
		pack_f32(&mut out, 84, self.wood_stiffness);
		pack_f32(&mut out, 88, self.foliage_density);
		pack_f32(&mut out, 92, self.species);
		out
	}
}

/// One connected branch. `p0..=p2` are metres in parent-local space (world for a trunk).
#[derive(Clone, Copy, Debug, ShaderType, PartialEq)]
pub struct BranchDescriptor {
	pub p0: Vec3,
	pub parent_index: u32,
	pub p1: Vec3,
	pub parent_t: f32,
	pub p2: Vec3,
	pub plant_index: u32,
	pub root_radius_m: f32,
	pub tip_radius_m: f32,
	pub stiffness: f32,
	/// `0` = load-bearing (never independently dropped). Higher = twig.
	pub keep_rank: f32,
	pub twist: f32,
	pub depth: u32,
	pub branch_id: u32,
	pub major_parent: u32,
}

impl BranchDescriptor {
	pub fn is_trunk(self) -> bool {
		self.parent_index == PARENT_NONE
	}

	pub fn parent_is_resolvable(self, count: u32) -> bool {
		self.parent_index == PARENT_NONE || self.parent_index < count
	}

	pub fn max_radius_m(self) -> f32 {
		self.root_radius_m.max(self.tip_radius_m)
	}

	pub fn to_std430_bytes(self) -> [u8; BRANCH_DESCRIPTOR_STRIDE as usize] {
		let mut out = [0u8; BRANCH_DESCRIPTOR_STRIDE as usize];
		pack_vec3(&mut out, 0, self.p0);
		pack_u32(&mut out, 12, self.parent_index);
		pack_vec3(&mut out, 16, self.p1);
		pack_f32(&mut out, 28, self.parent_t);
		pack_vec3(&mut out, 32, self.p2);
		pack_u32(&mut out, 44, self.plant_index);
		pack_f32(&mut out, 48, self.root_radius_m);
		pack_f32(&mut out, 52, self.tip_radius_m);
		pack_f32(&mut out, 56, self.stiffness);
		pack_f32(&mut out, 60, self.keep_rank);
		pack_f32(&mut out, 64, self.twist);
		pack_u32(&mut out, 68, self.depth);
		pack_u32(&mut out, 72, self.branch_id);
		pack_u32(&mut out, 76, self.major_parent);
		out
	}
}

/// One leaf attached at `attach_t` on `parent_branch`.
#[derive(Clone, Copy, Debug, ShaderType, PartialEq)]
pub struct LeafDescriptor {
	pub plant_index: u32,
	pub parent_branch: u32,
	pub attach_t: f32,
	pub size_m: f32,
	pub twist: f32,
	pub fold: f32,
	pub flutter: f32,
	pub density_rank: f32,
	pub facing: Vec3,
	pub leaf_id: u32,
	pub palette: Vec3,
	pub shape: f32,
}

impl LeafDescriptor {
	/// Packed `LeafLayer::shell_unit`: `0` inner, `0.5` mid, `1` outer.
	pub fn layer_shell(self) -> u32 {
		(self.shape * 2.0).round().clamp(0.0, 2.0) as u32
	}

	pub fn to_std430_bytes(self) -> [u8; LEAF_DESCRIPTOR_STRIDE as usize] {
		let mut out = [0u8; LEAF_DESCRIPTOR_STRIDE as usize];
		pack_u32(&mut out, 0, self.plant_index);
		pack_u32(&mut out, 4, self.parent_branch);
		pack_f32(&mut out, 8, self.attach_t);
		pack_f32(&mut out, 12, self.size_m);
		pack_f32(&mut out, 16, self.twist);
		pack_f32(&mut out, 20, self.fold);
		pack_f32(&mut out, 24, self.flutter);
		pack_f32(&mut out, 28, self.density_rank);
		pack_vec3(&mut out, 32, self.facing);
		pack_u32(&mut out, 44, self.leaf_id);
		pack_vec3(&mut out, 48, self.palette);
		pack_f32(&mut out, 60, self.shape);
		out
	}
}

pub fn pack_f32(out: &mut [u8], offset: usize, value: f32) {
	out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub fn pack_u32(out: &mut [u8], offset: usize, value: u32) {
	out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

pub fn pack_vec3(out: &mut [u8], offset: usize, value: Vec3) {
	pack_f32(out, offset, value.x);
	pack_f32(out, offset + 4, value.y);
	pack_f32(out, offset + 8, value.z);
}

/// Wood tube LOD. Separate longitudinal rings and radial sides.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WoodLod {
	Near,
	Mid,
	Far,
}

impl WoodLod {
	pub fn from_bucket(bucket: u32) -> Option<Self> {
		match bucket {
			BUSH_LOD_NEAR => Some(Self::Near),
			BUSH_LOD_MID => Some(Self::Mid),
			BUSH_LOD_FAR => Some(Self::Far),
			_ => None,
		}
	}

	pub fn bucket(self) -> u32 {
		match self {
			Self::Near => BUSH_LOD_NEAR,
			Self::Mid => BUSH_LOD_MID,
			Self::Far => BUSH_LOD_FAR,
		}
	}

	/// Longitudinal samples including the tip ring (cap is an extra vertex).
	pub fn rings(self) -> u32 {
		match self {
			Self::Near => 5,
			Self::Mid => 3,
			Self::Far => 2,
		}
	}

	pub fn sides(self) -> u32 {
		match self {
			Self::Near => 6,
			Self::Mid => 4,
			Self::Far => 3,
		}
	}

	/// Max branch depth submitted at this LOD (`0` = trunk only).
	pub fn max_depth(self) -> u32 {
		match self {
			Self::Near => 2,
			Self::Mid => 1,
			Self::Far => 0,
		}
	}

	pub fn index_count(self) -> u32 {
		let segs = self.rings().saturating_sub(1);
		segs * self.sides() * 6 + self.sides() * 3
	}

	pub fn vertex_count(self) -> u32 {
		self.rings() * self.sides() + 1
	}

	pub fn leaf_size_scale(self) -> f32 {
		match self {
			Self::Near => 1.0,
			Self::Mid => 1.35,
			Self::Far => 1.85,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::render::render_resource::ShaderSize;

	#[test]
	fn storage_strides_match_std430() -> anyhow::Result<()> {
		assert_eq!(PlantDescriptor::SHADER_SIZE.get(), PLANT_DESCRIPTOR_STRIDE);
		assert_eq!(BranchDescriptor::SHADER_SIZE.get(), BRANCH_DESCRIPTOR_STRIDE);
		assert_eq!(LeafDescriptor::SHADER_SIZE.get(), LEAF_DESCRIPTOR_STRIDE);
		assert_eq!(WoodLod::Near.index_count(), 162);
		assert_eq!(WoodLod::Mid.index_count(), 60);
		assert_eq!(WoodLod::Far.index_count(), 27);
		Ok(())
	}

	#[test]
	fn packed_bytes_are_tight() -> anyhow::Result<()> {
		let branch = BranchDescriptor {
			p0: Vec3::ZERO,
			parent_index: PARENT_NONE,
			p1: Vec3::Y * 0.3,
			parent_t: 0.0,
			p2: Vec3::Y,
			plant_index: 0,
			root_radius_m: 0.03,
			tip_radius_m: 0.01,
			stiffness: 0.7,
			keep_rank: 0.0,
			twist: 0.0,
			depth: 0,
			branch_id: 1,
			major_parent: PARENT_NONE,
		};
		assert_eq!(branch.to_std430_bytes().len(), 80);
		Ok(())
	}

	#[test]
	fn leaf_shape_encodes_layer_shell() -> anyhow::Result<()> {
		let mut leaf = LeafDescriptor {
			plant_index: 0,
			parent_branch: 0,
			attach_t: 0.5,
			size_m: 0.1,
			twist: 0.0,
			fold: 0.4,
			flutter: 0.1,
			density_rank: 0.2,
			facing: Vec3::Y,
			leaf_id: 1,
			palette: Vec3::ONE,
			shape: 0.0,
		};
		assert_eq!(leaf.layer_shell(), 0);
		leaf.shape = 0.5;
		assert_eq!(leaf.layer_shell(), 1);
		leaf.shape = 1.0;
		assert_eq!(leaf.layer_shell(), 2);
		Ok(())
	}
}
