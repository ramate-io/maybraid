//! CPU linear-blend skinning (matches in-game `skinning.wgsl`).

use bevy::ecs::entity::Entity;
use bevy::math::{Mat4, Vec3};
use bevy::mesh::skinning::{InfluenceIterator, SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::{GlobalTransform, Mesh};

pub struct SkinnedSurface {
	pub positions: Vec<Vec3>,
	pub normals: Vec<Vec3>,
}

/// Skin positions and normals into world space (mesh entity transform ignored for skinned draws).
pub fn skin_mesh_world(
	mesh: &Mesh,
	skin: &SkinnedMesh,
	bindposes: &SkinnedMeshInverseBindposes,
	globals: &[(Entity, &GlobalTransform)],
) -> Option<SkinnedSurface> {
	let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
	else {
		return None;
	};
	let normals_src = mesh.attribute(Mesh::ATTRIBUTE_NORMAL).and_then(|attr| match attr {
		VertexAttributeValues::Float32x3(n) => Some(n),
		_ => None,
	});
	let influences = InfluenceIterator::new(mesh).ok()?;

	let joint_lookup: Vec<Option<(Mat4, bool)>> = skin
		.joints
		.iter()
		.zip(bindposes.iter())
		.map(|(&joint, bindpose)| {
			globals.iter().find(|(e, _)| *e == joint).map(|(_, world)| {
				let collapsed = world.to_scale_rotation_translation().0.max_element() < 1e-3;
				(world.to_matrix() * *bindpose, collapsed)
			})
		})
		.collect();

	let mut skinned_pos = vec![Vec3::ZERO; positions.len()];
	let mut skinned_nrm = vec![Vec3::ZERO; positions.len()];
	let mut weights = vec![0.0_f32; positions.len()];

	for influence in influences {
		let Some(Some((joint, _collapsed))) = joint_lookup.get(influence.joint_index.0 as usize)
		else {
			continue;
		};
		let Some(&position) = positions.get(influence.vertex_index) else {
			continue;
		};
		let w = influence.joint_weight;
		skinned_pos[influence.vertex_index] +=
			joint.transform_point3(Vec3::from_array(position)) * w;
		if let Some(normals) = normals_src {
			if let Some(&normal) = normals.get(influence.vertex_index) {
				skinned_nrm[influence.vertex_index] +=
					joint.transform_vector3(Vec3::from_array(normal)) * w;
			}
		}
		weights[influence.vertex_index] += w;
	}

	let mut out_pos = Vec::with_capacity(positions.len());
	let mut out_nrm = Vec::with_capacity(positions.len());
	for i in 0..positions.len() {
		if weights[i] <= 1e-6 {
			out_pos.push(Vec3::from_array(positions[i]));
		} else {
			out_pos.push(skinned_pos[i] / weights[i]);
		}
		if let Some(normals) = normals_src {
			let n = if weights[i] <= 1e-6 {
				Vec3::from_array(normals[i])
			} else {
				skinned_nrm[i] / weights[i]
			};
			out_nrm.push(if n.length_squared() > 1e-8 { n.normalize() } else { Vec3::Y });
		}
	}

	Some(SkinnedSurface { positions: out_pos, normals: out_nrm })
}
