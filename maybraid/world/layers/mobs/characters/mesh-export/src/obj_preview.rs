//! Load a Wavefront OBJ (positions + triangular faces) for GPU re-import preview.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;

pub fn load_obj_mesh(path: &Path) -> Option<Mesh> {
	let file = File::open(path).ok()?;
	let reader = BufReader::new(file);
	let mut positions: Vec<[f32; 3]> = Vec::new();
	let mut faces: Vec<u32> = Vec::new();
	for line in reader.lines().map_while(Result::ok) {
		if line.starts_with("v ") {
			let mut parts = line.split_whitespace();
			parts.next();
			let x = parts.next()?.parse().ok()?;
			let y = parts.next()?.parse().ok()?;
			let z = parts.next()?.parse().ok()?;
			positions.push([x, y, z]);
		} else if line.starts_with("f ") {
			let mut tri = Vec::new();
			for token in line.split_whitespace().skip(1) {
				let idx = token.split('/').next()?.parse::<u32>().ok()? - 1;
				tri.push(idx);
			}
			if tri.len() >= 3 {
				for i in 1..tri.len() - 1 {
					faces.extend([tri[0], tri[i], tri[i + 1]]);
				}
			}
		}
	}
	if positions.is_empty() || faces.is_empty() {
		return None;
	}
	let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, default());
	mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, VertexAttributeValues::Float32x3(positions));
	mesh.insert_indices(Indices::U32(faces));
	Some(mesh)
}
