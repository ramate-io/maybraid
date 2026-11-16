//! Continuous bent tube / string fixture. Same `flow_s_m` contract as the
//! placeholder chain: `Mesh::ATTRIBUTE_UV_1.x` is cumulative rest arc length
//! in metres for every vertex of a ring; `.y` is the total rest length.
//!
//! This is a small material-contract mesh, not A04 chain generation.

use std::f32::consts::TAU;

use bevy::asset::RenderAssetUsages;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::*;

/// Bent continuous tube used to prove energy travel on rope / cable / vine.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContinuousTubeSpec {
	pub radius: f32,
	pub length: f32,
	pub rings: u32,
	pub segments: u32,
	pub bend_amplitude: f32,
	pub bend_waves: f32,
}

impl Default for ContinuousTubeSpec {
	fn default() -> Self {
		Self {
			radius: 0.045,
			length: 3.1,
			rings: 48,
			segments: 10,
			bend_amplitude: 0.28,
			bend_waves: 0.9,
		}
	}
}

impl ContinuousTubeSpec {
	pub fn bent(self, amplitude: f32) -> Self {
		Self { bend_amplitude: amplitude, ..self }
	}
}

/// Rest-space centerline of the tube, before bend, along +Y.
pub fn rest_centerline(s_m: f32) -> Vec3 {
	Vec3::new(0.0, s_m, 0.0)
}

/// Bent centerline used by the fixture. `s_m` is still the rest arc length.
pub fn bent_centerline(spec: ContinuousTubeSpec, s_m: f32) -> Vec3 {
	let t = (s_m / spec.length.max(1e-4)).clamp(0.0, 1.0);
	let mut p = rest_centerline(s_m);
	if spec.bend_amplitude.abs() > 1e-5 {
		p.x += spec.bend_amplitude * (t * spec.bend_waves * TAU).sin();
		p.z += spec.bend_amplitude * 0.34 * (t * spec.bend_waves * TAU + 0.6).cos();
	}
	p
}

/// Continuous tube with duplicated U seam and per-ring rest arc length.
pub fn continuous_tube_mesh(spec: ContinuousTubeSpec) -> Mesh {
	let rings = spec.rings.max(4);
	let segs = spec.segments.max(4);
	let ring_verts = segs + 1;
	let mut positions = Vec::new();
	let mut uvs = Vec::new();
	let mut flow_uvs = Vec::new();
	let mut colors = Vec::new();
	let mut indices = Vec::new();

	for i in 0..=rings {
		let s = spec.length * (i as f32) / (rings as f32);
		let center = bent_centerline(spec, s);
		let next = bent_centerline(spec, spec.length * ((i + 1) as f32) / (rings as f32));
		let tangent = (next - center).normalize_or(Vec3::Y);
		let helper = if tangent.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
		let normal = tangent.cross(helper).normalize_or(Vec3::X);
		let binormal = tangent.cross(normal).normalize_or(Vec3::Z);
		for j in 0..=segs {
			let u = j as f32 / segs as f32;
			let phi = TAU * u;
			let radial = normal * phi.cos() + binormal * phi.sin();
			let p = center + radial * spec.radius;
			positions.push([p.x, p.y, p.z]);
			uvs.push([u, s / spec.length.max(1e-4)]);
			flow_uvs.push([s, spec.length]);
			colors.push([0.0, 0.0, 0.0, 1.0]);
		}
	}

	for i in 0..rings {
		let row = i * ring_verts;
		let next = (i + 1) * ring_verts;
		for j in 0..segs {
			let a = row + j;
			let b = next + j;
			let c = next + j + 1;
			let d = row + j + 1;
			indices.extend_from_slice(&[a, b, c, a, c, d]);
		}
	}

	let mut mesh = Mesh::new(
		PrimitiveTopology::TriangleList,
		RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
	);
	mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
	mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
	mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, flow_uvs);
	mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
	mesh.insert_indices(Indices::U32(indices));
	mesh.compute_normals();
	mesh
}

/// Joint count for the skinned-tube contract fixture.
pub const SKINNED_TUBE_JOINTS: u32 = 4;

/// Skinned continuous tube. Weights blend between adjacent rest-length joints.
/// A04 owns real chain skinning; this only proves the halo copies `SkinnedMesh`.
pub fn skinned_tube_mesh(spec: ContinuousTubeSpec) -> (Mesh, Vec<Mat4>) {
	let bind_spec = ContinuousTubeSpec { bend_amplitude: 0.0, ..spec };
	let mut mesh = continuous_tube_mesh(bind_spec);
	let joints = SKINNED_TUBE_JOINTS.max(2);
	let Some(bevy::mesh::VertexAttributeValues::Float32x2(flow)) =
		mesh.attribute(Mesh::ATTRIBUTE_UV_1)
	else {
		return (mesh, vec![Mat4::IDENTITY; joints as usize]);
	};
	let mut indices = Vec::with_capacity(flow.len());
	let mut weights = Vec::with_capacity(flow.len());
	for uv1 in flow {
		let t = (uv1[0] / spec.length.max(1e-4)).clamp(0.0, 0.999);
		let scaled = t * (joints - 1) as f32;
		let a = scaled.floor() as u16;
		let f = scaled.fract();
		indices.push([a, a + 1, 0, 0]);
		weights.push([1.0 - f, f, 0.0, 0.0]);
	}
	mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_INDEX, VertexAttributeValues::Uint16x4(indices));
	mesh.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, weights);
	let bind = (0..joints)
		.map(|i| {
			let s = spec.length * (i as f32) / (joints - 1) as f32;
			Mat4::from_translation(rest_centerline(s)).inverse()
		})
		.collect();
	(mesh, bind)
}

/// Spawn a posed skinned tube plus joints. Caller parents the mesh as needed.
pub fn spawn_skinned_tube(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
	spec: ContinuousTubeSpec,
	transform: Transform,
	name: &str,
) -> (Entity, Vec<Entity>) {
	let (mesh, inverses) = skinned_tube_mesh(spec);
	let mesh_handle = meshes.add(mesh);
	let inverse = bindposes.add(SkinnedMeshInverseBindposes::from(inverses));
	let joints = SKINNED_TUBE_JOINTS.max(2);
	let mut joint_entities = Vec::new();
	let root = commands
		.spawn((Name::new(format!("{name}-armature")), Transform::IDENTITY, Visibility::default()))
		.id();
	for i in 0..joints {
		let s = spec.length * (i as f32) / (joints - 1) as f32;
		let joint = commands
			.spawn((
				Name::new(format!("{name}-joint-{i}")),
				Transform::from_translation(rest_centerline(s)),
				Visibility::default(),
			))
			.id();
		commands.entity(root).add_child(joint);
		joint_entities.push(joint);
	}
	let mesh_entity = commands
		.spawn((
			Name::new(name.to_string()),
			Mesh3d(mesh_handle),
			transform,
			Visibility::default(),
			SkinnedMesh { inverse_bindposes: inverse, joints: joint_entities.clone() },
		))
		.id();
	(mesh_entity, joint_entities)
}

#[cfg(test)]
mod tests {
	use super::*;

	fn flow_attr(mesh: &Mesh) -> anyhow::Result<&[[f32; 2]]> {
		match mesh.attribute(Mesh::ATTRIBUTE_UV_1) {
			Some(VertexAttributeValues::Float32x2(uvs)) => Ok(uvs),
			_ => Err(anyhow::anyhow!("missing uv1")),
		}
	}

	#[test]
	fn rings_share_one_rest_arc_length() -> anyhow::Result<()> {
		let spec = ContinuousTubeSpec::default();
		let mesh = continuous_tube_mesh(spec);
		let flow = flow_attr(&mesh)?;
		let ring_verts = spec.segments.max(4) + 1;
		for ring in 0..=spec.rings.max(4) {
			let start = (ring * ring_verts) as usize;
			let s0 = flow[start][0];
			for k in 0..ring_verts as usize {
				assert!((flow[start + k][0] - s0).abs() < 1e-5);
				assert!((flow[start + k][1] - spec.length).abs() < 1e-5);
			}
		}
		let last = flow.last().ok_or_else(|| anyhow::anyhow!("empty"))?;
		assert!((last[0] - spec.length).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn bend_leaves_rest_s_unchanged() -> anyhow::Result<()> {
		let straight = continuous_tube_mesh(ContinuousTubeSpec::default().bent(0.0));
		let bent = continuous_tube_mesh(ContinuousTubeSpec::default().bent(0.4));
		let a = flow_attr(&straight)?;
		let b = flow_attr(&bent)?;
		assert_eq!(a.len(), b.len());
		for (p, q) in a.iter().zip(b.iter()) {
			assert!((p[0] - q[0]).abs() < 1e-5);
		}
		Ok(())
	}

	#[test]
	fn skinned_weights_cover_the_length() -> anyhow::Result<()> {
		let spec = ContinuousTubeSpec::default();
		let (mesh, bind) = skinned_tube_mesh(spec);
		assert_eq!(bind.len(), SKINNED_TUBE_JOINTS as usize);
		assert!(mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_some());
		assert!(mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).is_some());
		Ok(())
	}
}
