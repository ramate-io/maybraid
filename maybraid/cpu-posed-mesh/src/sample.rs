//! CPU linear-blend sampler / vertex visitor.

use bevy::ecs::entity::Entity;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::mesh::skinning::{InfluenceIterator, SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::VertexAttributeValues;
use bevy::prelude::{GlobalTransform, Mesh};

use crate::palette::JointPalette;
use crate::policy::{DeadVertex, MissingInfluence, NormalPolicy, PosePolicy, PoseSpace};

/// One posed vertex after policy resolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PosedVertex {
	pub index: usize,
	pub position: Vec3,
	pub normal: Option<Vec3>,
}

/// Visitor over posed vertices. Closures implement this automatically.
pub trait PosedVertexVisitor {
	fn visit(&mut self, vertex: PosedVertex);
}

impl<F: FnMut(PosedVertex)> PosedVertexVisitor for F {
	fn visit(&mut self, vertex: PosedVertex) {
		self(vertex);
	}
}

/// Sampled positions (and optional normals) after [`PosePolicy`] application.
#[derive(Clone, Debug, PartialEq)]
pub struct CpuPosedMesh {
	pub positions: Vec<Vec3>,
	pub normals: Vec<Vec3>,
}

impl CpuPosedMesh {
	pub fn aabb(&self) -> Option<Aabb3d> {
		aabb_of(self.positions.iter().copied())
	}
}

impl PosePolicy {
	pub fn palette(
		self,
		skin: &SkinnedMesh,
		bindposes: &SkinnedMeshInverseBindposes,
		world_of: impl FnMut(Entity) -> Option<GlobalTransform>,
	) -> JointPalette {
		JointPalette::from_skin_with(skin, bindposes, self.collapsed_joint, world_of)
	}

	/// Visit every vertex that survives [`DeadVertex`] (AABB omits dead verts).
	pub fn visit_vertices(
		self,
		mesh: &Mesh,
		palette: &JointPalette,
		visitor: impl PosedVertexVisitor,
	) -> Option<()> {
		visit_posed_vertices(mesh, palette, self, visitor)
	}

	pub fn visit_skinned(
		self,
		mesh: &Mesh,
		skin: &SkinnedMesh,
		bindposes: &SkinnedMeshInverseBindposes,
		world_of: impl FnMut(Entity) -> Option<GlobalTransform>,
		visitor: impl PosedVertexVisitor,
	) -> Option<()> {
		let palette = self.palette(skin, bindposes, world_of);
		self.visit_vertices(mesh, &palette, visitor)
	}

	pub fn sample(self, mesh: &Mesh, palette: &JointPalette) -> Option<CpuPosedMesh> {
		let mut positions = Vec::new();
		let mut normals = Vec::new();
		self.visit_vertices(mesh, palette, |vertex: PosedVertex| {
			positions.push(vertex.position);
			if let Some(normal) = vertex.normal {
				normals.push(normal);
			}
		})?;
		Some(CpuPosedMesh { positions, normals })
	}

	pub fn sample_skinned(
		self,
		mesh: &Mesh,
		skin: &SkinnedMesh,
		bindposes: &SkinnedMeshInverseBindposes,
		world_of: impl FnMut(Entity) -> Option<GlobalTransform>,
	) -> Option<CpuPosedMesh> {
		let palette = self.palette(skin, bindposes, world_of);
		self.sample(mesh, &palette)
	}

	pub fn aabb(self, mesh: &Mesh, palette: &JointPalette) -> Option<Aabb3d> {
		let mut min = Vec3::splat(f32::MAX);
		let mut max = Vec3::splat(f32::MIN);
		self.visit_vertices(mesh, palette, |vertex: PosedVertex| {
			min = min.min(vertex.position);
			max = max.max(vertex.position);
		})?;
		(min.cmple(max).all()).then(|| Aabb3d { min: min.into(), max: max.into() })
	}

	pub fn aabb_skinned(
		self,
		mesh: &Mesh,
		skin: &SkinnedMesh,
		bindposes: &SkinnedMeshInverseBindposes,
		world_of: impl FnMut(Entity) -> Option<GlobalTransform>,
	) -> Option<Aabb3d> {
		let palette = self.palette(skin, bindposes, world_of);
		self.aabb(mesh, &palette)
	}
}

fn visit_posed_vertices(
	mesh: &Mesh,
	palette: &JointPalette,
	policy: PosePolicy,
	mut visitor: impl PosedVertexVisitor,
) -> Option<()> {
	let Some(VertexAttributeValues::Float32x3(positions)) =
		mesh.attribute(Mesh::ATTRIBUTE_POSITION)
	else {
		return None;
	};
	let want_normals = matches!(policy.normals, NormalPolicy::Skin { .. });
	let normals_src = if want_normals {
		mesh.attribute(Mesh::ATTRIBUTE_NORMAL).and_then(|attr| match attr {
			VertexAttributeValues::Float32x3(n) => Some(n),
			_ => None,
		})
	} else {
		None
	};
	let influences = InfluenceIterator::new(mesh).ok()?;

	let mut skinned_pos = vec![Vec3::ZERO; positions.len()];
	let mut skinned_nrm = vec![Vec3::ZERO; positions.len()];
	let mut weights = vec![0.0_f32; positions.len()];
	let mut live = vec![0.0_f32; positions.len()];

	for influence in influences {
		let slot = palette.get(influence.joint_index.0 as usize);
		let Some(slot) = slot else {
			match policy.missing_influence {
				MissingInfluence::Skip => continue,
			}
		};
		let Some(&position) = positions.get(influence.vertex_index) else {
			continue;
		};
		let w = influence.joint_weight;
		skinned_pos[influence.vertex_index] +=
			slot.skin.transform_point3(Vec3::from_array(position)) * w;
		if let Some(normals) = normals_src {
			if let Some(&normal) = normals.get(influence.vertex_index) {
				skinned_nrm[influence.vertex_index] +=
					slot.skin.transform_vector3(Vec3::from_array(normal)) * w;
			}
		}
		weights[influence.vertex_index] += w;
		if !slot.collapsed {
			live[influence.vertex_index] += w;
		}
	}

	for i in 0..positions.len() {
		let dead = policy.dead_vertex.is_dead(live[i], weights[i]);
		let bind = Vec3::from_array(positions[i]);
		let world_pos = if dead {
			match policy.dead_vertex {
				DeadVertex::Omit { .. } => continue,
				DeadVertex::RigidFallback { mesh_world, .. } => mesh_world.transform_point3(bind),
			}
		} else {
			skinned_pos[i] / weights[i]
		};
		let world_normal = normals_src.map(|normals| {
			let bind_n = Vec3::from_array(normals[i]);
			let n = if dead {
				match policy.dead_vertex {
					DeadVertex::Omit { .. } => bind_n,
					DeadVertex::RigidFallback { mesh_world, .. } => {
						mesh_world.transform_vector3(bind_n)
					}
				}
			} else {
				skinned_nrm[i] / weights[i]
			};
			normalize_or(n, policy.normals)
		});
		let (position, normal) = apply_space(policy.space, world_pos, world_normal, policy.normals);
		visitor.visit(PosedVertex { index: i, position, normal });
	}
	Some(())
}

fn normalize_or(n: Vec3, policy: NormalPolicy) -> Vec3 {
	match policy {
		NormalPolicy::Skip => n,
		NormalPolicy::Skin { degenerate } => {
			if n.length_squared() > 1e-8 {
				n.normalize()
			} else {
				degenerate
			}
		}
	}
}

fn apply_space(
	space: PoseSpace,
	world_pos: Vec3,
	world_normal: Option<Vec3>,
	normals: NormalPolicy,
) -> (Vec3, Option<Vec3>) {
	match space {
		PoseSpace::World => (world_pos, world_normal),
		PoseSpace::Root { inverse } => {
			let position = inverse.transform_point3(world_pos);
			let normal = world_normal.map(|n| {
				let transformed = inverse.transform_vector3(n);
				normalize_or(transformed, normals)
			});
			(position, normal)
		}
	}
}

fn aabb_of(points: impl Iterator<Item = Vec3>) -> Option<Aabb3d> {
	let mut min = Vec3::splat(f32::MAX);
	let mut max = Vec3::splat(f32::MIN);
	let mut any = false;
	for point in points {
		any = true;
		min = min.min(point);
		max = max.max(point);
	}
	(any && min.cmple(max).all()).then(|| Aabb3d { min: min.into(), max: max.into() })
}
