//! CPU linear-blend skinning (matches Bevy `skinning.wgsl` + `screen-capture` fallbacks).

use bevy::ecs::entity::Entity;
use bevy::math::{Affine3A, Mat4, Vec3};
use bevy::mesh::skinning::{InfluenceIterator, SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::{GlobalTransform, Mesh};

pub struct SkinnedSurface {
	pub positions: Vec<Vec3>,
	pub normals: Vec<Vec3>,
}

/// Skin positions and normals into **world space**.
///
/// Matches GPU palette skinning (`joint_global.affine() * inverse_bind`, blended per
/// influence). Vertices with no live joint weight use [`mesh_world`] (same as rigid
/// meshes) — not raw bind-space positions.
pub fn skin_mesh_world(
	mesh: &Mesh,
	skin: &SkinnedMesh,
	bindposes: &SkinnedMeshInverseBindposes,
	globals: &[(Entity, &GlobalTransform)],
	mesh_world: Affine3A,
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
				(Mat4::from(world.affine()) * *bindpose, collapsed)
			})
		})
		.collect();

	let mut skinned_pos = vec![Vec3::ZERO; positions.len()];
	let mut skinned_nrm = vec![Vec3::ZERO; positions.len()];
	let mut weights = vec![0.0_f32; positions.len()];
	let mut live = vec![0.0_f32; positions.len()];

	for influence in influences {
		let Some(Some((joint, collapsed))) = joint_lookup.get(influence.joint_index.0 as usize)
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
		if !collapsed {
			live[influence.vertex_index] += w;
		}
	}

	let mut out_pos = Vec::with_capacity(positions.len());
	let mut out_nrm = Vec::with_capacity(positions.len());
	for i in 0..positions.len() {
		let bind = Vec3::from_array(positions[i]);
		let bind_world = mesh_world.transform_point3(bind);
		if live[i] <= 1e-4 || weights[i] <= 1e-6 {
			out_pos.push(bind_world);
		} else {
			out_pos.push(skinned_pos[i] / weights[i]);
		}
		if let Some(normals) = normals_src {
			let bind_n = Vec3::from_array(normals[i]);
			let n = if live[i] <= 1e-4 || weights[i] <= 1e-6 {
				mesh_world.transform_vector3(bind_n)
			} else {
				skinned_nrm[i] / weights[i]
			};
			out_nrm.push(if n.length_squared() > 1e-8 { n.normalize() } else { Vec3::Y });
		}
	}

	Some(SkinnedSurface { positions: out_pos, normals: out_nrm })
}

#[derive(Debug, Clone, Copy)]
pub struct SkinWeightCoverage {
	pub live_fraction: f32,
	pub zero_weight_vertices: usize,
}

/// Fraction of vertices with non-collapsed skin influences (see `screen-capture`).
pub fn skin_weight_coverage(
	mesh: &Mesh,
	skin: &SkinnedMesh,
	bindposes: &SkinnedMeshInverseBindposes,
	globals: &[(Entity, &GlobalTransform)],
) -> SkinWeightCoverage {
	let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
	else {
		return SkinWeightCoverage { live_fraction: 0.0, zero_weight_vertices: 0 };
	};
	let influences = InfluenceIterator::new(mesh).ok();
	let joint_lookup: Vec<Option<bool>> = skin
		.joints
		.iter()
		.zip(bindposes.iter())
		.map(|(&joint, _bindpose)| {
			globals.iter().find(|(e, _)| *e == joint).map(|(_, world)| {
				world.to_scale_rotation_translation().0.max_element() >= 1e-3
			})
		})
		.collect();

	let mut live = vec![0.0_f32; positions.len()];
	if let Some(influences) = influences {
		for influence in influences {
			let Some(Some(joint_live)) = joint_lookup.get(influence.joint_index.0 as usize) else {
				continue;
			};
			if *joint_live {
				live[influence.vertex_index] += influence.joint_weight;
			}
		}
	}
	let zero_weight_vertices = live.iter().filter(|w| **w <= 1e-4).count();
	let live_fraction = if positions.is_empty() {
		0.0
	} else {
		(positions.len() - zero_weight_vertices) as f32 / positions.len() as f32
	};
	SkinWeightCoverage { live_fraction, zero_weight_vertices }
}
