//! Shared CPU linear-blend skinning for posed-mesh consumers.
//!
//! Matches Bevy `skinning.wgsl` (`joint_global * inverse_bind`, blended per
//! influence). [`screen-capture`] AABB extents and [`character-mesh-export`]
//! mesh baking both consume [`PosePolicy`] so missing influences, collapsed
//! joints, dead vertices, root space, normals, and mirrored winding stay
//! explicit instead of being reimplemented.

mod palette;
mod policy;
mod sample;
mod winding;

pub use palette::{JointPalette, JointSlot};
pub use policy::{
	CollapsedJoint, DeadVertex, MissingInfluence, NormalPolicy, PosePolicy, PoseSpace,
};
pub use sample::{CpuPosedMesh, PosedVertex, PosedVertexVisitor};
pub use winding::WindingPolicy;

#[cfg(test)]
mod tests;
