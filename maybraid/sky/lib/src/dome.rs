//! Inverted fade sphere. Vertex hue follows elevation; alpha follows XZ radius.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use std::f32::consts::PI;

use crate::{SKY_HORIZON, SKY_NADIR, SKY_ZENITH};

#[derive(Resource, Clone, Copy)]
pub(crate) struct DomeSettings {
	pub inner_fade_m: f32,
	pub outer_fade_m: f32,
	pub sphere_radius_m: f32,
	pub max_alpha: f32,
	pub horizon: Color,
	pub zenith: Color,
	pub nadir: Color,
}

impl Default for DomeSettings {
	fn default() -> Self {
		Self {
			inner_fade_m: crate::DEFAULT_INNER_FADE_M,
			outer_fade_m: crate::DEFAULT_OUTER_FADE_M,
			sphere_radius_m: crate::DEFAULT_SPHERE_RADIUS_M,
			max_alpha: crate::DEFAULT_MAX_ALPHA,
			horizon: SKY_HORIZON,
			zenith: SKY_ZENITH,
			nadir: SKY_NADIR,
		}
	}
}

impl DomeSettings {
	pub(crate) fn fade_sphere(self) -> Mesh {
		let rings = 48u32;
		let segs = 64u32;
		let radius = self.sphere_radius_m;

		let mut positions = Vec::new();
		let mut normals = Vec::new();
		let mut colors = Vec::new();
		let mut indices = Vec::new();

		for ring in 0..=rings {
			let v = ring as f32 / rings as f32;
			let theta = v * PI;
			let y = radius * theta.cos();
			let ring_r = radius * theta.sin();
			for seg in 0..=segs {
				let u = seg as f32 / segs as f32;
				let phi = u * 2.0 * PI;
				let x = ring_r * phi.cos();
				let z = ring_r * phi.sin();
				positions.push([x, y, z]);
				let len = (x * x + y * y + z * z).sqrt().max(1e-5);
				normals.push([-x / len, -y / len, -z / len]);
				colors.push(self.vertex_rgba(x, y, z));
			}
		}

		let verts_per_ring = segs + 1;
		for ring in 0..rings {
			for seg in 0..segs {
				let a = ring * verts_per_ring + seg;
				let b = a + verts_per_ring;
				indices.extend_from_slice(&[a, a + 1, b, a + 1, b + 1, b]);
			}
		}

		let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
		mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
		mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
		mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
		mesh.insert_indices(Indices::U32(indices));
		mesh
	}

	pub(crate) fn vertex_rgba(self, x: f32, y: f32, z: f32) -> [f32; 4] {
		let xz = (x * x + z * z).sqrt();
		let fade = smoothstep(self.inner_fade_m, self.outer_fade_m, xz);
		let alpha = self.max_alpha * fade * fade;
		let elevation = (y / self.sphere_radius_m).clamp(-1.0, 1.0);
		let rgb = self.rgb_at_elevation(elevation);
		[rgb.red, rgb.green, rgb.blue, alpha]
	}

	fn rgb_at_elevation(self, elevation: f32) -> LinearRgba {
		if elevation >= 0.0 {
			lerp_linear(
				self.horizon.to_linear(),
				self.zenith.to_linear(),
				smoothstep(0.0, 1.0, elevation),
			)
		} else {
			lerp_linear(
				self.horizon.to_linear(),
				self.nadir.to_linear(),
				smoothstep(0.0, 1.0, -elevation),
			)
		}
	}
}

fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
	let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

fn lerp_linear(a: LinearRgba, b: LinearRgba, t: f32) -> LinearRgba {
	LinearRgba {
		red: a.red + (b.red - a.red) * t,
		green: a.green + (b.green - a.green) * t,
		blue: a.blue + (b.blue - a.blue) * t,
		alpha: a.alpha + (b.alpha - a.alpha) * t,
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{DEFAULT_MAX_ALPHA, SKY_HORIZON};

	fn mesh_colors(mesh: &Mesh) -> anyhow::Result<&[[f32; 4]]> {
		let values =
			mesh.attribute(Mesh::ATTRIBUTE_COLOR).ok_or_else(|| anyhow::anyhow!("colors"))?;
		match values {
			bevy::mesh::VertexAttributeValues::Float32x4(colors) => Ok(colors),
			_ => Err(anyhow::anyhow!("expected rgba colors")),
		}
	}

	#[test]
	fn fade_sphere_stays_a_wash_not_a_wall() -> anyhow::Result<()> {
		let settings = DomeSettings::default();
		let mesh = settings.fade_sphere();
		let colors = mesh_colors(&mesh)?;
		assert!(colors.iter().any(|c| c[3] < 0.02), "near-axis vertices stay clear");
		let peak = colors.iter().map(|c| c[3]).fold(0.0_f32, f32::max);
		assert!(peak > 0.15 && peak <= DEFAULT_MAX_ALPHA + 1e-4, "peak={peak}");

		let positions = mesh
			.attribute(Mesh::ATTRIBUTE_POSITION)
			.ok_or_else(|| anyhow::anyhow!("positions"))?;
		let bevy::mesh::VertexAttributeValues::Float32x3(positions) = positions else {
			return Err(anyhow::anyhow!("expected xyz positions"));
		};
		let horizon = SKY_HORIZON.to_linear();
		let mut saw_horizon = false;
		for (pos, color) in positions.iter().zip(colors.iter()) {
			let y = pos[1];
			let xz = (pos[0] * pos[0] + pos[2] * pos[2]).sqrt();
			if y.abs() < 80.0 && xz > 1_000.0 {
				let dr = (color[0] - horizon.red).abs();
				let dg = (color[1] - horizon.green).abs();
				let db = (color[2] - horizon.blue).abs();
				assert!(dr + dg + db < 0.12, "horizon vertex should match SKY_HORIZON");
				saw_horizon = true;
			}
		}
		assert!(saw_horizon, "expected a horizon-band vertex");
		Ok(())
	}
}
