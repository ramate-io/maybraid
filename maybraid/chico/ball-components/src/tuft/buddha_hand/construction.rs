//! Standalone Buddha's-hand mesh construction (widening diamond cross-section per ring).
//!
//! Each finger is a 4-corner diamond/rectangle ring stack — reads as a clustered palm “hand”,
//! not a flat grass blade.

use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;
use procedural_common::{NoiseConfig, NoiseParams, NoiseType};

use super::super::profile::BellyTipProfile;

const MIN_FINGER_LENGTH: f32 = 1e-4;
const MAX_SWAY_FRACTION_OF_LENGTH: f32 = 0.35;

/// One finger strand before cluster merge.
#[derive(Clone, Debug, PartialEq)]
pub struct BuddhaHandElement {
	pub direction: Vec3,
	pub length: f32,
	pub profile: BellyTipProfile,
	pub seed: i32,
}

impl BuddhaHandElement {
	fn rotation(&self) -> Quat {
		align_direction(self.direction)
	}
}

/// Merged mesh from many [`BuddhaHandElement`] fingers sharing one anchor.
#[derive(Clone, Debug, PartialEq)]
pub struct BuddhaHandCluster {
	elements: Vec<BuddhaHandElement>,
	height_segments: u32,
	noise_frequency: f32,
	noise_amplitude: f32,
}

impl BuddhaHandCluster {
	pub fn new(
		elements: Vec<BuddhaHandElement>,
		height_segments: u32,
		noise_frequency: f32,
		noise_amplitude: f32,
	) -> Self {
		Self {
			elements,
			height_segments,
			noise_frequency,
			noise_amplitude,
		}
	}

	pub fn into_mesh(self) -> Mesh {
		let mut positions: Vec<[f32; 3]> = Vec::new();
		let mut indices: Vec<u32> = Vec::new();

		for element in &self.elements {
			if element.direction.length_squared() < 1e-10 {
				continue;
			}
			let length = element.length.max(MIN_FINGER_LENGTH);
			let rotation = element.rotation();
			if !rotation.is_finite() {
				continue;
			}
			append_finger(
				&mut positions,
				&mut indices,
				element,
				rotation,
				length,
				self.height_segments,
				self.noise_frequency,
				self.noise_amplitude,
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
}

fn align_direction(direction: Vec3) -> Quat {
	let up = Vec3::Y;
	let d = direction.normalize_or_zero();
	if d.length_squared() < 1e-12 {
		return Quat::IDENTITY;
	}
	let dot = up.dot(d);
	if dot > 1.0 - 1e-5 {
		return Quat::IDENTITY;
	}
	if dot < -1.0 + 1e-5 {
		return Quat::from_axis_angle(Vec3::X, std::f32::consts::PI);
	}
	Quat::from_rotation_arc(up, d)
}

fn append_finger(
	positions: &mut Vec<[f32; 3]>,
	indices: &mut Vec<u32>,
	element: &BuddhaHandElement,
	rotation: Quat,
	length: f32,
	height_segments: u32,
	noise_frequency: f32,
	noise_amplitude: f32,
) {
	let noise = NoiseConfig::new(NoiseParams {
		seed: element.seed,
		frequency: 1.0,
		amplitude: 1.0,
		octaves: 1,
		noise_type: NoiseType::Perlin,
		..Default::default()
	});

	let base_vertex = positions.len();
	let rings = height_segments.max(1) as usize;
	let max_sway = length * MAX_SWAY_FRACTION_OF_LENGTH;

	for ring in 0..=rings {
		let t = ring as f32 / rings as f32;
		let y = t * length;
		let half = element.profile.half_width_at(t).max(0.0);

		let nx = element.seed as f32 + 0.13;
		let nz = element.seed as f32 + 29.7;
		let mut sway_x = noise.raw_3d(nx, y * noise_frequency, nz) * noise_amplitude;
		let mut sway_z =
			noise.raw_3d(nx + 5.1, y * noise_frequency, nz + 2.3) * noise_amplitude;
		sway_x = sway_x.clamp(-max_sway, max_sway);
		sway_z = sway_z.clamp(-max_sway, max_sway);

		let center = Vec3::new(sway_x, y, sway_z);
		let corners = [
			Vec3::new(half, 0.0, half),
			Vec3::new(-half, 0.0, half),
			Vec3::new(-half, 0.0, -half),
			Vec3::new(half, 0.0, -half),
		];
		for corner in corners {
			let p = rotation * (center + corner);
			if !p.is_finite() {
				return;
			}
			positions.push(p.to_array());
		}
	}

	let sides = 4_usize;
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

	#[test]
	fn buddha_hand_cluster_mesh_has_vertices() -> Result<()> {
		let cluster = BuddhaHandCluster::new(
			vec![BuddhaHandElement {
				direction: Vec3::Y,
				length: 1.0,
				profile: BellyTipProfile {
					base_half_width: 0.02,
					belly_half_width: 0.06,
				},
				seed: 0,
			}],
			4,
			4.0,
			0.08,
		);
		let mesh = cluster.into_mesh();
		let positions = mesh
			.attribute(Mesh::ATTRIBUTE_POSITION)
			.and_then(|a| match a {
				bevy::mesh::VertexAttributeValues::Float32x3(p) => Some(p.len()),
				_ => None,
			})
			.unwrap_or(0);
		assert!(positions > 0);
		Ok(())
	}
}
