//! Anatomical authoring, calibrated joint frames, and indexed pose buffers.
//!
//! Character space is the armature local frame: **+X** right, **+Y** up, **+Z**
//! fight-forward. Bevy world space stays **+Y** up and **−Z** camera-forward.
//! Convert positions and directions at the character [`GlobalTransform`] boundary;
//! do not send a direction through point conversion.
//!
//! Angles are radians. Positive spine flexion tips a +Y bone toward fight-forward
//! (+Z). Positive hip and knee flexion move the distal endpoint in the sagittal
//! (YZ) plane. Lateral bend is about +Z, axial turn about +Y. Composition order
//! inside a joint is flexion, then lateral bend, then axial rotation.
//!
//! Side mirroring lives on the joint frame. A positive knee flexion is the same
//! number on both legs; gait phase offsets are separate.

mod binding;
mod buffer;
mod forelimbed;
mod frame;
mod humanoid;
mod quadruped;
mod space;

pub use binding::{BoneId, RigBinding, RigDefinition, RigMetrics, SkeletonFamily};
pub use buffer::{ArmatureOffset, BlendCurve, PoseBuffer, PoseScratch};
pub use forelimbed::{
	forelimbed_v0_definition, resolve_forelimbed, ForelimbedPose, FORELIMBED_V0_BONES,
	FORELIMBED_V0_COUNT,
};
pub use frame::{JointAngles, JointFrame};
pub use humanoid::{
	humanoid_v0_definition, identity_binding as humanoid_identity_binding, resolve_humanoid,
	ArmAim, ArmPose, HumanoidPose, LegPose, NeckBone, NeckPose, SpinePose, HUMANOID_V0_BONES,
	HUMANOID_V0_COUNT,
};
pub use quadruped::{
	identity_binding as quadruped_identity_binding, quadruped_v0_definition, resolve_quadruped,
	QuadrupedLimbPose, QuadrupedPose, QUADRUPED_V0_BONES, QUADRUPED_V0_COUNT,
};
pub use space::{
	character_direction_from_world, CharacterPoint, WorldPoint, CHARACTER_FORWARD, CHARACTER_RIGHT,
	CHARACTER_UP,
};
