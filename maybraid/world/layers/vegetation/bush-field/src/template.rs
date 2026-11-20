//! Tiny immutable tube / leaf templates. Final shape is generated in the vertex shader.

use crate::layout::WoodLod;

/// One template vertex: `t` along the primitive, `radial_or_u` around a tube or across a leaf.
#[derive(Clone, Copy, Debug)]
pub struct TemplateVertex {
	pub t: f32,
	pub radial: f32,
}

#[derive(Clone, Debug)]
pub struct WoodTemplate {
	pub lod: WoodLod,
	pub vertices: Vec<TemplateVertex>,
	pub indices: Vec<u32>,
}

impl WoodTemplate {
	pub fn for_lod(lod: WoodLod) -> Self {
		let rings = lod.rings();
		let sides = lod.sides();
		let mut vertices = Vec::with_capacity((rings * sides + 1) as usize);
		for ring in 0..rings {
			let t = ring as f32 / (rings - 1).max(1) as f32;
			for side in 0..sides {
				vertices.push(TemplateVertex { t, radial: side as f32 / sides as f32 });
			}
		}
		// Tip cap centre. `radial < 0` is the sentinel the wood shader uses.
		vertices.push(TemplateVertex { t: 1.0, radial: -1.0 });
		let cap = (rings * sides) as u32;
		let mut indices = Vec::new();
		for ring in 0..rings.saturating_sub(1) {
			for side in 0..sides {
				let a = ring * sides + side;
				let b = ring * sides + (side + 1) % sides;
				let c = (ring + 1) * sides + side;
				let d = (ring + 1) * sides + (side + 1) % sides;
				indices.extend_from_slice(&[a, c, b, b, c, d]);
			}
		}
		let last = (rings - 1) * sides;
		for side in 0..sides {
			let a = last + side;
			let b = last + (side + 1) % sides;
			indices.extend_from_slice(&[a, cap, b]);
		}
		Self { lod, vertices, indices }
	}

	pub fn vertex_bytes(&self) -> Vec<u8> {
		let mut bytes = Vec::with_capacity(self.vertices.len() * Self::VERTEX_STRIDE as usize);
		for vertex in &self.vertices {
			bytes.extend_from_slice(&vertex.t.to_le_bytes());
			bytes.extend_from_slice(&vertex.radial.to_le_bytes());
		}
		bytes
	}

	pub fn index_bytes(&self) -> Vec<u8> {
		self.indices.iter().flat_map(|i| i.to_le_bytes()).collect()
	}

	pub const VERTEX_STRIDE: u64 = 8;
}

#[derive(Clone, Debug)]
pub struct LeafTemplate {
	pub vertices: Vec<TemplateVertex>,
	pub indices: Vec<u32>,
}

impl LeafTemplate {
	/// Folded diamond: base, left, right, tip. `t` is `v` (base→tip), `radial` is `u`.
	pub fn folded() -> Self {
		Self {
			vertices: vec![
				TemplateVertex { t: 0.0, radial: 0.0 },
				TemplateVertex { t: 0.42, radial: -1.0 },
				TemplateVertex { t: 0.42, radial: 1.0 },
				TemplateVertex { t: 1.0, radial: 0.0 },
			],
			indices: vec![0, 1, 3, 0, 3, 2],
		}
	}

	pub fn vertex_bytes(&self) -> Vec<u8> {
		let mut bytes = Vec::with_capacity(self.vertices.len() * WoodTemplate::VERTEX_STRIDE as usize);
		for vertex in &self.vertices {
			bytes.extend_from_slice(&vertex.t.to_le_bytes());
			bytes.extend_from_slice(&vertex.radial.to_le_bytes());
		}
		bytes
	}

	pub fn index_bytes(&self) -> Vec<u8> {
		self.indices.iter().flat_map(|i| i.to_le_bytes()).collect()
	}

	pub fn index_count(&self) -> u32 {
		self.indices.len() as u32
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn wood_templates_match_lod_counts() -> anyhow::Result<()> {
		for lod in [WoodLod::Near, WoodLod::Mid, WoodLod::Far] {
			let template = WoodTemplate::for_lod(lod);
			assert_eq!(template.indices.len() as u32, lod.index_count());
			assert_eq!(template.vertices.len() as u32, lod.vertex_count());
			assert!(template.vertices.iter().any(|v| v.radial < 0.0));
		}
		Ok(())
	}

	#[test]
	fn leaf_template_is_two_sided_diamond() -> anyhow::Result<()> {
		let leaf = LeafTemplate::folded();
		assert_eq!(leaf.index_count(), 6);
		assert_eq!(leaf.vertices.len(), 4);
		Ok(())
	}
}
