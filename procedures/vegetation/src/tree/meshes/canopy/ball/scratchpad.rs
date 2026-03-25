use bevy::prelude::*;
use std::f32::consts::PI;

/// Generate a unit triangle mesh in the XY plane (normal pointing along +Z)
/// Returns (vertices, normals, uvs, indices)
pub fn generate_unit_triangle(
	size: f32,
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>) {
	// Positions (simple right triangle scaled by `size`)
	let vertices = vec![
		[0.0, 0.0, 0.0],  // bottom-left
		[size, 0.0, 0.0], // bottom-right
		[0.0, size, 0.0], // top-left
	];

	// Normals (+Z for all)
	let normals = vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]];

	// UV coordinates
	let uvs = vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]];

	// Indices (one triangle)
	let indices = vec![0, 1, 2];

	(vertices, normals, uvs, indices)
}

/// Generate a unit disk mesh in the XY plane (normal pointing along +Z)
/// Returns (vertices, normals, uvs, indices)
/// Includes both front and back faces for double-sided rendering
pub fn generate_unit_disk(
	radius: f32,
	segments: u32,
) -> (Vec<[f32; 3]>, Vec<[f32; 3]>, Vec<[f32; 2]>, Vec<u32>) {
	let mut vertices = Vec::new();
	let mut normals = Vec::new();
	let mut uvs = Vec::new();
	let mut indices = Vec::new();

	// Front face center vertex
	vertices.push([0.0, 0.0, 0.0]);
	normals.push([0.0, 0.0, 1.0]);
	uvs.push([0.5, 0.5]);

	// Generate front face vertices around the circle
	for i in 0..=segments {
		let angle = 2.0 * PI * i as f32 / segments as f32;
		let x = radius * angle.cos();
		let y = radius * angle.sin();
		vertices.push([x, y, 0.0]);
		normals.push([0.0, 0.0, 1.0]);
		// UV coordinates from center (0.5, 0.5) to edge
		let u = 0.5 + 0.5 * angle.cos();
		let v = 0.5 + 0.5 * angle.sin();
		uvs.push([u, v]);
	}

	// Generate front face triangle indices (fan from center, counter-clockwise)
	for i in 0..segments {
		indices.push(0); // Center vertex
		indices.push(i + 1);
		indices.push(i + 2);
	}

	// Back face center vertex
	let front_vertex_count = vertices.len() as u32;
	vertices.push([0.0, 0.0, 0.0]);
	normals.push([0.0, 0.0, -1.0]);
	uvs.push([0.5, 0.5]);

	// Generate back face vertices around the circle (same positions, flipped normals)
	for i in 0..=segments {
		let angle = 2.0 * PI * i as f32 / segments as f32;
		let x = radius * angle.cos();
		let y = radius * angle.sin();
		vertices.push([x, y, 0.0]);
		normals.push([0.0, 0.0, -1.0]);
		// UV coordinates from center (0.5, 0.5) to edge
		let u = 0.5 + 0.5 * angle.cos();
		let v = 0.5 + 0.5 * angle.sin();
		uvs.push([u, v]);
	}

	// Generate back face triangle indices (fan from center, clockwise/reversed)
	for i in 0..segments {
		indices.push(front_vertex_count); // Back center vertex
		indices.push(front_vertex_count + i + 2); // Reversed order
		indices.push(front_vertex_count + i + 1);
	}

	(vertices, normals, uvs, indices)
}
