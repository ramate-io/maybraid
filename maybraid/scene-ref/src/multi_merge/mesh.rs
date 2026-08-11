//! Bake per-entity + per-part transforms and concatenate meshes.

use bevy::mesh::{Mesh, MeshVertexAttributeId, VertexAttributeValues};
use bevy::platform::collections::HashMap;
use bevy::prelude::{Assets, ChildOf, Entity, Mesh3d, Transform, World};
use bevy::world_serialization::WorldAsset;

/// Collect every `Mesh3d` in `source`, bake `part_transform * entity_transform`,
/// then merge into one mesh.
///
/// Attributes not shared by every source mesh are dropped so [`Mesh::merge`] stays
/// vertex-count consistent. Materials are ignored.
pub(crate) fn merge_world_asset_meshes(
	source: &WorldAsset,
	part_transform: Transform,
	meshes: &Assets<Mesh>,
) -> Option<Mesh> {
	let mut prepared = Vec::new();
	let mut saw_mesh3d = false;

	for entity_ref in source.world.iter_entities() {
		let Some(mesh3d) = entity_ref.get::<Mesh3d>() else {
			continue;
		};
		saw_mesh3d = true;
		// Same mesh asset may appear on multiple entities with different transforms;
		// each instance must be baked separately. Missing mesh bytes → retry later.
		let mesh = meshes.get(&mesh3d.0)?;
		let entity_tf = entity_transform_in_world(&source.world, entity_ref.id());
		let baked = part_transform.mul_transform(entity_tf);
		prepared.push(bake_mesh(mesh, baked));
	}

	if !saw_mesh3d {
		return None;
	}
	merge_meshes(prepared)
}

/// Merge already-baked meshes (tests / callers that skip WorldAsset walk).
pub(crate) fn merge_meshes(mut prepared: Vec<Mesh>) -> Option<Mesh> {
	if prepared.is_empty() {
		return None;
	}

	retain_common_attributes(&mut prepared);

	let mut iter = prepared.into_iter();
	let mut merged = iter.next()?;
	for other in iter {
		merged.merge(&other).ok()?;
	}
	Some(merged)
}

fn bake_mesh(mesh: &Mesh, transform: Transform) -> Mesh {
	let mut out = mesh.clone();
	out.transform_by(transform);
	if transform.scale.x * transform.scale.y * transform.scale.z < 0.0 {
		let _ = out.invert_winding();
	}
	out
}

/// Local-to-scene-root transform by walking [`ChildOf`] + [`Transform`].
fn entity_transform_in_world(world: &World, entity: Entity) -> Transform {
	let mut transform = world.get::<Transform>(entity).copied().unwrap_or(Transform::IDENTITY);
	let mut current = entity;
	while let Some(child_of) = world.get::<ChildOf>(current) {
		current = child_of.parent();
		if let Some(parent_tf) = world.get::<Transform>(current) {
			transform = parent_tf.mul_transform(transform);
		}
	}
	transform
}

/// Drop attributes that are not present (same id + format) on every mesh.
///
/// Format identity uses [`std::mem::discriminant`] on [`VertexAttributeValues`] —
/// this is merge-time compatibility only, not part of the [`MultiSceneMerge`] cache key.
fn retain_common_attributes(meshes: &mut [Mesh]) {
	if meshes.is_empty() {
		return;
	}

	let mut common: Option<HashMap<MeshVertexAttributeId, std::mem::Discriminant<VertexAttributeValues>>> =
		None;
	for mesh in meshes.iter() {
		let mut attrs = HashMap::default();
		for (attr, values) in mesh.attributes() {
			attrs.insert(attr.id, std::mem::discriminant(values));
		}
		common = Some(match common.take() {
			None => attrs,
			Some(prev) => prev
				.into_iter()
				.filter(|(id, key)| attrs.get(id) == Some(key))
				.collect(),
		});
	}

	let Some(common) = common else {
		return;
	};

	for mesh in meshes.iter_mut() {
		let drop: Vec<_> = mesh
			.attributes()
			.filter(|(attr, values)| {
				common
					.get(&attr.id)
					.is_none_or(|key| *key != std::mem::discriminant(*values))
			})
			.map(|(attr, _)| attr.id)
			.collect();
		for id in drop {
			mesh.remove_attribute(id);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::asset::RenderAssetUsages;
	use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};

	fn triangle(offset: [f32; 3]) -> Mesh {
		let mut mesh = Mesh::new(
			PrimitiveTopology::TriangleList,
			RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![
				[offset[0], offset[1], offset[2]],
				[offset[0] + 1.0, offset[1], offset[2]],
				[offset[0], offset[1] + 1.0, offset[2]],
			],
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_NORMAL,
			vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
		);
		mesh.insert_indices(Indices::U32(vec![0, 1, 2]));
		mesh
	}

	#[test]
	fn merge_concatenates_vertices_and_rebases_indices() -> anyhow::Result<()> {
		let merged = merge_meshes(vec![triangle([0.0, 0.0, 0.0]), triangle([10.0, 0.0, 0.0])])
			.expect("merge");
		assert_eq!(merged.count_vertices(), 6);
		match merged.indices() {
			Some(Indices::U32(idx)) => assert_eq!(idx.as_slice(), &[0, 1, 2, 3, 4, 5]),
			other => anyhow::bail!("unexpected indices: {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn bake_applies_translation() -> anyhow::Result<()> {
		let baked = bake_mesh(&triangle([0.0, 0.0, 0.0]), Transform::from_xyz(2.0, 0.0, 0.0));
		let Some(VertexAttributeValues::Float32x3(positions)) =
			baked.attribute(Mesh::ATTRIBUTE_POSITION)
		else {
			anyhow::bail!("expected positions");
		};
		assert!((positions[0][0] - 2.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn retain_drops_unshared_attributes() -> anyhow::Result<()> {
		let mut a = triangle([0.0, 0.0, 0.0]);
		a.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
		let b = triangle([1.0, 0.0, 0.0]);
		let merged = merge_meshes(vec![a, b]).expect("merge");
		assert!(merged.attribute(Mesh::ATTRIBUTE_UV_0).is_none());
		assert!(merged.attribute(Mesh::ATTRIBUTE_POSITION).is_some());
		Ok(())
	}
}
