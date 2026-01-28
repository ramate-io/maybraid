pub mod scratchpad;

use crate::tree::builder::MeshFromTreeNum;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use noise::{NoiseFn, Perlin};
use render_item::{
	mesh::{IdentifiedMesh, MeshBuilder, MeshId},
	NormalizeChunk,
};
use scratchpad::{generate_unit_disk, generate_unit_triangle};
use std::f32::consts::PI;

/// Configuration for a noisy sphere/ball
/// All balls work in unit space (0-1) and are transformed later
#[derive(Debug, Clone)]
pub struct NoisyBallConfig {
	/// Seed for noise generation
	pub seed: u32,
	/// Base radius of the sphere (in unit space, typically 0.5)
	pub radius: f32,
	/// Noise amplitude for surface variation
	/// Higher values create more pronounced surface bumps
	pub noise_amplitude: f32,
	/// Noise frequency for surface variation
	/// Higher values create finer, more detailed noise patterns
	pub noise_frequency: f32,
	/// Number of noise octaves for fractal detail
	/// More octaves = more detailed but potentially slower
	pub noise_octaves: u32,
}

impl Default for NoisyBallConfig {
	fn default() -> Self {
		Self { seed: 0, radius: 0.5, noise_amplitude: 0.1, noise_frequency: 3.0, noise_octaves: 3 }
	}
}

/// Noisy sphere: a sphere with Perlin noise perturbation for organic surface variation
#[derive(Debug, Clone)]
pub struct NoisyBall {
	config: NoisyBallConfig,
	noise: Perlin,
}

impl NoisyBall {
	pub fn new(config: NoisyBallConfig) -> Self {
		let noise = Perlin::new(config.seed);
		Self { config, noise }
	}
}

/*impl Sdf for NoisyBall {
	/// Distance function for a noisy sphere
	/// The sphere is centered at the origin with configurable radius
	/// Perlin noise is used to perturb the surface for organic variation
	fn distance(&self, p: Vec3) -> f32 {
		// Distance from center
		let dist_from_center = p.length();

		// Base sphere distance (negative inside, positive outside)
		let mut dist = dist_from_center - self.config.radius;

		// Add noise perturbation for surface variation
		// Sample noise at the point's position, scaled by frequency
		let noise_value = self.noise.get([
			p.x as f64 * self.config.noise_frequency as f64,
			p.y as f64 * self.config.noise_frequency as f64,
			p.z as f64 * self.config.noise_frequency as f64,
		]) as f32;

		// Apply noise amplitude to the distance
		// This creates bumps and indentations on the sphere surface
		dist += noise_value * self.config.noise_amplitude;

		dist
	}
}*/

impl NormalizeChunk for NoisyBall {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		CascadeChunk::unit_3d_center_chunk()
			.with_res_2(cascade_chunk.res_2)
			.with_mu(self.config.noise_amplitude + 0.001)
	}
}

impl IdentifiedMesh for NoisyBall {
	fn id(&self) -> MeshId {
		let debug_string = format!("{:?}", self);
		MeshId::new(debug_string)
	}
}

#[derive(Clone, Copy)]
enum ShapeType {
	Disk,
	Rectangle,
	Triangle,
}

impl MeshBuilder for NoisyBall {
	fn build_mesh_impl(&self, cascade_chunk: &CascadeChunk) -> Option<Mesh> {
		// if the resolution is less than 1 just return nothing
		if cascade_chunk.res_2 < 1 {
			return None;
		}

		// if the resolution is less than 5 just spawn a unit sphere
		if cascade_chunk.res_2 < 5 {
			return Some(Mesh::from(Sphere::new(1.0)));
		}

		// Generate a mix of 8 plane meshes (discs, triangles, rectangles) intersecting at the origin
		let num_planes = 8;
		let size = 1.0; // Unit-sized shapes
		let radius = 1.0; // For discs
		let segments = 32; // For discs
		let edge_noise_amplitude = 0.15; // How much to perturb edges
		let edge_noise_frequency = 8.0; // Frequency of edge noise

		// Use Fibonacci sphere algorithm for even distribution of directions
		let golden_angle = PI * (3.0 - (5.0_f32).sqrt());

		// Cycle through shape types for variety
		let shape_types = [
			ShapeType::Disk,
			ShapeType::Rectangle,
			ShapeType::Triangle,
			ShapeType::Disk,
			ShapeType::Triangle,
			ShapeType::Rectangle,
			ShapeType::Disk,
			ShapeType::Triangle,
		];

		let mut all_vertices: Vec<[f32; 3]> = Vec::new();
		let mut all_normals: Vec<[f32; 3]> = Vec::new();
		let mut all_uvs: Vec<[f32; 2]> = Vec::new();
		let mut all_indices: Vec<u32> = Vec::new();

		for i in 0..num_planes {
			// Calculate direction using Fibonacci sphere for even distribution
			let theta = golden_angle * i as f32;
			let y = 1.0 - (2.0 * i as f32) / (num_planes as f32 - 1.0); // y goes from 1 to -1
			let radius_at_y = (1.0 - y * y).sqrt(); // Radius at this y level

			// Calculate direction vector (normal of the plane)
			let x = radius_at_y * theta.cos();
			let z = radius_at_y * theta.sin();
			let direction = Vec3::new(x, y, z).normalize();

			// Generate geometry based on shape type
			let (mut plane_vertices, plane_normals, plane_uvs, plane_indices) = match shape_types[i]
			{
				ShapeType::Disk => generate_unit_triangle(size),
				ShapeType::Rectangle => generate_unit_triangle(size),
				ShapeType::Triangle => generate_unit_triangle(size),
			};

			// Apply noise to edge vertices (not center vertices for discs)
			let is_disk = matches!(shape_types[i], ShapeType::Disk);
			let back_center_index = if is_disk {
				// For discs: front center at 0, back center at segments+2
				Some((segments + 2) as usize)
			} else {
				None
			};

			for (idx, vertex) in plane_vertices.iter_mut().enumerate() {
				// Skip center vertices for discs (front at 0, back at segments+2)
				if is_disk && (idx == 0 || Some(idx) == back_center_index) {
					continue;
				}

				// This is an edge vertex, apply noise
				let noise_x = self.noise.get([
					vertex[0] as f64 * edge_noise_frequency as f64,
					vertex[1] as f64 * edge_noise_frequency as f64,
					(i as f64) * 0.5, // Vary per plane
				]) as f32;
				let noise_y = self.noise.get([
					vertex[0] as f64 * edge_noise_frequency as f64 + 100.0,
					vertex[1] as f64 * edge_noise_frequency as f64 + 100.0,
					(i as f64) * 0.5 + 50.0,
				]) as f32;

				// Perturb in the plane (XY plane before rotation)
				vertex[0] += noise_x * edge_noise_amplitude;
				vertex[1] += noise_y * edge_noise_amplitude;
			}

			// Transform plane to the appropriate orientation
			// Plane's default normal is Vec3::Z, so rotate Z to direction
			let rotation = if direction.abs_diff_eq(Vec3::Z, 1e-4) {
				Quat::IDENTITY
			} else {
				Quat::from_rotation_arc(Vec3::Z, direction)
			};

			// Apply rotation to vertices and normals
			let vertex_offset = all_vertices.len() as u32;
			for (vertex, normal) in plane_vertices.iter().zip(plane_normals.iter()) {
				let v = Vec3::new(vertex[0], vertex[1], vertex[2]);
				let n = Vec3::new(normal[0], normal[1], normal[2]);
				let rotated_v = rotation * v;
				let rotated_n = rotation * n;
				all_vertices.push([rotated_v.x, rotated_v.y, rotated_v.z]);
				all_normals.push([rotated_n.x, rotated_n.y, rotated_n.z]);
			}

			// Add UVs (no transformation needed)
			all_uvs.extend(plane_uvs);

			// Add indices with offset
			all_indices.extend(plane_indices.iter().map(|&idx| idx + vertex_offset));
		}

		// Create the mesh
		let mut mesh = Mesh::new(
			bevy::mesh::PrimitiveTopology::TriangleList,
			bevy::asset::RenderAssetUsages::RENDER_WORLD,
		);
		mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, all_vertices);
		mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, all_normals);
		mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, all_uvs);
		mesh.insert_indices(bevy::mesh::Indices::U32(all_indices));

		Some(mesh)
	}
}

impl MeshFromTreeNum for NoisyBall {
	fn from_tree_num(_tree_num: f32) -> Self {
		Self::new(NoisyBallConfig::default())
	}
}
