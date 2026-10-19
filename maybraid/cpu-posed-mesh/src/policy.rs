//! Explicit policies for CPU posed-mesh sampling.

use bevy::math::{Affine3A, Vec3};

/// What to do when an influence names a joint the palette does not resolve.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MissingInfluence {
	/// Drop the influence (no position, weight, or live contribution).
	#[default]
	Skip,
}

/// How a joint whose world scale is ~zero participates in the blend.
///
/// The joint matrix is still applied (so a barely-collapsed bone can tug a
/// blend), but it does not count as live. Vertices whose only influences are
/// collapsed then follow [`DeadVertex`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CollapsedJoint {
	/// World-scale max-element below `max_scale` is dead.
	DeadBelow { max_scale: f32 },
}

impl CollapsedJoint {
	/// Threshold shared by screen-capture AABB and mesh-export bake (`1e-3`).
	pub const DEFAULT: Self = Self::DeadBelow { max_scale: 1e-3 };
}

impl Default for CollapsedJoint {
	fn default() -> Self {
		Self::DEFAULT
	}
}

/// Vertices whose accumulated live weight is ~zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DeadVertex {
	/// Drop from the sample (screen-capture AABB — degenerate triangles).
	Omit { live_epsilon: f32 },
	/// Rigid-transform the bind-pose vertex by the mesh entity (mesh-export).
	RigidFallback { mesh_world: Affine3A, live_epsilon: f32, weight_epsilon: f32 },
}

impl DeadVertex {
	/// `screen-capture::skinned_world_aabb` omit threshold (`live <= 1e-4`).
	pub const CAPTURE_OMIT: Self = Self::Omit { live_epsilon: 1e-4 };

	/// `character-mesh-export::skin_mesh_world` rigid fallback.
	pub fn mesh_bake(mesh_world: Affine3A) -> Self {
		Self::RigidFallback { mesh_world, live_epsilon: 1e-4, weight_epsilon: 1e-6 }
	}

	pub fn is_dead(self, live: f32, weight: f32) -> bool {
		match self {
			Self::Omit { live_epsilon } => live <= live_epsilon,
			Self::RigidFallback { live_epsilon, weight_epsilon, .. } => {
				live <= live_epsilon || weight <= weight_epsilon
			}
		}
	}
}

/// Output space after world-space linear-blend skinning.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PoseSpace {
	/// Joint-palette world positions / normals (screen-capture AABB).
	World,
	/// Root-local: apply `inverse` per vertex after world skinning (mesh-export).
	///
	/// Positions use `inverse.transform_point3`. Normals are world-normalized
	/// first, then `inverse.transform_vector3` and re-normalized — the same
	/// two-step as the previous exporter.
	Root { inverse: Affine3A },
}

/// Whether (and how) to skin vertex normals.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum NormalPolicy {
	/// Positions only (screen-capture AABB).
	Skip,
	/// Skin with `transform_vector3`, then normalize; `degenerate` if ~zero.
	Skin { degenerate: Vec3 },
}

impl NormalPolicy {
	/// Mesh-export fallback when a skinned normal is degenerate (`Vec3::Y`).
	pub const MESH_BAKE: Self = Self::Skin { degenerate: Vec3::Y };
}

/// Policy bundle consumed by both AABB extents and mesh baking.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PosePolicy {
	pub missing_influence: MissingInfluence,
	pub collapsed_joint: CollapsedJoint,
	pub dead_vertex: DeadVertex,
	pub space: PoseSpace,
	pub normals: NormalPolicy,
}

impl PosePolicy {
	/// `screen-capture::skinned_world_aabb`: skip missing influences, treat
	/// collapsed joints as dead, omit dead vertices, world space, no normals.
	pub fn capture_aabb() -> Self {
		Self {
			missing_influence: MissingInfluence::Skip,
			collapsed_joint: CollapsedJoint::DEFAULT,
			dead_vertex: DeadVertex::CAPTURE_OMIT,
			space: PoseSpace::World,
			normals: NormalPolicy::Skip,
		}
	}

	/// World-space surface of the previous `skin_mesh_world` helper.
	pub fn mesh_bake_world(mesh_world: Affine3A) -> Self {
		Self {
			missing_influence: MissingInfluence::Skip,
			collapsed_joint: CollapsedJoint::DEFAULT,
			dead_vertex: DeadVertex::mesh_bake(mesh_world),
			space: PoseSpace::World,
			normals: NormalPolicy::MESH_BAKE,
		}
	}

	/// Mesh-export bake into character-root space.
	pub fn mesh_bake(mesh_world: Affine3A, root_inverse: Affine3A) -> Self {
		Self {
			missing_influence: MissingInfluence::Skip,
			collapsed_joint: CollapsedJoint::DEFAULT,
			dead_vertex: DeadVertex::mesh_bake(mesh_world),
			space: PoseSpace::Root { inverse: root_inverse },
			normals: NormalPolicy::MESH_BAKE,
		}
	}
}
