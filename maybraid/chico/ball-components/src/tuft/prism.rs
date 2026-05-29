//! Low-poly prismatic cluster mesh builder (shared by tuft variants).

use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;
use procedural_common::{NoiseConfig, NoiseParams, NoiseType};

const MIN_ELEMENT_LENGTH: f32 = 1e-4;
const MAX_SWAY_FRACTION_OF_LENGTH: f32 = 0.35;

/// One prismatic element in a tuft cluster.
pub(crate) struct PrismaticElement {
	pub direction: Vec3,
	pub length: f32,
	pub base_radius: f32,
	pub tip_radius: f32,
	pub seed: i32,
}

impl PrismaticElement {
	/// Stable +Y → [`Self::direction`] rotation (avoids arc blow-ups near parallel/anti-parallel).
	pub(crate) fn rotation(&self) -> Quat {
		let up = Vec3::Y;
		let d = self.direction.normalize_or_zero();
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
}

/// Merged mesh from prismatic elements sharing one origin.
pub(crate) struct PrismaticCluster {
	elements: Vec<PrismaticElement>,
	height_segments: u32,
	side_count: u32,
	noise_frequency: f32,
	noise_amplitude: f32,
}

impl PrismaticCluster {
	pub(crate) fn new(
		elements: Vec<PrismaticElement>,
		height_segments: u32,
		side_count: u32,
		noise_frequency: f32,
		noise_amplitude: f32,
	) -> Self {
		Self {
			elements,
			height_segments,
			side_count,
			noise_frequency,
			noise_amplitude,
		}
	}

	pub(crate) fn into_mesh(self) -> Mesh {
		let mut positions: Vec<[f32; 3]> = Vec::new();
		let mut indices: Vec<u32> = Vec::new();

		for element in &self.elements {
			if element.direction.length_squared() < 1e-10 {
				continue;
			}
			let length = element.length.max(MIN_ELEMENT_LENGTH);
			let rotation = element.rotation();
			if !rotation.is_finite() {
				continue;
			}
			self.append_element(
				&mut positions,
				&mut indices,
				element,
				rotation,
				length,
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

	fn append_element(
		&self,
		positions: &mut Vec<[f32; 3]>,
		indices: &mut Vec<u32>,
		element: &PrismaticElement,
		rotation: Quat,
		length: f32,
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
		let sides = self.side_count.max(2) as usize;
		let rings = self.height_segments.max(1) as usize;
		let max_sway = length * MAX_SWAY_FRACTION_OF_LENGTH;

		for ring in 0..=rings {
			let t = ring as f32 / rings as f32;
			let y = t * length;
			let radius = element.base_radius + (element.tip_radius - element.base_radius) * t;

			let nx = element.seed as f32 + 0.13;
			let nz = element.seed as f32 + 29.7;
			let mut sway_x =
				noise.raw_3d(nx, y * self.noise_frequency, nz) * self.noise_amplitude;
			let mut sway_z = noise.raw_3d(nx + 5.1, y * self.noise_frequency, nz + 2.3)
				* self.noise_amplitude;
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
}
