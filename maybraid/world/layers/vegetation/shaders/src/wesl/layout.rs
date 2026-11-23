//! CPU mirror of `layout.wesl` with documented units, spaces, and alignment.

use bevy::math::{Mat4, Vec3, Vec4};
use bevy::render::render_resource::ShaderType;

/// Near ribbon LOD (full curvature).
pub const GRASS_LOD_NEAR: u32 = 0;
/// Mid ribbon LOD (fewer segments, slightly wider).
pub const GRASS_LOD_MID: u32 = 1;
/// Far card / clump LOD (own compacted list).
pub const GRASS_LOD_CARD: u32 = 2;
/// Number of submitted LOD buckets.
pub const GRASS_LOD_COUNT: usize = 3;

pub const GRASS_VIEW_KIND_CAMERA: u32 = 0;
pub const GRASS_VIEW_KIND_SHADOW: u32 = 1;

/// Geometry recipe for a submitted bucket. Topology is a tiny immutable template.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GeometryLod {
	Near,
	Mid,
	Card,
}

impl GeometryLod {
	pub fn from_bucket(bucket: u32) -> Option<Self> {
		match bucket {
			GRASS_LOD_NEAR => Some(Self::Near),
			GRASS_LOD_MID => Some(Self::Mid),
			GRASS_LOD_CARD => Some(Self::Card),
			_ => None,
		}
	}

	pub fn bucket(self) -> u32 {
		match self {
			Self::Near => GRASS_LOD_NEAR,
			Self::Mid => GRASS_LOD_MID,
			Self::Card => GRASS_LOD_CARD,
		}
	}

	/// Segment count along the blade. Cards are a single quad.
	pub fn segments(self) -> u32 {
		match self {
			Self::Near => 8,
			Self::Mid => 4,
			Self::Card => 1,
		}
	}

	pub fn index_count(self) -> u32 {
		self.segments() * 6
	}

	pub fn vertex_count(self) -> u32 {
		(self.segments() + 1) * 2
	}

	/// Width scale applied to survivors so coverage holds as density drops.
	pub fn width_scale(self) -> f32 {
		match self {
			Self::Near => 1.0,
			Self::Mid => 2.35,
			Self::Card => 5.8,
		}
	}
}

/// Persistent per-blade descriptor. World metres. std430.
#[derive(Clone, Copy, Debug, ShaderType, PartialEq)]
pub struct BladeDescriptor {
	pub root: Vec3,
	pub height_m: f32,
	pub normal: Vec3,
	pub width_m: f32,
	pub facing: Vec3,
	pub plant_id: u32,
	/// x = bend, y = taper, z = stiffness, w = density_rank.
	pub shape: Vec4,
	/// xyz = linear RGB variation, w = species id as f32.
	pub palette: Vec4,
}

impl BladeDescriptor {
	pub fn density_rank(self) -> f32 {
		self.shape.w
	}

	pub fn bend(self) -> f32 {
		self.shape.x
	}

	pub fn taper(self) -> f32 {
		self.shape.y
	}

	pub fn stiffness(self) -> f32 {
		self.shape.z
	}

	pub fn conservative_extent_m(self, wind_strength_m: f32) -> f32 {
		let wind = crate::wesl::wind::max_wind_displacement_m(
			self.height_m,
			wind_strength_m,
			self.stiffness(),
		);
		self.height_m + wind + self.width_m * GeometryLod::Card.width_scale()
	}

	/// Tight std430 packing matching `layout.wesl` (no runtime-array length prefix).
	pub fn to_std430_bytes(self) -> [u8; BLADE_DESCRIPTOR_STRIDE as usize] {
		let mut out = [0u8; BLADE_DESCRIPTOR_STRIDE as usize];
		pack_vec3(&mut out, 0, self.root);
		pack_f32(&mut out, 12, self.height_m);
		pack_vec3(&mut out, 16, self.normal);
		pack_f32(&mut out, 28, self.width_m);
		pack_vec3(&mut out, 32, self.facing);
		pack_u32(&mut out, 44, self.plant_id);
		pack_vec4(&mut out, 48, self.shape);
		pack_vec4(&mut out, 64, self.palette);
		out
	}
}

fn pack_f32(out: &mut [u8], offset: usize, value: f32) {
	out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn pack_u32(out: &mut [u8], offset: usize, value: u32) {
	out[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

fn pack_vec3(out: &mut [u8], offset: usize, value: Vec3) {
	pack_f32(out, offset, value.x);
	pack_f32(out, offset + 4, value.y);
	pack_f32(out, offset + 8, value.z);
}

fn pack_vec4(out: &mut [u8], offset: usize, value: Vec4) {
	pack_f32(out, offset, value.x);
	pack_f32(out, offset + 4, value.y);
	pack_f32(out, offset + 8, value.z);
	pack_f32(out, offset + 12, value.w);
}

fn pack_mat4(out: &mut [u8], offset: usize, mat: Mat4) {
	let cols = mat.to_cols_array();
	for (i, value) in cols.iter().enumerate() {
		pack_f32(out, offset + i * 4, *value);
	}
}

/// One CPU-owned patch slice into the persistent blade buffer.
#[derive(Clone, Copy, Debug, ShaderType, PartialEq)]
pub struct GrassPatchHeader {
	pub aabb_min: Vec3,
	pub blade_offset: u32,
	pub aabb_max: Vec3,
	pub blade_count: u32,
	pub seed: u32,
	pub coverage: f32,
	pub species: u32,
	pub flags: u32,
}

/// GPU grove slot. AABB/sphere plus item spans. Inactive slots keep `flags = 0`.
///
/// Primary is plants (woody) or blades (grass). Secondary/tertiary are branches
/// and leaves; grass leaves them empty. std430 array stride is 64.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GroveRecord {
	pub aabb_min: Vec3,
	pub flags: u32,
	pub aabb_max: Vec3,
	pub seed: u32,
	pub plant_off: u32,
	pub plant_count: u32,
	pub branch_off: u32,
	pub branch_count: u32,
	pub leaf_off: u32,
	pub leaf_count: u32,
	_pad: [u32; 2],
}

/// Compact visible-grove span consumed by pass-2 select. `prefix` is exclusive.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VisibleGroveSpan {
	pub offset: u32,
	pub count: u32,
	pub prefix: u32,
	pub lod: u32,
}

impl GrassPatchHeader {
	pub const FLAG_ACTIVE: u32 = 1 << 0;

	pub fn is_active(self) -> bool {
		self.flags & Self::FLAG_ACTIVE != 0
	}

	pub fn blade_range(self) -> std::ops::Range<u32> {
		self.blade_offset..self.blade_offset.saturating_add(self.blade_count)
	}

	pub fn grove_record(self) -> GroveRecord {
		GroveRecord {
			aabb_min: self.aabb_min,
			flags: self.flags,
			aabb_max: self.aabb_max,
			seed: self.seed,
			plant_off: self.blade_offset,
			plant_count: self.blade_count,
			branch_off: 0,
			branch_count: 0,
			leaf_off: 0,
			leaf_count: 0,
			_pad: [0; 2],
		}
	}
}

impl GroveRecord {
	pub const FLAG_ACTIVE: u32 = 1 << 0;

	pub fn active(
		aabb_min: Vec3,
		aabb_max: Vec3,
		seed: u32,
		plants: (u32, u32),
		branches: (u32, u32),
		leaves: (u32, u32),
	) -> Self {
		Self {
			aabb_min,
			flags: Self::FLAG_ACTIVE,
			aabb_max,
			seed,
			plant_off: plants.0,
			plant_count: plants.1,
			branch_off: branches.0,
			branch_count: branches.1,
			leaf_off: leaves.0,
			leaf_count: leaves.1,
			_pad: [0; 2],
		}
	}

	pub fn is_active(self) -> bool {
		self.flags & Self::FLAG_ACTIVE != 0
	}

	pub fn sphere(self) -> (Vec3, f32) {
		let center = (self.aabb_min + self.aabb_max) * 0.5;
		let radius = (self.aabb_max - self.aabb_min).length() * 0.5;
		(center, radius)
	}

	pub fn height_m(self) -> f32 {
		(self.aabb_max.y - self.aabb_min.y).max(1.0)
	}

	pub fn plant_range(self) -> std::ops::Range<u32> {
		self.plant_off..self.plant_off.saturating_add(self.plant_count)
	}

	pub fn branch_range(self) -> std::ops::Range<u32> {
		self.branch_off..self.branch_off.saturating_add(self.branch_count)
	}

	pub fn leaf_range(self) -> std::ops::Range<u32> {
		self.leaf_off..self.leaf_off.saturating_add(self.leaf_count)
	}

	pub fn blade_range(self) -> std::ops::Range<u32> {
		self.plant_range()
	}

	pub fn to_std430_bytes(self) -> [u8; GROVE_RECORD_STRIDE as usize] {
		let mut out = [0u8; GROVE_RECORD_STRIDE as usize];
		pack_vec3(&mut out, 0, self.aabb_min);
		pack_u32(&mut out, 12, self.flags);
		pack_vec3(&mut out, 16, self.aabb_max);
		pack_u32(&mut out, 28, self.seed);
		pack_u32(&mut out, 32, self.plant_off);
		pack_u32(&mut out, 36, self.plant_count);
		pack_u32(&mut out, 40, self.branch_off);
		pack_u32(&mut out, 44, self.branch_count);
		pack_u32(&mut out, 48, self.leaf_off);
		pack_u32(&mut out, 52, self.leaf_count);
		out
	}
}

impl VisibleGroveSpan {
	pub fn contains(self, compact_index: u32) -> bool {
		compact_index >= self.prefix && compact_index < self.prefix.saturating_add(self.count)
	}

	pub fn item_index(self, compact_index: u32) -> u32 {
		self.offset.saturating_add(compact_index.saturating_sub(self.prefix))
	}

	pub fn to_std430_bytes(self) -> [u8; VISIBLE_SPAN_STRIDE as usize] {
		let mut out = [0u8; VISIBLE_SPAN_STRIDE as usize];
		pack_u32(&mut out, 0, self.offset);
		pack_u32(&mut out, 4, self.count);
		pack_u32(&mut out, 8, self.prefix);
		pack_u32(&mut out, 12, self.lod);
		out
	}
}

/// Per-view uniforms. std140. Resolution is the *internal* render size.
#[derive(Clone, Copy, Debug, ShaderType, PartialEq)]
pub struct GrassViewUniform {
	pub clip_from_world: Mat4,
	pub world_from_clip: Mat4,
	pub world_position: Vec4,
	pub prev_world_position: Vec4,
	pub resolution: Vec4,
	pub wind: Vec4,
	pub lod_distances: Vec4,
	pub lod_density: Vec4,
	pub counts: Vec4,
	pub light_direction: Vec4,
}

impl GrassViewUniform {
	pub fn to_std140_bytes(self) -> [u8; 256] {
		let mut bytes = [0u8; 256];
		pack_mat4(&mut bytes, 0, self.clip_from_world);
		pack_mat4(&mut bytes, 64, self.world_from_clip);
		pack_vec4(&mut bytes, 128, self.world_position);
		pack_vec4(&mut bytes, 144, self.prev_world_position);
		pack_vec4(&mut bytes, 160, self.resolution);
		pack_vec4(&mut bytes, 176, self.wind);
		pack_vec4(&mut bytes, 192, self.lod_distances);
		pack_vec4(&mut bytes, 208, self.lod_density);
		pack_vec4(&mut bytes, 224, self.counts);
		pack_vec4(&mut bytes, 240, self.light_direction);
		bytes
	}

	pub fn view_kind(self) -> u32 {
		self.resolution.w as u32
	}

	pub fn render_height_px(self) -> f32 {
		self.resolution.y
	}

	pub fn fov_y_radians(self) -> f32 {
		self.resolution.z
	}

	pub fn time_s(self) -> f32 {
		self.world_position.w
	}

	pub fn prev_time_s(self) -> f32 {
		self.prev_world_position.w
	}
}

#[derive(Clone, Copy, Debug, ShaderType, PartialEq)]
pub struct GrassDrawParams {
	pub lod_bucket: u32,
	pub index_count: u32,
	pub vertex_count: u32,
	pub view_kind: u32,
}

impl GrassDrawParams {
	pub fn to_std140_bytes(self) -> [u8; 16] {
		let mut out = [0u8; 16];
		out[0..4].copy_from_slice(&self.lod_bucket.to_le_bytes());
		out[4..8].copy_from_slice(&self.index_count.to_le_bytes());
		out[8..12].copy_from_slice(&self.vertex_count.to_le_bytes());
		out[12..16].copy_from_slice(&self.view_kind.to_le_bytes());
		out
	}
}

/// GPU `draw_indexed_indirect` args. Tightly packed 20 bytes (not `ShaderType` /
/// std140 — the indirect command encoder requires this layout).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawIndexedIndirect {
	pub index_count: u32,
	pub instance_count: u32,
	pub first_index: u32,
	pub base_vertex: i32,
	pub first_instance: u32,
}

impl DrawIndexedIndirect {
	pub fn empty(index_count: u32) -> Self {
		Self { index_count, instance_count: 0, first_index: 0, base_vertex: 0, first_instance: 0 }
	}

	pub fn with_instances(self, instance_count: u32) -> Self {
		Self { instance_count, ..self }
	}

	pub fn to_bytes(self) -> [u8; DRAW_INDEXED_INDIRECT_STRIDE as usize] {
		let mut out = [0u8; DRAW_INDEXED_INDIRECT_STRIDE as usize];
		out[0..4].copy_from_slice(&self.index_count.to_le_bytes());
		out[4..8].copy_from_slice(&self.instance_count.to_le_bytes());
		out[8..12].copy_from_slice(&self.first_index.to_le_bytes());
		out[12..16].copy_from_slice(&self.base_vertex.to_le_bytes());
		out[16..20].copy_from_slice(&self.first_instance.to_le_bytes());
		out
	}
}

pub const BLADE_DESCRIPTOR_STRIDE: u64 = 80;
pub const PATCH_HEADER_STRIDE: u64 = 48;
pub const GROVE_RECORD_STRIDE: u64 = 64;
pub const VISIBLE_SPAN_STRIDE: u64 = 16;
pub const DISPATCH_INDIRECT_STRIDE: u64 = 12;
pub const DRAW_INDEXED_INDIRECT_STRIDE: u64 = 20;

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::render::render_resource::ShaderSize;

	#[test]
	fn storage_strides_match_std430() -> anyhow::Result<()> {
		assert_eq!(BladeDescriptor::SHADER_SIZE.get(), BLADE_DESCRIPTOR_STRIDE);
		assert_eq!(GrassPatchHeader::SHADER_SIZE.get(), PATCH_HEADER_STRIDE);
		assert_eq!(u64::try_from(std::mem::size_of::<GroveRecord>())?, GROVE_RECORD_STRIDE);
		assert_eq!(u64::try_from(std::mem::size_of::<VisibleGroveSpan>())?, VISIBLE_SPAN_STRIDE);
		assert_eq!(
			u64::try_from(std::mem::size_of::<DrawIndexedIndirect>())?,
			DRAW_INDEXED_INDIRECT_STRIDE
		);
		assert_eq!(GrassViewUniform::SHADER_SIZE.get(), 256);
		assert_eq!(GrassDrawParams::SHADER_SIZE.get(), 16);
		Ok(())
	}

	#[test]
	fn lod_templates_are_small() -> anyhow::Result<()> {
		assert_eq!(GeometryLod::Near.vertex_count(), 18);
		assert_eq!(GeometryLod::Mid.vertex_count(), 10);
		assert_eq!(GeometryLod::Card.vertex_count(), 4);
		assert_eq!(GeometryLod::Near.index_count(), 48);
		Ok(())
	}
}
