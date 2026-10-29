//! Bind-space terminal points for forearms on skinned character meshes.

use bevy::mesh::{
	skinning::{InfluenceIterator, SkinnedMeshInverseBindposes},
	Mesh, VertexAttributeValues,
};
use bevy::prelude::Vec3;

const FULL_WEIGHT_TOLERANCE: f32 = 0.01;
const CAP_Y_TOLERANCE: f32 = 1.0e-4;

/// Finds each forearm's distal cap centroid in that joint's local bind space.
///
/// `joints` contains the skin-joint indices for left and right forearms. Only
/// vertices whose aggregate weight for a side is approximately one are used,
/// so the returned point follows the actual skinned mesh terminal rather than
/// the forearm joint origin or a family-wide length estimate.
pub(super) fn forearm_terminal_points(
	mesh: &Mesh,
	inverse_bindposes: &SkinnedMeshInverseBindposes,
	joints: [Option<usize>; 2],
) -> [Option<Vec3>; 2] {
	let Ok(VertexAttributeValues::Float32x3(positions)) =
		mesh.try_attribute(Mesh::ATTRIBUTE_POSITION)
	else {
		return [None, None];
	};
	let Ok(VertexAttributeValues::Uint16x4(joint_indices)) =
		mesh.try_attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
	else {
		return [None, None];
	};
	let Ok(VertexAttributeValues::Float32x4(joint_weights)) =
		mesh.try_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
	else {
		return [None, None];
	};
	if positions.len() != joint_indices.len() || positions.len() != joint_weights.len() {
		return [None, None];
	}

	let Ok(influences) = InfluenceIterator::new(mesh) else {
		return [None, None];
	};
	let mut side_weights = [vec![0.0_f32; positions.len()], vec![0.0_f32; positions.len()]];
	let mut invalid_weight = [vec![false; positions.len()], vec![false; positions.len()]];
	for influence in influences {
		for side in 0..2 {
			if joints[side] != Some(influence.joint_index.0 as usize) {
				continue;
			}
			let weight = influence.joint_weight;
			if !weight.is_finite() || weight < 0.0 {
				invalid_weight[side][influence.vertex_index] = true;
			} else {
				side_weights[side][influence.vertex_index] += weight;
			}
		}
	}

	let mut points = [None, None];
	for side in 0..2 {
		let Some(joint) = joints[side] else {
			continue;
		};
		let Some(inverse_bindpose) = inverse_bindposes.get(joint).copied() else {
			continue;
		};
		if !inverse_bindpose.is_finite() {
			continue;
		}

		let mut candidates = Vec::new();
		let mut max_y = f32::NEG_INFINITY;
		for (vertex_index, position) in positions.iter().enumerate() {
			let total_weight = side_weights[side][vertex_index];
			if invalid_weight[side][vertex_index]
				|| !total_weight.is_finite()
				|| (total_weight - 1.0).abs() > FULL_WEIGHT_TOLERANCE
				|| !position.iter().all(|component| component.is_finite())
			{
				continue;
			}
			let local = inverse_bindpose.transform_point3(Vec3::from_array(*position));
			if !local.is_finite() {
				continue;
			}
			if local.y > max_y {
				max_y = local.y;
			}
			candidates.push(local);
		}

		if max_y <= 0.0 || candidates.is_empty() {
			continue;
		}
		let cap: Vec<_> = candidates
			.into_iter()
			.filter(|point| max_y - point.y <= CAP_Y_TOLERANCE)
			.collect();
		if cap.is_empty() {
			continue;
		}
		let centroid = cap.iter().copied().sum::<Vec3>() / cap.len() as f32;
		if centroid.is_finite() {
			points[side] = Some(centroid);
		}
	}

	points
}

#[cfg(test)]
mod tests {
	use super::forearm_terminal_points;
	use anyhow::{anyhow, bail, Context, Result};
	use bevy::asset::RenderAssetUsages;
	use bevy::mesh::{
		skinning::SkinnedMeshInverseBindposes, Mesh, PrimitiveTopology, VertexAttributeValues,
	};
	use bevy::prelude::Mat4;
	use serde_json::Value;
	use std::path::Path;

	#[test]
	fn standard_and_full_assets_have_full_weight_terminal_caps() -> Result<()> {
		let assets = [
			(
				"../../../../../assets/characters/bodies/biped/humanoid_full_body.glb",
				[0.37539, 0.37579],
				[[-0.00224, -0.00511], [-0.00091, -0.00535]],
			),
			(
				"../../../../../assets/characters/bodies/biped/leron_biped_full_body.glb",
				[0.30889, 0.31241],
				[[0.00406, 0.01048], [-0.02246, 0.01318]],
			),
		];

		for (path, expected_y, expected_xz) in assets {
			let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
			let (mesh, bindposes, forearms) = load_glb_skin(&path)?;
			let actual = forearm_terminal_points(&mesh, &bindposes, forearms);
			for side in 0..2 {
				let point = actual[side]
					.with_context(|| format!("missing forearm cap for side {side} in {path:?}"))?;
				assert!(
					(point.y - expected_y[side]).abs() < 0.001,
					"{path:?} side {side}: {point:?}"
				);
				assert!(
					(point.x - expected_xz[side][0]).abs() < 0.001,
					"{path:?} side {side}: {point:?}"
				);
				assert!(
					(point.z - expected_xz[side][1]).abs() < 0.001,
					"{path:?} side {side}: {point:?}"
				);
			}
		}
		Ok(())
	}

	#[test]
	fn missing_geometry_and_zero_weight_forearms_return_none() -> Result<()> {
		let inverse_bindposes = SkinnedMeshInverseBindposes::from(vec![Mat4::IDENTITY]);
		let missing_positions =
			Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
				.with_inserted_attribute(
					Mesh::ATTRIBUTE_JOINT_INDEX,
					VertexAttributeValues::Uint16x4(vec![[0, 0, 0, 0]]),
				)
				.with_inserted_attribute(
					Mesh::ATTRIBUTE_JOINT_WEIGHT,
					vec![[1.0_f32, 0.0, 0.0, 0.0]],
				);
		assert_eq!(
			forearm_terminal_points(&missing_positions, &inverse_bindposes, [Some(0), None]),
			[None, None]
		);

		let zero_weight = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
			.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0_f32, 0.2, 0.0]])
			.with_inserted_attribute(
				Mesh::ATTRIBUTE_JOINT_INDEX,
				VertexAttributeValues::Uint16x4(vec![[0, 0, 0, 0]]),
			)
			.with_inserted_attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT, vec![[0.0_f32, 0.0, 0.0, 0.0]]);
		assert_eq!(
			forearm_terminal_points(&zero_weight, &inverse_bindposes, [Some(0), None]),
			[None, None]
		);
		Ok(())
	}

	fn load_glb_skin(
		path: &Path,
	) -> Result<(Mesh, SkinnedMeshInverseBindposes, [Option<usize>; 2])> {
		let bytes = std::fs::read(path).with_context(|| format!("read GLB {path:?}"))?;
		if !bytes.starts_with(b"glTF") {
			bail!("invalid GLB magic in {path:?}");
		}
		let declared_len = read_u32(&bytes, 8)? as usize;
		if declared_len != bytes.len() {
			bail!("GLB length mismatch in {path:?}");
		}

		let mut offset = 12;
		let mut document: Option<serde_json::Value> = None;
		let mut binary = None;
		while offset < bytes.len() {
			let chunk_len = read_u32(&bytes, offset)? as usize;
			let chunk_type = read_u32(&bytes, offset + 4)?;
			let start = offset.checked_add(8).context("GLB chunk overflow")?;
			let end = start.checked_add(chunk_len).context("GLB chunk overflow")?;
			let chunk = bytes.get(start..end).context("GLB chunk outside file")?;
			match chunk_type {
				0x4e4f534a => document = Some(serde_json::from_slice(chunk)?),
				0x004e4942 => binary = Some(chunk),
				_ => {}
			}
			offset = end;
		}
		let document = document.context("GLB has no JSON chunk")?;
		let binary = binary.context("GLB has no binary chunk")?;
		let mesh_index = document["nodes"]
			.as_array()
			.and_then(|nodes| nodes.iter().find(|node| node.get("mesh").is_some()))
			.and_then(|node| node["mesh"].as_u64())
			.context("GLB has no mesh node")? as usize;
		let skin_index = document["nodes"]
			.as_array()
			.and_then(|nodes| nodes.iter().find(|node| node.get("mesh").is_some()))
			.and_then(|node| node["skin"].as_u64())
			.context("GLB mesh node has no skin")? as usize;
		let primitive = &document["meshes"][mesh_index]["primitives"][0];
		let attributes = &primitive["attributes"];
		let positions = read_accessor_f32::<3>(
			&document,
			binary,
			attributes["POSITION"].as_u64().context("mesh has no POSITION")? as usize,
		)?;
		let joint_indices = read_accessor_u16x4(
			&document,
			binary,
			attributes["JOINTS_0"].as_u64().context("mesh has no JOINTS_0")? as usize,
		)?;
		let joint_weights = read_accessor_f32::<4>(
			&document,
			binary,
			attributes["WEIGHTS_0"].as_u64().context("mesh has no WEIGHTS_0")? as usize,
		)?;

		let skin = &document["skins"][skin_index];
		let joints_json = skin["joints"].as_array().context("skin has no joint list")?;
		let nodes = document["nodes"].as_array().context("GLB has no nodes")?;
		let names = joints_json
			.iter()
			.map(|joint| {
				let node_index = joint.as_u64().context("invalid skin joint index")? as usize;
				nodes
					.get(node_index)
					.and_then(|node| node["name"].as_str())
					.map(str::to_owned)
					.context("skin joint node has no name")
			})
			.collect::<Result<Vec<_>>>()?;
		let forearms = [
			names.iter().position(|name| name == "forearm.L"),
			names.iter().position(|name| name == "forearm.R"),
		];
		let bindpose_accessor = skin["inverseBindMatrices"]
			.as_u64()
			.context("skin has no inverseBindMatrices")? as usize;
		let matrices: Vec<Mat4> = read_accessor_f32::<16>(&document, binary, bindpose_accessor)?
			.into_iter()
			.map(|matrix| Mat4::from_cols_array(&matrix))
			.collect();
		let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
			.with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
			.with_inserted_attribute(
				Mesh::ATTRIBUTE_JOINT_INDEX,
				VertexAttributeValues::Uint16x4(joint_indices),
			)
			.with_inserted_attribute(
				Mesh::ATTRIBUTE_JOINT_WEIGHT,
				VertexAttributeValues::Float32x4(joint_weights),
			);
		Ok((mesh, SkinnedMeshInverseBindposes::from(matrices), forearms))
	}

	fn read_accessor_f32<const N: usize>(
		document: &Value,
		binary: &[u8],
		accessor_index: usize,
	) -> Result<Vec<[f32; N]>> {
		let (count, offset, stride, component_type) = accessor_layout(document, accessor_index, N)?;
		if component_type != 5126 {
			bail!("accessor {accessor_index} is not f32");
		}
		let mut result = Vec::with_capacity(count);
		for item in 0..count {
			let item_offset = offset + item * stride;
			let mut values = [0.0; N];
			for (component, value) in values.iter_mut().enumerate() {
				let start = item_offset + component * 4;
				let bytes = binary.get(start..start + 4).context("accessor outside BIN chunk")?;
				*value = f32::from_le_bytes(bytes.try_into().map_err(|_| anyhow!("invalid f32"))?);
			}
			result.push(values);
		}
		Ok(result)
	}

	fn read_accessor_u16x4(
		document: &Value,
		binary: &[u8],
		accessor_index: usize,
	) -> Result<Vec<[u16; 4]>> {
		let (count, offset, stride, component_type) = accessor_layout(document, accessor_index, 4)?;
		let size = match component_type {
			5121 => 1,
			5123 => 2,
			_ => bail!("accessor {accessor_index} is not an unsigned joint index"),
		};
		let mut result = Vec::with_capacity(count);
		for item in 0..count {
			let item_offset = offset + item * stride;
			let mut values = [0; 4];
			for (component, value) in values.iter_mut().enumerate() {
				let start = item_offset + component * size;
				let bytes =
					binary.get(start..start + size).context("accessor outside BIN chunk")?;
				*value = if size == 1 {
					u16::from(bytes[0])
				} else {
					u16::from_le_bytes(bytes.try_into().map_err(|_| anyhow!("invalid u16"))?)
				};
			}
			result.push(values);
		}
		Ok(result)
	}

	fn accessor_layout(
		document: &Value,
		accessor_index: usize,
		expected_components: usize,
	) -> Result<(usize, usize, usize, u64)> {
		let accessor = document["accessors"]
			.as_array()
			.and_then(|accessors| accessors.get(accessor_index))
			.context("accessor index out of range")?;
		let accessor_type = accessor["type"].as_str().context("accessor has no type")?;
		let components = match accessor_type {
			"SCALAR" => 1,
			"VEC2" => 2,
			"VEC3" => 3,
			"VEC4" => 4,
			"MAT4" => 16,
			_ => bail!("unsupported accessor type {accessor_type}"),
		};
		if components != expected_components {
			bail!("unexpected component count {components}");
		}
		let view_index =
			accessor["bufferView"].as_u64().context("accessor has no bufferView")? as usize;
		let view = document["bufferViews"]
			.as_array()
			.and_then(|views| views.get(view_index))
			.context("bufferView index out of range")?;
		let count = accessor["count"].as_u64().context("accessor has no count")? as usize;
		let component_type =
			accessor["componentType"].as_u64().context("accessor has no componentType")?;
		let component_size = match component_type {
			5121 => 1,
			5123 => 2,
			5126 => 4,
			_ => bail!("unsupported component type {component_type}"),
		};
		let element_size = component_size * components;
		let view_offset = view["byteOffset"].as_u64().map_or(0, |value| value as usize);
		let accessor_offset = accessor["byteOffset"].as_u64().map_or(0, |value| value as usize);
		let offset =
			view_offset.checked_add(accessor_offset).context("accessor offset overflow")?;
		let stride = view["byteStride"].as_u64().map_or(element_size, |value| value as usize);
		if stride < element_size {
			bail!("accessor stride is too short");
		}
		Ok((count, offset, stride, component_type))
	}

	fn read_u32(bytes: &[u8], offset: usize) -> Result<u32> {
		let end = offset.checked_add(4).context("integer offset overflow")?;
		let value = bytes.get(offset..end).context("integer outside file")?;
		Ok(u32::from_le_bytes(value.try_into().map_err(|_| anyhow!("invalid u32"))?))
	}
}
