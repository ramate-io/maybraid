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

impl GrassPatchHeader {
	pub const FLAG_ACTIVE: u32 = 1 << 0;

	pub fn is_active(self) -> bool {
		self.flags & Self::FLAG_ACTIVE != 0
	}

	pub fn blade_range(self) -> std::ops::Range<u32> {
		self.blade_offset..self.blade_offset.saturating_add(self.blade_count)
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
pub const DRAW_INDEXED_INDIRECT_STRIDE: u64 = 20;

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::render::render_resource::ShaderSize;

	#[test]
	fn storage_strides_match_std430() -> anyhow::Result<()> {
		assert_eq!(BladeDescriptor::SHADER_SIZE.get(), BLADE_DESCRIPTOR_STRIDE);
		assert_eq!(GrassPatchHeader::SHADER_SIZE.get(), PATCH_HEADER_STRIDE);
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
