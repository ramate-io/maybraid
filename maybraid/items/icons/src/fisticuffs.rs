//! Fisticuffs icon: posed assembly, arm joints only, crop left to framing.

use bevy::mesh::{Indices, VertexAttributeValues};
use bevy::prelude::*;
use characters::{AnimClip, CharacterPartSlot};

/// Authored visibility for the Fisticuffs arms thumbnail.
///
/// Body mesh is one GLB; arms are selected by humanoid joint membership, not
/// a height-slab heuristic. Head / clothing / face stay hidden as part slots.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FisticuffsArmMask;

impl FisticuffsArmMask {
	/// Humanoid v0 arm chain. Hands are forearm children; they inherit skin.
	pub const JOINTS: &'static [&'static str] =
		&["shoulder.L", "humerus.L", "forearm.L", "shoulder.R", "humerus.R", "forearm.R"];

	pub fn shows_joint(name: &str) -> bool {
		Self::JOINTS.iter().copied().any(|joint| joint == name)
	}

	pub fn hides_part(slot: CharacterPartSlot) -> bool {
		slot != CharacterPartSlot::BodyMesh
	}
}

/// Live jab clip at the authored boxing guard (rest / recover).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FisticuffsGuardPose;

impl FisticuffsGuardPose {
	pub fn clip() -> AnimClip {
		AnimClip::fisticuffs_guard()
	}

	pub fn progress() -> f32 {
		AnimClip::fisticuffs_guard_progress()
	}
}

/// Keep triangles whose vertices are skinned to [`FisticuffsArmMask`] joints.
///
/// Joint channels stay so the live armature can pose the remaining mesh.
/// Geometry crop is framing ([`screen_capture::CaptureFraming::FitExtents`]),
/// not this filter.
pub fn isolate_arm_joints(mesh: &Mesh, joint_is_arm: &[bool]) -> Option<Mesh> {
	let keep_vertex = vertex_keeps_arm(mesh, joint_is_arm)?;
	if keep_vertex.iter().filter(|keep| **keep).count() < 3 {
		return None;
	}
	let triangles = mesh_triangles(mesh, keep_vertex.len())?;
	let kept: Vec<[u32; 3]> = triangles
		.into_iter()
		.filter(|tri| {
			tri.iter()
				.all(|&index| keep_vertex.get(index as usize).copied().unwrap_or(false))
		})
		.collect();
	if kept.len() < 2 {
		return None;
	}
	remap_kept_triangles(mesh, &kept)
}

fn vertex_keeps_arm(mesh: &Mesh, joint_is_arm: &[bool]) -> Option<Vec<bool>> {
	let VertexAttributeValues::Uint16x4(indices) = mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)?
	else {
		return None;
	};
	let weights = match mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)? {
		VertexAttributeValues::Float32x4(values) => values.as_slice(),
		_ => return None,
	};
	if indices.len() != weights.len() {
		return None;
	}
	Some(
		indices
			.iter()
			.zip(weights)
			.map(|(joints, weight)| dominant_joint_is_arm(*joints, *weight, joint_is_arm))
			.collect(),
	)
}

fn dominant_joint_is_arm(joints: [u16; 4], weights: [f32; 4], joint_is_arm: &[bool]) -> bool {
	let mut best_i = 0usize;
	let mut best_w = -1.0_f32;
	for (lane, weight) in weights.iter().copied().enumerate() {
		if weight > best_w {
			best_w = weight;
			best_i = lane;
		}
	}
	if best_w <= 1e-4 {
		return false;
	}
	let index = joints[best_i] as usize;
	joint_is_arm.get(index).copied().unwrap_or(false)
}

fn mesh_triangles(mesh: &Mesh, vertex_count: usize) -> Option<Vec<[u32; 3]>> {
	match mesh.indices() {
		Some(Indices::U32(indices)) => triangles_from_indices(indices),
		Some(Indices::U16(indices)) => {
			let indices: Vec<u32> = indices.iter().copied().map(u32::from).collect();
			triangles_from_indices(&indices)
		}
		None => {
			if vertex_count % 3 != 0 {
				return None;
			}
			Some((0..vertex_count as u32).step_by(3).map(|i| [i, i + 1, i + 2]).collect())
		}
	}
}

fn triangles_from_indices(indices: &[u32]) -> Option<Vec<[u32; 3]>> {
	if indices.len() % 3 != 0 {
		return None;
	}
	Some(indices.chunks_exact(3).map(|chunk| [chunk[0], chunk[1], chunk[2]]).collect())
}

fn remap_kept_triangles(mesh: &Mesh, kept: &[[u32; 3]]) -> Option<Mesh> {
	let VertexAttributeValues::Float32x3(positions) = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?
	else {
		return None;
	};
	let mut remap = vec![None; positions.len()];
	let mut new_positions = Vec::new();
	let mut new_indices = Vec::new();
	let normals = mesh.attribute(Mesh::ATTRIBUTE_NORMAL).cloned();
	let joint_index = mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).cloned();
	let joint_weight = mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).cloned();
	let mut new_normals = Vec::new();
	let mut new_joint_index = Vec::new();
	let mut new_joint_weight = Vec::new();
	for tri in kept {
		for &index in tri {
			if remap[index as usize].is_none() {
				remap[index as usize] = Some(new_positions.len() as u32);
				new_positions.push(positions[index as usize]);
				if let Some(VertexAttributeValues::Float32x3(values)) = normals.as_ref() {
					if let Some(normal) = values.get(index as usize) {
						new_normals.push(*normal);
					}
				}
				if let Some(VertexAttributeValues::Uint16x4(values)) = joint_index.as_ref() {
					if let Some(joints) = values.get(index as usize) {
						new_joint_index.push(*joints);
					}
				}
				if let Some(VertexAttributeValues::Float32x4(values)) = joint_weight.as_ref() {
					if let Some(weights) = values.get(index as usize) {
						new_joint_weight.push(*weights);
					}
				}
			}
			new_indices.push(remap[index as usize].expect("remapped"));
		}
	}
	let mut isolated = Mesh::new(mesh.primitive_topology(), mesh.asset_usage);
	isolated.insert_attribute(Mesh::ATTRIBUTE_POSITION, new_positions);
	if !new_normals.is_empty() {
		isolated.insert_attribute(Mesh::ATTRIBUTE_NORMAL, new_normals);
	}
	if !new_joint_index.is_empty() {
		isolated.insert_attribute(
			Mesh::ATTRIBUTE_JOINT_INDEX,
			VertexAttributeValues::Uint16x4(new_joint_index),
		);
	}
	if !new_joint_weight.is_empty() {
		isolated.insert_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, new_joint_weight);
	}
	isolated.insert_indices(Indices::U32(new_indices));
	Some(isolated)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::asset::RenderAssetUsages;
	use bevy::mesh::PrimitiveTopology;

	#[test]
	fn mask_keeps_arm_joints_and_hides_other_parts() {
		assert!(FisticuffsArmMask::shows_joint("humerus.L"));
		assert!(FisticuffsArmMask::shows_joint("forearm.R"));
		assert!(!FisticuffsArmMask::shows_joint("femur.L"));
		assert!(!FisticuffsArmMask::shows_joint("root"));
		assert!(FisticuffsArmMask::hides_part(CharacterPartSlot::HeadMesh));
		assert!(!FisticuffsArmMask::hides_part(CharacterPartSlot::BodyMesh));
	}

	#[test]
	fn guard_pose_is_the_live_jab_rest() {
		assert_eq!(FisticuffsGuardPose::clip(), AnimClip::jab());
		assert_eq!(FisticuffsGuardPose::progress(), 0.0);
	}

	fn skinned_two_tri_mesh() -> Mesh {
		Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
			.with_inserted_attribute(
				Mesh::ATTRIBUTE_POSITION,
				vec![
					[0.0, 0.0, 0.0],
					[0.1, 0.0, 0.0],
					[0.0, 0.1, 0.0],
					[0.0, 1.0, 0.0],
					[0.1, 1.0, 0.0],
					[0.0, 1.1, 0.0],
					[0.2, 1.0, 0.0],
					[0.3, 1.0, 0.0],
					[0.2, 1.1, 0.0],
				],
			)
			.with_inserted_attribute(
				Mesh::ATTRIBUTE_JOINT_INDEX,
				VertexAttributeValues::Uint16x4(vec![
					[0, 0, 0, 0],
					[0, 0, 0, 0],
					[0, 0, 0, 0],
					[1, 0, 0, 0],
					[1, 0, 0, 0],
					[1, 0, 0, 0],
					[1, 0, 0, 0],
					[1, 0, 0, 0],
					[1, 0, 0, 0],
				]),
			)
			.with_inserted_attribute(
				Mesh::ATTRIBUTE_JOINT_WEIGHT,
				vec![
					[1.0_f32, 0.0, 0.0, 0.0],
					[1.0, 0.0, 0.0, 0.0],
					[1.0, 0.0, 0.0, 0.0],
					[1.0, 0.0, 0.0, 0.0],
					[1.0, 0.0, 0.0, 0.0],
					[1.0, 0.0, 0.0, 0.0],
					[1.0, 0.0, 0.0, 0.0],
					[1.0, 0.0, 0.0, 0.0],
					[1.0, 0.0, 0.0, 0.0],
				],
			)
			.with_inserted_indices(Indices::U32(vec![0, 1, 2, 3, 4, 5, 6, 7, 8]))
	}

	#[test]
	fn isolate_keeps_arm_weighted_triangles_only() {
		let joint_is_arm = [false, true];
		let isolated = isolate_arm_joints(&skinned_two_tri_mesh(), &joint_is_arm).expect("arms");
		let VertexAttributeValues::Float32x3(positions) =
			isolated.attribute(Mesh::ATTRIBUTE_POSITION).expect("positions")
		else {
			panic!("float3");
		};
		assert_eq!(positions.len(), 6);
		assert!(positions.iter().all(|p| p[1] >= 0.9));
		assert!(isolated.attribute(Mesh::ATTRIBUTE_JOINT_INDEX).is_some());
		assert!(isolated.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT).is_some());
	}

	#[test]
	fn isolate_drops_a_mesh_with_no_arm_joints() {
		let joint_is_arm = [false, false];
		assert!(isolate_arm_joints(&skinned_two_tri_mesh(), &joint_is_arm).is_none());
	}
}
