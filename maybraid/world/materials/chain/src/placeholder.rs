//! Temporary interlocking-torus chain. A04 / #1125 replaces this mesh.
//!
//! The generator writes the UV and vertex-color contract the material needs
//! so playgrounds and tests can exercise the shader before runtime skinning
//! exists.

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;

/// Preview chain that satisfies the mythical-chain geometry contract.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceholderChainSpec {
	pub link_count: u32,
	pub ring_radius: f32,
	pub tube_radius: f32,
	/// Along-chain length in meters (start anchor at y=0).
	pub length: f32,
	pub segments_ring: u32,
	pub segments_tube: u32,
	/// Sine offset in X. Stand-in for A04 skinning / A05 motion.
	pub bend_amplitude: f32,
	pub bend_waves: f32,
}

impl Default for PlaceholderChainSpec {
	fn default() -> Self {
		Self {
			link_count: 14,
			ring_radius: 0.16,
			tube_radius: 0.052,
			length: 3.1,
			segments_ring: 20,
			segments_tube: 12,
			bend_amplitude: 0.0,
			bend_waves: 0.85,
		}
	}
}

impl PlaceholderChainSpec {
	pub fn bent(self, amplitude: f32) -> Self {
		Self { bend_amplitude: amplitude, ..self }
	}

	pub fn with_link_count(self, link_count: u32) -> Self {
		Self { link_count: link_count.max(2), ..self }
	}
}

/// One placeholder chain mesh with UV.y along the length and per-link color ids.
pub fn placeholder_chain_mesh(spec: PlaceholderChainSpec) -> Mesh {
	let links = spec.link_count.max(2);
	let ring = spec.segments_ring.max(6);
	let tube = spec.segments_tube.max(4);
	let spacing = spec.length / (links.saturating_sub(1) as f32).max(1.0);

	let mut positions = Vec::new();
	let mut uvs = Vec::new();
	let mut colors = Vec::new();
	let mut indices = Vec::new();

	for link in 0..links {
		let center_y = link as f32 * spacing;
		let odd = link % 2 == 1;
		let base = positions.len() as u32;
		for i in 0..ring {
			let theta = TAU * (i as f32) / (ring as f32);
			for j in 0..tube {
				let phi = TAU * (j as f32) / (tube as f32);
				let (x, y, z) = torus_point(spec.ring_radius, spec.tube_radius, theta, phi, odd);
				let mut p = Vec3::new(x, y + center_y, z);
				if spec.bend_amplitude.abs() > 1e-5 {
					let t = (p.y / spec.length).clamp(0.0, 1.0);
					p.x += spec.bend_amplitude * (t * spec.bend_waves * TAU).sin();
					p.z += spec.bend_amplitude * 0.28 * (t * spec.bend_waves * TAU + 0.7).cos();
				}
				positions.push([p.x, p.y, p.z]);
				uvs.push([
					j as f32 / tube as f32,
					(link as f32 + i as f32 / ring as f32) / links as f32,
				]);
				colors.push([link as f32, 0.0, if odd { 1.0 } else { 0.0 }, 1.0]);
			}
		}
		for i in 0..ring {
			let i0 = i;
			let i1 = (i + 1) % ring;
			for j in 0..tube {
				let j0 = j;
				let j1 = (j + 1) % tube;
				let a = base + i0 * tube + j0;
				let b = base + i1 * tube + j0;
				let c = base + i1 * tube + j1;
				let d = base + i0 * tube + j1;
				indices.extend_from_slice(&[a, b, c, a, c, d]);
			}
		}
	}

	let mut mesh = Mesh::new(
		PrimitiveTopology::TriangleList,
		RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
	);
	mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
	mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
	mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
	mesh.insert_indices(Indices::U32(indices));
	mesh.compute_normals();
	mesh
}

/// Write per-link localized damage into vertex color G. Opt-in extension.
pub fn apply_localized_damage(mesh: &mut Mesh, per_link: &[f32]) {
	let Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) =
		mesh.attribute_mut(Mesh::ATTRIBUTE_COLOR)
	else {
		return;
	};
	for color in colors.iter_mut() {
		let link = color[0].round().clamp(0.0, 10_000.0) as usize;
		if let Some(damage) = per_link.get(link) {
			color[1] = damage.clamp(0.0, 1.0);
		}
	}
}

fn torus_point(ring: f32, tube: f32, theta: f32, phi: f32, odd: bool) -> (f32, f32, f32) {
	let radial = ring + tube * phi.cos();
	if odd {
		// Hole along X so it passes through the even link.
		(tube * phi.sin(), radial * theta.cos(), radial * theta.sin())
	} else {
		// Hole along Y (chain axis).
		(radial * theta.cos(), tube * phi.sin(), radial * theta.sin())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn color_attr(mesh: &Mesh) -> anyhow::Result<&[[f32; 4]]> {
		match mesh.attribute(Mesh::ATTRIBUTE_COLOR) {
			Some(bevy::mesh::VertexAttributeValues::Float32x4(colors)) => Ok(colors),
			_ => Err(anyhow::anyhow!("missing float4 color")),
		}
	}

	fn uv_attr(mesh: &Mesh) -> anyhow::Result<&[[f32; 2]]> {
		match mesh.attribute(Mesh::ATTRIBUTE_UV_0) {
			Some(bevy::mesh::VertexAttributeValues::Float32x2(uvs)) => Ok(uvs),
			_ => Err(anyhow::anyhow!("missing float2 uv")),
		}
	}

	#[test]
	fn writes_along_chain_uv_and_per_link_ids() -> anyhow::Result<()> {
		let spec = PlaceholderChainSpec::default().with_link_count(8);
		let mesh = placeholder_chain_mesh(spec);
		let colors = color_attr(&mesh)?;
		let uvs = uv_attr(&mesh)?;
		assert_eq!(colors.len(), uvs.len());
		assert!(mesh.attribute(Mesh::ATTRIBUTE_NORMAL).is_some());

		let mut seen = [false; 8];
		let mut max_v = 0.0_f32;
		let mut min_v = 1.0_f32;
		for (color, uv) in colors.iter().zip(uvs.iter()) {
			let link = color[0].round() as usize;
			assert!(link < 8, "link id {link}");
			seen[link] = true;
			assert!((color[3] - 1.0).abs() < 1e-5);
			assert!((0.0..=1.0).contains(&uv[0]));
			assert!((0.0..=1.0).contains(&uv[1]));
			max_v = max_v.max(uv[1]);
			min_v = min_v.min(uv[1]);
			let expected_band = link as f32 / 8.0;
			assert!(
				uv[1] + 1e-4 >= expected_band,
				"uv.y {} should be at/after link band {}",
				uv[1],
				expected_band
			);
		}
		assert!(seen.iter().all(|flag| *flag), "every link needs verts");
		assert!(min_v < 0.05);
		assert!(max_v > 0.85);
		Ok(())
	}

	#[test]
	fn localized_damage_writes_color_g() -> anyhow::Result<()> {
		let spec = PlaceholderChainSpec::default().with_link_count(4);
		let mut mesh = placeholder_chain_mesh(spec);
		apply_localized_damage(&mut mesh, &[0.0, 0.4, 0.8, 1.0]);
		let colors = color_attr(&mesh)?;
		let mut found = [false; 4];
		for color in colors {
			let link = color[0].round() as usize;
			assert!((color[1] - [0.0, 0.4, 0.8, 1.0][link]).abs() < 1e-5);
			found[link] = true;
		}
		assert!(found.iter().all(|flag| *flag));
		Ok(())
	}

	#[test]
	fn bend_moves_vertices_off_the_axis() -> anyhow::Result<()> {
		let straight = placeholder_chain_mesh(PlaceholderChainSpec::default());
		let bent = placeholder_chain_mesh(PlaceholderChainSpec::default().bent(0.35));
		let Some(bevy::mesh::VertexAttributeValues::Float32x3(a)) =
			straight.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			return Err(anyhow::anyhow!("straight positions"));
		};
		let Some(bevy::mesh::VertexAttributeValues::Float32x3(b)) =
			bent.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			return Err(anyhow::anyhow!("bent positions"));
		};
		let drift = a.iter().zip(b.iter()).map(|(p, q)| (p[0] - q[0]).abs()).fold(0.0, f32::max);
		assert!(drift > 0.1, "bend should offset X, drift={drift}");
		Ok(())
	}
}
