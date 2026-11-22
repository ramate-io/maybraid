//! Compact plant / branch / leaf GPU structs. Extra tree fields stay in this crate
//! (shared `layout.wesl` keeps grass `BladeDescriptor`).

use bevy::math::Vec3;
use bevy::render::render_resource::ShaderType;

/// Sentinel parent index: this branch is rooted on the plant, not another branch.
pub const PARENT_NONE: u32 = u32::MAX;

/// Close-up foliage treatment packed into [`PlantDescriptor::flags`] bits 1–2.
///
/// Far LOD still fades hashed cutouts to solid clumps in every style.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[repr(u32)]
pub enum LeafCloseStyle {
	/// Solid painterly clump cards (the pre-hashed look).
	#[default]
	Smooth = 0,
	/// Hashed small-leaf silhouettes with dithered cutout.
	Dither = 1,
	/// Same hashed leaf shapes, hard alpha cutoff, clean edges.
	Crisp = 2,
}

impl LeafCloseStyle {
	pub const FLAG_SHIFT: u32 = 1;
	pub const FLAG_MASK: u32 = 0b11 << Self::FLAG_SHIFT;

	pub fn as_u32(self) -> u32 {
		self as u32
	}

	pub fn from_u32(value: u32) -> Self {
		match value {
			1 => Self::Dither,
			2 => Self::Crisp,
			_ => Self::Smooth,
		}
	}

	pub fn parse_name(name: &str) -> Option<Self> {
		match name {
			"smooth" => Some(Self::Smooth),
			"dither" => Some(Self::Dither),
			"crisp" => Some(Self::Crisp),
			_ => None,
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::Smooth => "smooth",
			Self::Dither => "dither",
			Self::Crisp => "crisp",
		}
	}

	/// Hashed in-card silhouettes (dither or crisp). Smooth cards skip this.
	pub fn uses_cluster(self) -> bool {
		self != Self::Smooth
	}
}

pub const TREE_LOD_NEAR: u32 = 0;
pub const TREE_LOD_MID: u32 = 1;
pub const TREE_LOD_FAR: u32 = 2;
pub const TREE_LOD_COUNT: usize = 3;

pub const PLANT_DESCRIPTOR_STRIDE: u64 = 96;
pub const BRANCH_DESCRIPTOR_STRIDE: u64 = 80;
pub const LEAF_DESCRIPTOR_STRIDE: u64 = 64;

/// CPU-owned patch slice into the persistent plant buffer.
#[derive(Clone, Copy, Debug, ShaderType, PartialEq)]
pub struct TreePatchHeader {
	pub aabb_min: Vec3,
	pub plant_offset: u32,
	pub aabb_max: Vec3,
	pub plant_count: u32,
	pub seed: u32,
	pub coverage: f32,
	pub species: u32,
	pub flags: u32,
}

impl TreePatchHeader {
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
	/// Packed into `flags` bits 8..=31 so layout stride stays 96 bytes.
	pub const GROVE_SHIFT: u32 = 8;

	pub fn is_active(self) -> bool {
		self.flags & Self::FLAG_ACTIVE != 0
	}

	pub fn grove_id(self) -> u32 {
		self.flags >> Self::GROVE_SHIFT
	}

	pub fn with_grove_slot(mut self, slot: u32) -> Self {
		self.flags = (self.flags & ((1 << Self::GROVE_SHIFT) - 1)) | (slot << Self::GROVE_SHIFT);
		self
	}

	pub fn with_active(mut self) -> Self {
		self.flags |= Self::FLAG_ACTIVE;
		self
	}

	pub fn inactive() -> Self {
		Self {
			root: Vec3::ZERO,
			height_m: 0.0,
			up: Vec3::Y,
			plant_id: 0,
			aabb_min: Vec3::ZERO,
			branch_offset: 0,
			aabb_max: Vec3::ZERO,
			branch_count: 0,
			leaf_offset: 0,
			leaf_count: 0,
			seed: 0,
			flags: 0,
			crown_radius_m: 0.0,
			wood_stiffness: 0.0,
			foliage_density: 0.0,
			species: 0.0,
		}
	}

	pub fn with_leaf_close_style(mut self, style: LeafCloseStyle) -> Self {
		self.flags = (self.flags & !LeafCloseStyle::FLAG_MASK)
			| (style.as_u32() << LeafCloseStyle::FLAG_SHIFT);
		self
	}

	pub fn leaf_close_style(self) -> LeafCloseStyle {
		LeafCloseStyle::from_u32(
			(self.flags & LeafCloseStyle::FLAG_MASK) >> LeafCloseStyle::FLAG_SHIFT,
		)
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

impl Default for BranchDescriptor {
	fn default() -> Self {
		Self {
			p0: Vec3::ZERO,
			parent_index: PARENT_NONE,
			p1: Vec3::ZERO,
			parent_t: 0.0,
			p2: Vec3::ZERO,
			plant_index: 0,
			root_radius_m: 0.0,
			tip_radius_m: 0.0,
			stiffness: 0.0,
			keep_rank: 0.0,
			twist: 0.0,
			depth: 0,
			branch_id: 0,
			major_parent: PARENT_NONE,
		}
	}
}

impl BranchDescriptor {
	pub fn is_trunk(self) -> bool {
		self.parent_index == PARENT_NONE
	}

	pub fn parent_is_resolvable(self, count: u32) -> bool {
		self.parent_index == PARENT_NONE || self.parent_index < count
	}

	/// Trunks are depth 0; every child is exactly one deeper than its parent.
	pub fn follows_parent_depth(self, parent: Option<Self>) -> bool {
		match parent {
			None => self.is_trunk() && self.depth == 0,
			Some(parent) => !self.is_trunk() && self.depth == parent.depth.saturating_add(1),
		}
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

impl Default for LeafDescriptor {
	fn default() -> Self {
		Self {
			plant_index: 0,
			parent_branch: 0,
			attach_t: 0.0,
			size_m: 0.0,
			twist: 0.0,
			fold: 0.0,
			flutter: 0.0,
			density_rank: 0.0,
			facing: Vec3::Y,
			leaf_id: 0,
			palette: Vec3::ZERO,
			shape: 0.0,
		}
	}
}

impl LeafDescriptor {
	/// Willow hanging strips use `shape >= 1.5`.
	pub fn is_drape(self) -> bool {
		self.shape >= 1.5
	}

	/// Packed `CanopyLayer::shell_unit`: `0` inner, `0.5` mid, `1` outer.
	pub fn layer_shell(self) -> u32 {
		if self.is_drape() {
			return 2;
		}
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
			TREE_LOD_NEAR => Some(Self::Near),
			TREE_LOD_MID => Some(Self::Mid),
			TREE_LOD_FAR => Some(Self::Far),
			_ => None,
		}
	}

	pub fn bucket(self) -> u32 {
		match self {
			Self::Near => TREE_LOD_NEAR,
			Self::Mid => TREE_LOD_MID,
			Self::Far => TREE_LOD_FAR,
		}
	}

	/// Longitudinal samples including the tip ring (cap is an extra vertex).
	pub fn rings(self) -> u32 {
		match self {
			Self::Near => 6,
			Self::Mid => 4,
			Self::Far => 3,
		}
	}

	pub fn sides(self) -> u32 {
		match self {
			Self::Near => 8,
			Self::Mid => 6,
			Self::Far => 4,
		}
	}

	/// Max branch depth submitted at this LOD (`0` = trunk only).
	pub fn max_depth(self) -> u32 {
		match self {
			Self::Near => 3,
			Self::Mid => 2,
			Self::Far => 1,
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
			Self::Mid => 1.28,
			Self::Far => 1.72,
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
		assert_eq!(WoodLod::Near.index_count(), 264);
		assert_eq!(WoodLod::Mid.index_count(), 126);
		assert_eq!(WoodLod::Far.index_count(), 60);
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
			size_m: 0.4,
			twist: 0.0,
			fold: 0.4,
			flutter: 0.03,
			density_rank: 0.1,
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
		leaf.shape = 2.0;
		assert!(leaf.is_drape());
		assert_eq!(leaf.layer_shell(), 2);
		Ok(())
	}

	#[test]
	fn plant_flags_pack_leaf_close_style() -> anyhow::Result<()> {
		let plant = PlantDescriptor {
			root: Vec3::ZERO,
			height_m: 10.0,
			up: Vec3::Y,
			plant_id: 1,
			aabb_min: Vec3::ZERO,
			branch_offset: 0,
			aabb_max: Vec3::ONE,
			branch_count: 1,
			leaf_offset: 0,
			leaf_count: 0,
			seed: 1,
			flags: PlantDescriptor::FLAG_ACTIVE,
			crown_radius_m: 4.0,
			wood_stiffness: 0.8,
			foliage_density: 1.0,
			species: 2.0,
		};
		assert_eq!(plant.leaf_close_style(), LeafCloseStyle::Smooth);
		let dithered = plant.with_leaf_close_style(LeafCloseStyle::Dither);
		assert!(dithered.is_active());
		assert_eq!(dithered.leaf_close_style(), LeafCloseStyle::Dither);
		assert_eq!(
			dithered.with_leaf_close_style(LeafCloseStyle::Crisp).leaf_close_style(),
			LeafCloseStyle::Crisp
		);
		assert_eq!(LeafCloseStyle::parse_name("dither"), Some(LeafCloseStyle::Dither));
		assert_eq!(LeafCloseStyle::default(), LeafCloseStyle::Smooth);
		Ok(())
	}
}
