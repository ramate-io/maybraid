//! Standalone flat spear (grass blade) mesh construction.
//!
//! Each spear is a flat ribbon: rectangular segments along the strand, with a **pentagon**
//! profile (base → belly → tip) on the **last segment only**.

use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;
use procedural_common::{NoiseConfig, NoiseParams, NoiseType};

use super::super::profile::BellyTipProfile;
use super::super::sway::strand_sway_at;

const MIN_SPEAR_LENGTH: f32 = 1e-4;
const MAX_SWAY_FRACTION_OF_LENGTH: f32 = 0.35;

/// One flat grass-like spear before cluster merge.
#[derive(Clone, Debug, PartialEq)]
pub struct SpearElement {
	pub direction: Vec3,
	pub length: f32,
	pub profile: BellyTipProfile,
	pub seed: i32,
}

impl SpearElement {
	fn tangent_frame(&self) -> (Vec3, Vec3, Vec3) {
		blade_tangent_frame(self.direction)
	}
}

/// Growth axis `up`, ribbon width `width`, broad-face normal `outward` (horizontal, away from tuft center).
fn blade_tangent_frame(direction: Vec3) -> (Vec3, Vec3, Vec3) {
	let up = direction.normalize_or_zero();
	let mut outward = Vec3::new(up.x, 0.0, up.z);
	if outward.length_squared() < 1e-8 {
		// Perfectly vertical strand: pick a stable broad-side normal.
		outward = Vec3::Z;
	} else {
		outward = outward.normalize();
	}
	let mut width = outward.cross(up);
	if width.length_squared() < 1e-8 {
		width = up.cross(Vec3::X);
	}
	width = width.normalize_or_zero();
	(up, width, outward)
}

/// Merged mesh from many flat [`SpearElement`] ribbons sharing one anchor.
#[derive(Clone, Debug, PartialEq)]
pub struct SpearCluster {
	elements: Vec<SpearElement>,
	height_segments: u32,
	noise_frequency: f32,
	noise_amplitude: f32,
}

impl SpearCluster {
	pub fn new(
		elements: Vec<SpearElement>,
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
			let length = element.length.max(MIN_SPEAR_LENGTH);
			append_ribbon(
				&mut positions,
				&mut indices,
				element,
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

/// Cross-section along a spear: either a width pair or the tip apex.
#[derive(Clone, Copy)]
enum CrossSection {
	Pair { left: u32, right: u32 },
	Apex { tip: u32 },
}

/// Normalized height where the final pentagonal taper segment begins.
fn last_segment_start(bend_segments: u32) -> f32 {
	let segments = bend_segments.max(1) as f32;
	(segments - 1.0) / segments
}

/// Body ring heights before the final taper segment (`0 … t_split` inclusive).
fn body_ts(bend_segments: u32) -> Vec<f32> {
	let segments = bend_segments.max(1) as usize;
	(0..segments).map(|i| i as f32 / segments as f32).collect()
}

/// Cross-section heights inside the final segment: midpoint belly sample, then tip.
fn last_segment_ts(bend_segments: u32) -> Vec<f32> {
	let t_split = last_segment_start(bend_segments);
	let mut ts = Vec::new();
	if t_split < 1.0 - 1e-6 {
		ts.push(t_split + 0.5 * (1.0 - t_split));
	}
	ts.push(1.0);
	ts
}

fn strand_center(
	up: Vec3,
	width: Vec3,
	outward: Vec3,
	length: f32,
	t: f32,
	noise: &NoiseConfig,
	seed: i32,
	noise_frequency: f32,
	noise_amplitude: f32,
	max_sway: f32,
) -> Vec3 {
	let sway = strand_sway_at(
		noise,
		seed,
		t,
		noise_frequency,
		noise_amplitude,
		max_sway,
	);
	up * (t * length) + width * sway.right + outward * sway.forward
}

fn push_cross_section(
	positions: &mut Vec<[f32; 3]>,
	center: Vec3,
	width: Vec3,
	half: f32,
	is_tip: bool,
) -> Option<CrossSection> {
	if !center.is_finite() {
		return None;
	}
	if is_tip {
		let tip = positions.len() as u32;
		positions.push(center.to_array());
		Some(CrossSection::Apex { tip })
	} else {
		let left = center - width * half;
		let right = center + width * half;
		if !left.is_finite() || !right.is_finite() {
			return None;
		}
		let left_idx = positions.len() as u32;
		positions.push(left.to_array());
		let right_idx = positions.len() as u32;
		positions.push(right.to_array());
		Some(CrossSection::Pair { left: left_idx, right: right_idx })
	}
}

fn stitch_cross_sections(indices: &mut Vec<u32>, lower: CrossSection, upper: CrossSection) {
	match (lower, upper) {
		(CrossSection::Pair { left: l0, right: r0 }, CrossSection::Pair { left: l1, right: r1 }) => {
			indices.extend_from_slice(&[l0, l1, r0, r0, l1, r1]);
		}
		(CrossSection::Pair { left: l0, right: r0 }, CrossSection::Apex { tip }) => {
			indices.extend_from_slice(&[l0, tip, r0]);
		}
		_ => {}
	}
}

fn append_ribbon(
	positions: &mut Vec<[f32; 3]>,
	indices: &mut Vec<u32>,
	element: &SpearElement,
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

	let (up, width, outward) = element.tangent_frame();
	if up.length_squared() < 1e-12 {
		return;
	}

	let max_sway = length * MAX_SWAY_FRACTION_OF_LENGTH;
	// Scale the sway coordinate with the segment count so each segment crosses new noise
	// features; otherwise extra segments only refine the same smooth bow and
	// `height_segments` has no visible effect.
	let sway_frequency = noise_frequency * height_segments.max(1) as f32;
	let mut sections: Vec<CrossSection> = Vec::new();

	for t in body_ts(height_segments) {
		let half = element.profile.half_width_at(t);
		if half < 1e-6 {
			continue;
		}
		let center = strand_center(
			up,
			width,
			outward,
			length,
			t,
			&noise,
			element.seed,
			sway_frequency,
			noise_amplitude,
			max_sway,
		);
		let Some(section) = push_cross_section(positions, center, width, half, false) else {
			return;
		};
		sections.push(section);
	}

	for t in last_segment_ts(height_segments) {
		let is_tip = t >= 1.0 - 1e-6;
		let half = element.profile.half_width_at(t);
		if !is_tip && half < 1e-6 {
			continue;
		}
		let center = strand_center(
			up,
			width,
			outward,
			length,
			t,
			&noise,
			element.seed,
			sway_frequency,
			noise_amplitude,
			max_sway,
		);
		let Some(section) =
			push_cross_section(positions, center, width, half, is_tip)
		else {
			return;
		};
		sections.push(section);
	}

	for window in sections.windows(2) {
		stitch_cross_sections(indices, window[0], window[1]);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy::mesh::VertexAttributeValues;

	#[test]
	fn spear_ribbon_faces_outward_not_edge_on() -> Result<()> {
		let direction = Vec3::new(0.2, 0.98, 0.0).normalize();
		let cluster = SpearCluster::new(
			vec![SpearElement {
				direction,
				length: 1.0,
				profile: BellyTipProfile {
					base_half_width: 0.01,
					belly_half_width: 0.03,
				},
				seed: 0,
			}],
			4,
			1.0,
			0.08,
		);
		let mesh = cluster.into_mesh();
		let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("missing positions");
		};
		let Some(bevy::mesh::Indices::U32(indices)) = mesh.indices() else {
			anyhow::bail!("missing indices");
		};
		assert!(pos.len() >= 4);
		assert!(!indices.is_empty());

		let (_, width, outward) = blade_tangent_frame(direction);
		let mut max_width = 0.0_f32;
		for p in pos {
			let v = Vec3::from_array(*p);
			max_width = max_width.max(v.length());
		}
		assert!(max_width > 0.01, "ribbon should have extent");
		assert!(outward.length() > 0.9, "broad face should have stable outward normal");
		assert!(width.dot(outward).abs() < 1e-4, "width axis should be perpendicular to outward");
		Ok(())
	}

	#[test]
	fn default_spear_tuft_visible_from_positive_z() -> Result<()> {
		use crate::tuft::spear::{SpearTuft, SpearTuftShape};
		use bevy::prelude::StandardMaterial;

		let tuft = SpearTuft::<StandardMaterial, MeshMaterial3d<StandardMaterial>>::from_shape(
			SpearTuftShape::default(),
			MeshMaterial3d::<StandardMaterial>::default(),
		);
		let mesh = tuft.build_mesh(1.0);
		let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("missing positions");
		};
		let Some(bevy::mesh::Indices::U32(indices)) = mesh.indices() else {
			anyhow::bail!("missing indices");
		};
		assert!(!pos.is_empty(), "default mesh should have vertices");
		assert!(!indices.is_empty(), "default mesh should have triangles");

		let mut max_z_extent = 0.0_f32;
		for p in pos {
			max_z_extent = max_z_extent.max(p[2].abs());
		}
		assert!(
			max_z_extent > 0.05,
			"default tuft should have visible extent toward +Z camera, got {max_z_extent}"
		);
		Ok(())
	}

	#[test]
	fn spear_tip_reaches_full_length() -> Result<()> {
		let direction = Vec3::Y;
		let length = 0.9_f32;
		let mesh = SpearCluster::new(
			vec![SpearElement {
				direction,
				length,
				profile: BellyTipProfile {
					base_half_width: 0.008,
					belly_half_width: 0.022,
				},
				seed: 0,
			}],
			2,
			1.0,
			0.08,
		)
		.into_mesh();
		let Some(VertexAttributeValues::Float32x3(pos)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("missing positions");
		};
		let max_y = pos.iter().map(|p| p[1]).fold(0.0_f32, f32::max);
		assert!(
			(max_y - length).abs() < 1e-4,
			"tip apex should reach full length, got {max_y} vs {length}"
		);
		Ok(())
	}

	#[test]
	fn pentagon_on_last_segment_only() -> Result<()> {
		// One segment total → full-height pentagon (5 verts).
		let single = SpearCluster::new(
			vec![SpearElement {
				direction: Vec3::Y,
				length: 1.0,
				profile: BellyTipProfile {
					base_half_width: 0.01,
					belly_half_width: 0.03,
				},
				seed: 0,
			}],
			1,
			0.0,
			0.0,
		)
		.into_mesh();
		let Some(VertexAttributeValues::Float32x3(single_pos)) =
			single.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("missing positions");
		};
		assert_eq!(single_pos.len(), 5);

		// Two segments → rectangular lower half, pentagon taper on upper half (7 verts).
		let split = SpearCluster::new(
			vec![SpearElement {
				direction: Vec3::Y,
				length: 1.0,
				profile: BellyTipProfile {
					base_half_width: 0.01,
					belly_half_width: 0.03,
				},
				seed: 0,
			}],
			2,
			0.0,
			0.0,
		)
		.into_mesh();
		let Some(VertexAttributeValues::Float32x3(split_pos)) =
			split.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("missing positions");
		};
		assert_eq!(split_pos.len(), 7);
		Ok(())
	}

	#[test]
	fn fewer_bend_segments_use_fewer_vertices() -> Result<()> {
		let element = SpearElement {
			direction: Vec3::Y,
			length: 1.0,
			profile: BellyTipProfile {
				base_half_width: 0.01,
				belly_half_width: 0.03,
			},
			seed: 0,
		};
		let straight = SpearCluster::new(vec![element.clone()], 1, 0.0, 0.0).into_mesh();
		let kinky = SpearCluster::new(vec![element], 5, 0.0, 0.0).into_mesh();
		let vertex_count = |mesh: Mesh| -> usize {
			mesh.attribute(Mesh::ATTRIBUTE_POSITION)
				.and_then(|a| match a {
					bevy::mesh::VertexAttributeValues::Float32x3(p) => Some(p.len()),
					_ => None,
				})
				.unwrap_or(0)
		};
		assert!(vertex_count(straight) < vertex_count(kinky));
		Ok(())
	}
}
