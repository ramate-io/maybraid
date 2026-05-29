//! Low-poly noisy blade meshes for [`super::ChicoTuft`] (terrain-style sway, shared anchor).

use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;
use procedural_common::{NoiseConfig, NoiseParams, NoiseType};

use super::{align_blade_direction, spear_directions, spear_length_scale, ChicoTuft};

/// Ring count along each blade (inclusive endpoints); keep small for triangle budget.
const HEIGHT_SEGMENTS: u32 = 4;
/// Triangular cross-section (3 sides × 2 tris × segments).
const SIDE_COUNT: u32 = 3;
const MIN_BLADE_LENGTH: f32 = 1e-4;
const MAX_SWAY_FRACTION_OF_LENGTH: f32 = 0.35;

/// Build one merged mesh: every blade shares the origin and radiates with noisy bent centerlines.
pub fn build_tuft_mesh<M: Material, S>(tuft: &ChicoTuft<M, S>, world_uniform_scale: f32) -> Mesh
where
	S: Clone + Into<MeshMaterial3d<M>>,
{
	let scale = world_uniform_scale.max(1e-8);
	let directions = spear_directions(tuft.spear_count, tuft.seed, tuft.max_tilt_radians);
	let base_radius = (tuft.base_radius * scale).max(1e-6);
	let tip_radius = (base_radius * tuft.tip_radius_fraction).max(0.0);

	let mut positions: Vec<[f32; 3]> = Vec::new();
	let mut indices: Vec<u32> = Vec::new();

	for (blade_idx, dir) in directions.iter().enumerate() {
		if dir.length_squared() < 1e-10 {
			continue;
		}
		let length = (tuft.spear_length
			* spear_length_scale(blade_idx as u32, tuft.seed, 0.72, 1.0)
			* scale)
			.max(MIN_BLADE_LENGTH);
		let rotation = align_blade_direction(*dir);
		if !rotation.is_finite() {
			continue;
		}
		let blade_seed = tuft.seed.wrapping_add(blade_idx as i32);
		append_blade(
			&mut positions,
			&mut indices,
			blade_seed,
			rotation,
			length,
			base_radius,
			tip_radius,
			tuft.noise_frequency,
			tuft.noise_amplitude * scale,
		);
	}

	let mut mesh = Mesh::new(
		PrimitiveTopology::TriangleList,
		bevy::asset::RenderAssetUsages::RENDER_WORLD,
	);
	mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
	mesh.insert_indices(Indices::U32(indices));
	mesh.compute_smooth_normals();
	mesh
}

fn append_blade(
	positions: &mut Vec<[f32; 3]>,
	indices: &mut Vec<u32>,
	blade_seed: i32,
	rotation: Quat,
	length: f32,
	base_radius: f32,
	tip_radius: f32,
	noise_frequency: f32,
	noise_amplitude: f32,
) {
	let noise = NoiseConfig::new(NoiseParams {
		seed: blade_seed,
		frequency: 1.0,
		amplitude: 1.0,
		octaves: 1,
		noise_type: NoiseType::Perlin,
		..Default::default()
	});

	let base_vertex = positions.len();
	let sides = SIDE_COUNT.max(3) as usize;
	let rings = HEIGHT_SEGMENTS.max(1) as usize;
	let max_sway = length * MAX_SWAY_FRACTION_OF_LENGTH;

	for ring in 0..=rings {
		let t = ring as f32 / rings as f32;
		let y = t * length;
		let radius = base_radius + (tip_radius - base_radius) * t;

		let nx = blade_seed as f32 + 0.13;
		let nz = blade_seed as f32 + 29.7;
		let mut sway_x = noise.raw_3d(nx, y * noise_frequency, nz) * noise_amplitude;
		let mut sway_z = noise.raw_3d(nx + 5.1, y * noise_frequency, nz + 2.3) * noise_amplitude;
		sway_x = sway_x.clamp(-max_sway, max_sway);
		sway_z = sway_z.clamp(-max_sway, max_sway);

		let center = Vec3::new(sway_x, y, sway_z);
		for side in 0..sides {
			let angle = side as f32 * std::f32::consts::TAU / sides as f32;
			let local = center + Vec3::new(angle.cos() * radius, 0.0, angle.sin() * radius);
			let p = rotation * local;
			if !p.is_finite() {
				return;
			}
			positions.push(p.to_array());
		}
	}

	for ring in 0..rings {
		for side in 0..sides {
			let i0 = base_vertex + ring * sides + side;
			let i1 = base_vertex + ring * sides + (side + 1) % sides;
			let i2 = base_vertex + (ring + 1) * sides + side;
			let i3 = base_vertex + (ring + 1) * sides + (side + 1) % sides;
			indices.extend_from_slice(&[
				i0 as u32,
				i2 as u32,
				i1 as u32,
				i1 as u32,
				i2 as u32,
				i3 as u32,
			]);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy::mesh::VertexAttributeValues;

	fn test_tuft(seed: i32, amplitude: f32) -> ChicoTuft<StandardMaterial, MeshMaterial3d<StandardMaterial>> {
		ChicoTuft {
			seed,
			noise_amplitude: amplitude,
			noise_frequency: 4.0,
			spear_count: 6,
			..ChicoTuft::default()
		}
	}

	fn max_triangle_edge(pos: &[[f32; 3]], indices: &[u32]) -> f32 {
		let mut max_len = 0.0_f32;
		for tri in indices.chunks_exact(3) {
			for i in 0..3 {
				let a = Vec3::from_array(pos[tri[i] as usize]);
				let b = Vec3::from_array(pos[tri[(i + 1) % 3] as usize]);
				max_len = max_len.max(a.distance(b));
			}
		}
		max_len
	}

	#[test]
	fn tuft_mesh_is_low_poly() -> Result<()> {
		let mesh = build_tuft_mesh(&test_tuft(1, 0.08), 1.0);
		let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION) else {
			anyhow::bail!("expected float positions");
		};
		let idx = mesh.indices().expect("indices");
		let Indices::U32(indices) = idx else {
			anyhow::bail!("expected u32 indices");
		};
		assert!(pos.len() <= 6 * (HEIGHT_SEGMENTS as usize + 1) * SIDE_COUNT as usize);
		assert!(indices.len() <= 6 * HEIGHT_SEGMENTS as usize * SIDE_COUNT as usize * 6);
		Ok(())
	}

	#[test]
	fn noise_bends_blade_vertices() -> Result<()> {
		let flat = build_tuft_mesh(&test_tuft(9, 0.0), 1.0);
		let wavy = build_tuft_mesh(&test_tuft(9, 0.12), 1.0);
		let Some(VertexAttributeValues::Float32x3(flat_p)) =
			flat.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("expected flat positions");
		};
		let Some(VertexAttributeValues::Float32x3(wavy_p)) =
			wavy.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("expected wavy positions");
		};
		assert_eq!(flat_p.len(), wavy_p.len());
		let max_delta: f32 = flat_p
			.iter()
			.zip(wavy_p.iter())
			.map(|(a, b)| (a[0] - b[0]).abs() + (a[2] - b[2]).abs())
			.fold(0.0_f32, f32::max);
		assert!(max_delta > 1e-3, "sway amplitude should offset blade vertices in XZ");
		Ok(())
	}

	#[test]
	fn no_degenerate_long_edges_for_many_seeds() -> Result<()> {
		for seed in 0..512_i32 {
			let mesh = build_tuft_mesh(&test_tuft(seed, 0.08), 0.6);
			let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
			else {
				anyhow::bail!("expected float positions");
			};
			for p in pos {
				assert!(p[0].is_finite() && p[1].is_finite() && p[2].is_finite(), "seed {seed}");
			}
			let Some(Indices::U32(indices)) = mesh.indices() else {
				anyhow::bail!("expected u32 indices");
			};
			if pos.is_empty() {
				continue;
			}
			let max_edge = max_triangle_edge(pos, indices);
			assert!(
				max_edge < 1.5,
				"seed {seed} produced long edge {max_edge} (likely bad rotation/indices)"
			);
		}
		Ok(())
	}

	#[test]
	fn spawn_transform_strips_non_uniform_scale() -> Result<()> {
		let (t, u) = super::super::tuft_spawn_transform(Transform {
			translation: Vec3::new(1.0, 2.0, 3.0),
			rotation: Quat::from_rotation_y(0.5),
			scale: Vec3::new(0.5, 2.0, 0.25),
		});
		assert_eq!(t.scale, Vec3::ONE);
		assert!((u - 2.0).abs() < 1e-5);
		Ok(())
	}
}
