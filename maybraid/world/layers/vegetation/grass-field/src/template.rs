//! Tiny immutable ribbon/card templates. Final shape is generated in the vertex shader.

use vegetation_shaders::wesl::GeometryLod;

/// One template vertex: `t` along the blade, `side` in `{-1, +1}`.
#[derive(Clone, Copy, Debug)]
pub struct TemplateVertex {
	pub t: f32,
	pub side: f32,
}

/// CPU-side template mesh for one geometry LOD.
#[derive(Clone, Debug)]
pub struct BladeTemplate {
	pub lod: GeometryLod,
	pub vertices: Vec<TemplateVertex>,
	pub indices: Vec<u32>,
}

impl BladeTemplate {
	pub fn for_lod(lod: GeometryLod) -> Self {
		let segments = lod.segments();
		let mut vertices = Vec::with_capacity(((segments + 1) * 2) as usize);
		for i in 0..=segments {
			let t = i as f32 / segments as f32;
			vertices.push(TemplateVertex { t, side: -1.0 });
			vertices.push(TemplateVertex { t, side: 1.0 });
		}
		let mut indices = Vec::with_capacity((segments * 6) as usize);
		for i in 0..segments {
			let base = i * 2;
			indices.extend_from_slice(&[base, base + 1, base + 2, base + 1, base + 3, base + 2]);
		}
		Self { lod, vertices, indices }
	}

	pub fn vertex_bytes(&self) -> Vec<u8> {
		let mut bytes = Vec::with_capacity(self.vertices.len() * Self::VERTEX_STRIDE as usize);
		for vertex in &self.vertices {
			bytes.extend_from_slice(&vertex.t.to_le_bytes());
			bytes.extend_from_slice(&vertex.side.to_le_bytes());
		}
		bytes
	}

	pub fn index_bytes(&self) -> Vec<u8> {
		self.indices.iter().flat_map(|i| i.to_le_bytes()).collect()
	}

	pub const VERTEX_STRIDE: u64 = 8;
}
