//! Reference CPU skinning (matches `screen-capture` / `skinning.wgsl`).

use bevy::ecs::entity::Entity;
use bevy::math::{Affine3A, Mat4, Vec3};
use bevy::mesh::skinning::{InfluenceIterator, SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::{GlobalTransform, Mesh};

pub fn skin_mesh_world_reference(
	mesh: &Mesh,
	skin: &SkinnedMesh,
	bindposes: &SkinnedMeshInverseBindposes,
	globals: &[(Entity, &GlobalTransform)],
	mesh_world: Affine3A,
) -> Option<Vec<Vec3>> {
	let Some(VertexAttributeValues::Float32x3(positions)) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)
	else {
		return None;
	};
	let influences = InfluenceIterator::new(mesh).ok()?;
	let joint_mats: Vec<Option<(Mat4, bool)>> = skin
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

	let mut skinned = vec![Vec3::ZERO; positions.len()];
	let mut weights = vec![0.0_f32; positions.len()];
	let mut live = vec![0.0_f32; positions.len()];

	for influence in influences {
		let Some(Some((joint_mat, collapsed))) = joint_mats.get(influence.joint_index.0 as usize)
		else {
			continue;
		};
		let Some(&position) = positions.get(influence.vertex_index) else {
			continue;
		};
		let w = influence.joint_weight;
		skinned[influence.vertex_index] += joint_mat.transform_point3(Vec3::from_array(position)) * w;
		weights[influence.vertex_index] += w;
		if !collapsed {
			live[influence.vertex_index] += w;
		}
	}

	let mut out = Vec::with_capacity(positions.len());
	for i in 0..positions.len() {
		let bind = Vec3::from_array(positions[i]);
		if live[i] <= 1e-4 || weights[i] <= 1e-6 {
			out.push(mesh_world.transform_point3(bind));
		} else {
			out.push(skinned[i] / weights[i]);
		}
	}
	Some(out)
}

pub fn bbox(positions: &[Vec3]) -> (Vec3, Vec3) {
	let mut min = Vec3::splat(f32::MAX);
	let mut max = Vec3::splat(f32::MIN);
	for p in positions {
		min = min.min(*p);
		max = max.max(*p);
	}
	(min, max)
}
