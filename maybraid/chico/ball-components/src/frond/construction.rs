//! High-LOD merged frond mesh (shoot tube + lateral leaflet pairs).

use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;

use super::config::FrondConfig;
use super::crown::align_frond_direction;
use super::leaflet::append_spine_shoot_and_leaflets;

/// One frond strand before cluster merge.
#[derive(Clone, Debug, PartialEq)]
pub struct FrondElement {
	pub direction: Vec3,
	pub config: FrondConfig,
	pub seed: i32,
}

impl FrondElement {
	fn rotation(&self) -> Quat {
		align_frond_direction(self.direction)
	}
}

/// Merged mesh from many [`FrondElement`] strands sharing one anchor.
#[derive(Clone, Debug, PartialEq)]
pub struct FrondCluster {
	elements: Vec<FrondElement>,
	shoot_half_radius: f32,
	leaflet_length_scale: f32,
}

impl FrondCluster {
	pub fn new(
		elements: Vec<FrondElement>,
		shoot_half_radius: f32,
		leaflet_length_scale: f32,
	) -> Self {
		Self { elements, shoot_half_radius, leaflet_length_scale }
	}

	pub fn into_mesh(self) -> Mesh {
		let mut positions: Vec<[f32; 3]> = Vec::new();
		let mut indices: Vec<u32> = Vec::new();

		for element in &self.elements {
			if element.direction.length_squared() < 1e-10 {
				continue;
			}
			let rotation = element.rotation();
			if !rotation.is_finite() {
				continue;
			}
			append_frond(
				&mut positions,
				&mut indices,
				&element.config,
				rotation,
				self.shoot_half_radius,
				self.leaflet_length_scale,
			);
		}

		let mut mesh = Mesh::new(
			PrimitiveTopology::TriangleList,
			bevy::asset::RenderAssetUsages::RENDER_WORLD,
		);
		mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
		if !indices.is_empty() {
			mesh.insert_indices(Indices::U32(indices));
		}
		mesh.compute_smooth_normals();
		mesh
	}
}

fn append_frond(
	positions: &mut Vec<[f32; 3]>,
	indices: &mut Vec<u32>,
	config: &FrondConfig,
	rotation: Quat,
	shoot_half_radius: f32,
	leaflet_length_scale: f32,
) {
	let base = positions.len();
	append_spine_shoot_and_leaflets(
		positions,
		indices,
		config,
		shoot_half_radius,
		6,
		leaflet_length_scale,
	);
	for p in &mut positions[base..] {
		let v = rotation * Vec3::from_array(*p);
		if v.is_finite() {
			*p = v.to_array();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy::mesh::VertexAttributeValues;

	#[test]
	fn single_frond_has_shoot_and_lateral_leaflets() -> Result<()> {
		let cluster = FrondCluster::new(
			vec![FrondElement {
				direction: Vec3::new(0.2, -0.9, 0.1),
				config: FrondConfig {
					segments: 8,
					leaflet_count: 24,
					..FrondConfig::default()
				},
				seed: 0,
			}],
			0.02,
			3.0,
		);
		let mesh = cluster.into_mesh();
		let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("expected positions");
		};
		assert!(pos.len() > 80);
		assert!(mesh.indices().is_some());
		Ok(())
	}
}
