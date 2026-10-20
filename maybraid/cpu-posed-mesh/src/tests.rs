//! Bit-exact equivalence with the previous AABB and mesh-export loops.

use bevy::asset::{Assets, RenderAssetUsages};
use bevy::ecs::entity::Entity;
use bevy::math::{Affine3A, Mat4, Vec3};
use bevy::mesh::skinning::{InfluenceIterator, SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};
use bevy::prelude::{GlobalTransform, Mesh, Transform, World};

use crate::{
	CollapsedJoint, CpuPosedMesh, DeadVertex, JointPalette, JointSlot, MissingInfluence,
	NormalPolicy, PosePolicy, PoseSpace, WindingPolicy,
};

struct SkinFixture {
	mesh: Mesh,
	skin: SkinnedMesh,
	bindposes: Assets<SkinnedMeshInverseBindposes>,
	globals: Vec<(Entity, GlobalTransform)>,
}

fn lookup<'a>(
	globals: &'a [(Entity, GlobalTransform)],
) -> impl Fn(Entity) -> Option<GlobalTransform> + 'a {
	|entity| globals.iter().find(|(id, _)| *id == entity).map(|(_, g)| *g)
}

fn fixture_capture_case() -> SkinFixture {
	// Matches `screen-capture` `skinned_extents_follow_joints_not_the_mesh_entity`.
	let mut world = World::new();
	let posed = world.spawn_empty().id();
	let collapsed = world.spawn_empty().id();
	let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
		.with_inserted_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [3.0, 0.0, 0.0]],
		)
		.with_inserted_attribute(
			Mesh::ATTRIBUTE_NORMAL,
			vec![[0.0, 0.0, 1.0], [0.0, 0.0, 1.0], [0.0, 0.0, 1.0]],
		)
		.with_inserted_attribute(
			Mesh::ATTRIBUTE_JOINT_INDEX,
			VertexAttributeValues::Uint16x4(vec![[0, 0, 0, 0], [0, 0, 0, 0], [1, 0, 0, 0]]),
		)
		.with_inserted_attribute(
			Mesh::ATTRIBUTE_JOINT_WEIGHT,
			vec![[1.0_f32, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]],
		);
	let mut bindposes = Assets::<SkinnedMeshInverseBindposes>::default();
	let inverse =
		bindposes.add(SkinnedMeshInverseBindposes::from(vec![Mat4::IDENTITY, Mat4::IDENTITY]));
	SkinFixture {
		mesh,
		skin: SkinnedMesh { inverse_bindposes: inverse, joints: vec![posed, collapsed] },
		bindposes,
		globals: vec![
			(posed, GlobalTransform::from(Transform::from_xyz(5.0, 0.0, 0.0))),
			(
				collapsed,
				GlobalTransform::from(Transform::from_xyz(-50.0, 0.0, 0.0).with_scale(Vec3::ZERO)),
			),
		],
	}
}

fn fixture_missing_and_blend() -> SkinFixture {
	let mut world = World::new();
	let live = world.spawn_empty().id();
	let missing = world.spawn_empty().id();
	let mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default())
		.with_inserted_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[0.0, 0.0, 0.0], [2.0, 0.0, 0.0], [0.0, 2.0, 0.0]],
		)
		.with_inserted_attribute(
			Mesh::ATTRIBUTE_NORMAL,
			vec![[0.0, 1.0, 0.0], [0.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
		)
		.with_inserted_attribute(
			Mesh::ATTRIBUTE_JOINT_INDEX,
			VertexAttributeValues::Uint16x4(vec![[0, 0, 0, 0], [0, 1, 0, 0], [2, 0, 0, 0]]),
		)
		.with_inserted_attribute(
			Mesh::ATTRIBUTE_JOINT_WEIGHT,
			vec![[1.0_f32, 0.0, 0.0, 0.0], [0.5, 0.5, 0.0, 0.0], [1.0, 0.0, 0.0, 0.0]],
		);
	let mut bindposes = Assets::<SkinnedMeshInverseBindposes>::default();
	let inverse =
		bindposes.add(SkinnedMeshInverseBindposes::from(vec![Mat4::IDENTITY, Mat4::IDENTITY]));
	SkinFixture {
		mesh,
		skin: SkinnedMesh { inverse_bindposes: inverse, joints: vec![live, missing] },
		bindposes,
		globals: vec![(
			live,
			GlobalTransform::from(Transform {
				translation: Vec3::new(0.0, 4.0, 0.0),
				rotation: bevy::math::Quat::from_rotation_x(std::f32::consts::FRAC_PI_2),
				scale: Vec3::ONE,
			}),
		)],
	}
}

/// Previous `screen-capture::skinned_world_aabb` accumulation (world points).
fn legacy_capture_points(
	mesh: &Mesh,
	skin: &SkinnedMesh,
	bindposes: &SkinnedMeshInverseBindposes,
	globals: &[(Entity, GlobalTransform)],
) -> Option<Vec<Vec3>> {
	let Some(VertexAttributeValues::Float32x3(positions)) =
		mesh.attribute(Mesh::ATTRIBUTE_POSITION)
	else {
		return None;
	};
	let influences = InfluenceIterator::new(mesh).ok()?;
	let joints: Vec<Option<(Mat4, bool)>> = skin
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
	let mut skinned = vec![Vec3::ZERO; positions.len()];
	let mut weights = vec![0.0_f32; positions.len()];
	let mut live = vec![0.0_f32; positions.len()];
	for influence in influences {
		let Some(Some((joint, collapsed))) = joints.get(influence.joint_index.0 as usize) else {
			continue;
		};
		let Some(&position) = positions.get(influence.vertex_index) else {
			continue;
		};
		skinned[influence.vertex_index] +=
			joint.transform_point3(Vec3::from_array(position)) * influence.joint_weight;
		weights[influence.vertex_index] += influence.joint_weight;
		if !collapsed {
			live[influence.vertex_index] += influence.joint_weight;
		}
	}
	let mut out = Vec::new();
	for ((point, weight), live) in skinned.iter().zip(&weights).zip(&live) {
		if *live <= 1e-4 {
			continue;
		}
		out.push(*point / *weight);
	}
	Some(out)
}

/// Previous `character-mesh-export::skin_mesh_world`.
fn legacy_mesh_bake_world(
	mesh: &Mesh,
	skin: &SkinnedMesh,
	bindposes: &SkinnedMeshInverseBindposes,
	globals: &[(Entity, GlobalTransform)],
	mesh_world: Affine3A,
) -> Option<CpuPosedMesh> {
	let Some(VertexAttributeValues::Float32x3(positions)) =
		mesh.attribute(Mesh::ATTRIBUTE_POSITION)
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
	Some(CpuPosedMesh { positions: out_pos, normals: out_nrm })
}

fn legacy_root(surface: &CpuPosedMesh, inverse: Affine3A) -> CpuPosedMesh {
	CpuPosedMesh {
		positions: surface.positions.iter().map(|p| inverse.transform_point3(*p)).collect(),
		normals: surface
			.normals
			.iter()
			.map(|n| inverse.transform_vector3(*n).normalize())
			.collect(),
	}
}

fn legacy_align_winding(positions: &[Vec3], normals: &[Vec3], faces: &mut [[u32; 3]]) {
	if normals.len() != positions.len() {
		return;
	}
	for face in faces.iter_mut() {
		let [ia, ib, ic] = *face;
		let (Some(&pa), Some(&pb), Some(&pc)) =
			(positions.get(ia as usize), positions.get(ib as usize), positions.get(ic as usize))
		else {
			continue;
		};
		let (Some(&na), Some(&nb), Some(&nc)) =
			(normals.get(ia as usize), normals.get(ib as usize), normals.get(ic as usize))
		else {
			continue;
		};
		let geo = (pb - pa).cross(pc - pa);
		if geo.dot(na + nb + nc) < 0.0 {
			*face = [ia, ic, ib];
		}
	}
}

fn bits(v: Vec3) -> [u32; 3] {
	[v.x.to_bits(), v.y.to_bits(), v.z.to_bits()]
}

fn assert_vec3s_eq(a: &[Vec3], b: &[Vec3]) {
	assert_eq!(a.len(), b.len(), "len");
	for (i, (left, right)) in a.iter().zip(b).enumerate() {
		assert_eq!(bits(*left), bits(*right), "vertex {i}: {left:?} vs {right:?}");
	}
}

#[test]
fn to_matrix_matches_affine_cast() {
	let world = GlobalTransform::from(
		Transform::from_xyz(1.25, -0.5, 3.0)
			.with_scale(Vec3::new(0.5, 2.0, 1.0))
			.with_rotation(bevy::math::Quat::from_rotation_y(0.4)),
	);
	assert_eq!(world.to_matrix(), Mat4::from(world.affine()));
}

#[test]
fn capture_aabb_matches_legacy_world_points() {
	let fix = fixture_capture_case();
	let bind = fix.bindposes.get(&fix.skin.inverse_bindposes).expect("bind");
	let legacy = legacy_capture_points(&fix.mesh, &fix.skin, bind, &fix.globals).expect("legacy");
	let sampled = PosePolicy::capture_aabb()
		.sample_skinned(&fix.mesh, &fix.skin, bind, lookup(&fix.globals))
		.expect("sample");
	assert_vec3s_eq(&sampled.positions, &legacy);
	assert!(sampled.normals.is_empty());

	let aabb = PosePolicy::capture_aabb()
		.aabb_skinned(&fix.mesh, &fix.skin, bind, lookup(&fix.globals))
		.expect("aabb");
	assert_eq!(aabb.min.x.to_bits(), 5.0_f32.to_bits());
	assert_eq!(aabb.max.x.to_bits(), 6.0_f32.to_bits());
}

#[test]
fn mesh_bake_world_matches_legacy_including_collapsed_fallback() {
	let fix = fixture_capture_case();
	let bind = fix.bindposes.get(&fix.skin.inverse_bindposes).expect("bind");
	let mesh_world = Affine3A::from_translation(Vec3::new(100.0, 0.0, 0.0));
	let legacy = legacy_mesh_bake_world(&fix.mesh, &fix.skin, bind, &fix.globals, mesh_world)
		.expect("legacy");
	let sampled = PosePolicy::mesh_bake_world(mesh_world)
		.sample_skinned(&fix.mesh, &fix.skin, bind, lookup(&fix.globals))
		.expect("sample");
	assert_vec3s_eq(&sampled.positions, &legacy.positions);
	assert_vec3s_eq(&sampled.normals, &legacy.normals);
	// Collapsed vertex falls back to the mesh entity, not the joint at x = -50.
	assert_eq!(sampled.positions[2], Vec3::new(103.0, 0.0, 0.0));
}

#[test]
fn mesh_bake_root_space_matches_legacy_two_step() {
	let fix = fixture_missing_and_blend();
	let bind = fix.bindposes.get(&fix.skin.inverse_bindposes).expect("bind");
	let mesh_world = Affine3A::from_translation(Vec3::new(1.0, 2.0, 3.0));
	let root_inverse = Affine3A::from_translation(Vec3::new(-8.0, 0.5, 0.0));
	let legacy_world = legacy_mesh_bake_world(&fix.mesh, &fix.skin, bind, &fix.globals, mesh_world)
		.expect("legacy");
	let legacy = legacy_root(&legacy_world, root_inverse);
	let sampled = PosePolicy::mesh_bake(mesh_world, root_inverse)
		.sample_skinned(&fix.mesh, &fix.skin, bind, lookup(&fix.globals))
		.expect("sample");
	assert_vec3s_eq(&sampled.positions, &legacy.positions);
	assert_vec3s_eq(&sampled.normals, &legacy.normals);
}

#[test]
fn missing_influence_is_skip_and_dead_vertex_follows_policy() {
	let fix = fixture_missing_and_blend();
	let bind = fix.bindposes.get(&fix.skin.inverse_bindposes).expect("bind");
	let mesh_world = Affine3A::IDENTITY;
	let aabb = PosePolicy::capture_aabb()
		.sample_skinned(&fix.mesh, &fix.skin, bind, lookup(&fix.globals))
		.expect("aabb sample");
	// Vertex 2 indexes joint 2, which is missing — omitted from AABB.
	assert_eq!(aabb.positions.len(), 2);

	let baked = PosePolicy::mesh_bake_world(mesh_world)
		.sample_skinned(&fix.mesh, &fix.skin, bind, lookup(&fix.globals))
		.expect("bake");
	assert_eq!(baked.positions.len(), 3);
	assert_eq!(bits(baked.positions[2]), bits(Vec3::new(0.0, 2.0, 0.0)));
	assert_eq!(PosePolicy::capture_aabb().missing_influence, MissingInfluence::Skip);
}

#[test]
fn collapsed_joint_is_dead_below_default_scale() {
	let world = GlobalTransform::from(Transform::from_scale(Vec3::splat(5e-4)));
	let slot = JointSlot::from_world(world, Mat4::IDENTITY, CollapsedJoint::DEFAULT);
	assert!(slot.collapsed);
	let world = GlobalTransform::from(Transform::from_scale(Vec3::splat(2e-3)));
	let slot = JointSlot::from_world(world, Mat4::IDENTITY, CollapsedJoint::DEFAULT);
	assert!(!slot.collapsed);
}

#[test]
fn mirrored_winding_flips_when_geometric_normal_disagrees() {
	let positions = [Vec3::ZERO, Vec3::X, Vec3::Y];
	let normals = [Vec3::NEG_Z, Vec3::NEG_Z, Vec3::NEG_Z];
	let mut faces = [[0_u32, 1, 2]];
	let mut legacy = faces;
	WindingPolicy::AlignWithVertexNormals.apply(&positions, &normals, &mut faces);
	legacy_align_winding(&positions, &normals, &mut legacy);
	assert_eq!(faces, [[0, 2, 1]]);
	assert_eq!(faces, legacy);
	WindingPolicy::Keep.apply(&positions, &normals, &mut faces);
	assert_eq!(faces, [[0, 2, 1]]);
}

#[test]
fn keep_winding_leaves_mirrored_indices() {
	let positions = [Vec3::ZERO, Vec3::X, Vec3::Y];
	let normals = [Vec3::Z, Vec3::Z, Vec3::Z];
	let mut faces = [[0_u32, 2, 1]];
	WindingPolicy::Keep.apply(&positions, &normals, &mut faces);
	assert_eq!(faces, [[0, 2, 1]]);
	WindingPolicy::AlignWithVertexNormals.apply(&positions, &normals, &mut faces);
	assert_eq!(faces, [[0, 1, 2]]);
}

#[test]
fn policies_are_explicit_on_the_shared_types() {
	let policy = PosePolicy::capture_aabb();
	assert_eq!(policy.missing_influence, MissingInfluence::Skip);
	assert_eq!(policy.collapsed_joint, CollapsedJoint::DEFAULT);
	assert_eq!(policy.dead_vertex, DeadVertex::CAPTURE_OMIT);
	assert_eq!(policy.space, PoseSpace::World);
	assert_eq!(policy.normals, NormalPolicy::Skip);

	let mesh_world = Affine3A::IDENTITY;
	let inverse = Affine3A::from_translation(Vec3::X);
	let bake = PosePolicy::mesh_bake(mesh_world, inverse);
	assert_eq!(bake.dead_vertex, DeadVertex::mesh_bake(mesh_world));
	assert_eq!(bake.space, PoseSpace::Root { inverse });
	assert_eq!(bake.normals, NormalPolicy::MESH_BAKE);
	assert_eq!(WindingPolicy::AlignWithVertexNormals, WindingPolicy::default());
}

#[test]
fn joint_palette_skips_unresolved_worlds() {
	let fix = fixture_missing_and_blend();
	let bind = fix.bindposes.get(&fix.skin.inverse_bindposes).expect("bind");
	let palette = JointPalette::from_skin(&fix.skin, bind, lookup(&fix.globals));
	assert_eq!(palette.len(), 2);
	assert!(palette.get(0).is_some());
	assert!(palette.get(1).is_none());
}

#[test]
fn triangle_list_indices_survive_winding_policy() {
	let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::default());
	mesh.insert_attribute(
		Mesh::ATTRIBUTE_POSITION,
		vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
	);
	mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 0.0, -1.0]; 3]);
	mesh.insert_indices(Indices::U32(vec![0, 1, 2]));
	let mut faces = [[0_u32, 1, 2]];
	let positions: Vec<Vec3> = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
		Some(VertexAttributeValues::Float32x3(p)) => {
			p.iter().map(|a| Vec3::from_array(*a)).collect()
		}
		_ => Vec::new(),
	};
	let normals: Vec<Vec3> = match mesh.attribute(Mesh::ATTRIBUTE_NORMAL) {
		Some(VertexAttributeValues::Float32x3(n)) => {
			n.iter().map(|a| Vec3::from_array(*a)).collect()
		}
		_ => Vec::new(),
	};
	WindingPolicy::AlignWithVertexNormals.apply(&positions, &normals, &mut faces);
	assert_eq!(faces, [[0, 2, 1]]);
}
