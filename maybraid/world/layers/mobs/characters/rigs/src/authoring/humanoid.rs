//! Humanoid semantic pose and V0 joint frames.
//!
//! Positive directions, character space, radians:
//!
//! | Control | Axis | Positive motion |
//! | --- | --- | --- |
//! | Forward bend / hip flexion / knee flexion / nod | +X | Distal +Y moves toward fight-forward +Z |
//! | Side bend / abduction / side tilt | +Z | Distal +Y moves toward −X |
//! | Turn / axial rotation | +Y | Fight-forward +Z moves toward −X |
//!
//! Spine stack weights sum to 1 so an authored bend is the total fold, split
//! across root, lumbar, mid-back, and upper back. Root-only lean (walk) is a
//! separate helper and does not spread.

use std::sync::{Arc, OnceLock};

use bevy::prelude::*;

use super::binding::{BoneId, RigBinding, RigDefinition, SkeletonFamily};
use super::buffer::PoseBuffer;
use super::frame::{JointAngles, JointFrame};
use crate::articulation::{rotation_along_with_roll, BONE_LENGTH_AXIS};
use crate::Side;

pub const HUMANOID_V0_BONES: &[&str] = &[
	"root",
	"lumbar",
	"midback",
	"upper_back",
	"lower_neck",
	"upper_neck",
	"shoulder.L",
	"humerus.L",
	"forearm.L",
	"shoulder.R",
	"humerus.R",
	"forearm.R",
	"pelvis.L",
	"femur.L",
	"shin.L",
	"pelvis.R",
	"femur.R",
	"shin.R",
];

pub const HUMANOID_V0_COUNT: usize = HUMANOID_V0_BONES.len();

const PARENTS: &[(&str, &str)] = &[
	("lumbar", "root"),
	("midback", "lumbar"),
	("upper_back", "midback"),
	("lower_neck", "upper_back"),
	("upper_neck", "lower_neck"),
	("shoulder.L", "upper_back"),
	("shoulder.R", "upper_back"),
	("humerus.L", "shoulder.L"),
	("humerus.R", "shoulder.R"),
	("forearm.L", "humerus.L"),
	("forearm.R", "humerus.R"),
	("femur.L", "pelvis.L"),
	("femur.R", "pelvis.R"),
	("shin.L", "femur.L"),
	("shin.R", "femur.R"),
];

const STACK_FORWARD: [f32; 4] = [0.28, 0.26, 0.24, 0.22];
const WAIST_FORWARD: [f32; 4] = [0.40, 0.35, 0.15, 0.10];
const TURN_WEIGHTS: [f32; 4] = [0.0, 0.35, 0.40, 0.25];

pub fn humanoid_v0_definition() -> Arc<RigDefinition> {
	static DEFINITION: OnceLock<Arc<RigDefinition>> = OnceLock::new();
	DEFINITION
		.get_or_init(|| {
			Arc::new(RigDefinition::from_names(
				SkeletonFamily::Humanoid,
				HUMANOID_V0_BONES,
				PARENTS,
			))
		})
		.clone()
}

/// Sagittal, lateral, and axial bend already split across the spine stack.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SpinePose {
	/// Index 0 root, 1 lumbar, 2 mid-back, 3 upper back. Radians.
	pub forward_bend: [f32; 4],
	pub side_bend: [f32; 4],
	pub turn: [f32; 4],
}

impl SpinePose {
	/// Full sagittal lean on the root. Walk uses this so the authored angle is not diluted.
	pub fn add_root_forward(&mut self, radians: f32) {
		self.forward_bend[0] += radians;
	}

	/// Held and looping squat share this spread. Weights sum to 1.
	pub fn add_stack_forward(&mut self, radians: f32) {
		for (slot, weight) in self.forward_bend.iter_mut().zip(STACK_FORWARD) {
			*slot += radians * weight;
		}
	}

	/// Jab waist fold, heavier at the root and lumbar.
	pub fn add_waist_forward(&mut self, radians: f32) {
		for (slot, weight) in self.forward_bend.iter_mut().zip(WAIST_FORWARD) {
			*slot += radians * weight;
		}
	}

	pub fn add_turn(&mut self, radians: f32) {
		for (slot, weight) in self.turn.iter_mut().zip(TURN_WEIGHTS) {
			*slot += radians * weight;
		}
	}
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NeckBone {
	pub nod: f32,
	pub side_tilt: f32,
	pub turn: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NeckPose {
	pub lower: NeckBone,
	pub upper: NeckBone,
}

/// One leg. Pelvis fields are the pelvis bone; hip fields are the femur.
///
/// The same positive [`Self::knee_flexion`] closes both knees. Phase offsets are not stored here.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LegPose {
	pub hip_flexion: f32,
	pub hip_abduction: f32,
	pub hip_rotation: f32,
	pub knee_flexion: f32,
	pub pelvis_flexion: f32,
	pub pelvis_lateral: f32,
	pub pelvis_turn: f32,
}

/// Character-space humerus aim. `along` is a direction, not a point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArmAim {
	pub along: Vec3,
	pub roll: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ArmPose {
	pub forward_elevation: f32,
	pub lateral_elevation: f32,
	pub axial_rotation: f32,
	pub elbow_flexion: f32,
	pub shoulder_forward: f32,
	pub shoulder_lift: f32,
	pub shoulder_twist: f32,
	pub aim: Option<ArmAim>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct HumanoidPose {
	pub spine: SpinePose,
	pub neck: NeckPose,
	pub legs: [LegPose; 2],
	pub arms: [ArmPose; 2],
}

impl HumanoidPose {
	pub fn leg_mut(&mut self, side: Side) -> &mut LegPose {
		&mut self.legs[side.index()]
	}

	pub fn arm_mut(&mut self, side: Side) -> &mut ArmPose {
		&mut self.arms[side.index()]
	}
}

pub(crate) fn frame_for(
	definition: &RigDefinition,
	bone: BoneId,
	rest: Quat,
	parent: Quat,
) -> JointFrame {
	let _ = definition;
	let _ = bone;
	// Character +X / +Z / +Y, conjugated into this bone's parent. The inspected
	// pelvis bind sends local +X to character +Z; without the parent conjugate,
	// hip flexion swings the thigh sideways. Right-side hinges do not take an
	// extra sign: both knees share positive flexion.
	JointFrame::calibrate_in_character(rest, parent).unwrap_or(JointFrame::IDENTITY)
}

pub fn resolve_humanoid(pose: &HumanoidPose, binding: &RigBinding, out: &mut PoseBuffer) {
	apply(pose, &binding.definition, &binding.frames, &binding.effective_rest, out);
}

pub(crate) fn apply(
	pose: &HumanoidPose,
	definition: &RigDefinition,
	frames: &[JointFrame],
	rest: &PoseBuffer,
	out: &mut PoseBuffer,
) {
	out.copy_from(rest);
	for (index, name) in definition.names.iter().enumerate() {
		let bone = BoneId(index as u16);
		let Some(angles) = angles_for(name, pose) else {
			continue;
		};
		let frame = frames.get(index).copied().unwrap_or(JointFrame::IDENTITY);
		let rest_rotation = rest.rotation(bone);
		out.set_rotation(bone, frame.local_rotation(rest_rotation, angles));
	}
	for side in [Side::Left, Side::Right] {
		let Some(aim) = pose.arms[side.index()].aim else {
			continue;
		};
		aim_humerus(definition, rest, out, side, aim);
	}
}

fn angles_for(name: &str, pose: &HumanoidPose) -> Option<JointAngles> {
	let spine = |slot: usize| JointAngles {
		flexion: pose.spine.forward_bend[slot],
		lateral: pose.spine.side_bend[slot],
		axial: pose.spine.turn[slot],
	};
	let neck = |bone: NeckBone| JointAngles {
		flexion: bone.nod,
		lateral: bone.side_tilt,
		axial: bone.turn,
	};
	let leg = |side: Side| {
		let leg = pose.legs[side.index()];
		JointAngles {
			flexion: leg.hip_flexion,
			lateral: leg.hip_abduction,
			axial: leg.hip_rotation,
		}
	};
	let pelvis = |side: Side| {
		let leg = pose.legs[side.index()];
		JointAngles {
			flexion: leg.pelvis_flexion,
			lateral: leg.pelvis_lateral,
			axial: leg.pelvis_turn,
		}
	};
	let shoulder = |side: Side| {
		let arm = pose.arms[side.index()];
		JointAngles {
			flexion: arm.shoulder_forward,
			lateral: arm.shoulder_lift,
			axial: arm.shoulder_twist,
		}
	};
	let humerus = |side: Side| {
		let arm = pose.arms[side.index()];
		if arm.aim.is_some() {
			None
		} else {
			Some(JointAngles {
				flexion: arm.forward_elevation,
				lateral: arm.lateral_elevation,
				axial: arm.axial_rotation,
			})
		}
	};
	let elbow = |side: Side| JointAngles {
		flexion: pose.arms[side.index()].elbow_flexion,
		lateral: 0.0,
		axial: 0.0,
	};
	match name {
		"root" => Some(spine(0)),
		"lumbar" => Some(spine(1)),
		"midback" => Some(spine(2)),
		"upper_back" => Some(spine(3)),
		"lower_neck" => Some(neck(pose.neck.lower)),
		"upper_neck" => Some(neck(pose.neck.upper)),
		"pelvis.L" => Some(pelvis(Side::Left)),
		"pelvis.R" => Some(pelvis(Side::Right)),
		"femur.L" => Some(leg(Side::Left)),
		"femur.R" => Some(leg(Side::Right)),
		"shin.L" => Some(elbow_like(pose.legs[0].knee_flexion)),
		"shin.R" => Some(elbow_like(pose.legs[1].knee_flexion)),
		"shoulder.L" => Some(shoulder(Side::Left)),
		"shoulder.R" => Some(shoulder(Side::Right)),
		"humerus.L" => humerus(Side::Left),
		"humerus.R" => humerus(Side::Right),
		"forearm.L" => Some(elbow(Side::Left)),
		"forearm.R" => Some(elbow(Side::Right)),
		_ => None,
	}
}

fn elbow_like(flexion: f32) -> JointAngles {
	JointAngles { flexion, lateral: 0.0, axial: 0.0 }
}

fn aim_humerus(
	definition: &RigDefinition,
	rest: &PoseBuffer,
	out: &mut PoseBuffer,
	side: Side,
	aim: ArmAim,
) {
	let name = match side {
		Side::Left => "humerus.L",
		Side::Right => "humerus.R",
	};
	let Some(bone) = definition.id(name) else {
		return;
	};
	let parent = definition.parent_rotation(out, bone);
	let along_parent = parent.inverse() * aim.along;
	let posed =
		rotation_along_with_roll(rest.rotation(bone), along_parent, aim.roll, BONE_LENGTH_AXIS);
	out.set_rotation(bone, posed);
}

/// Identity-rest binding used by clip tests. Production bindings come from imported bones.
pub fn identity_binding() -> RigBinding {
	let definition = humanoid_v0_definition();
	let len = definition.len();
	RigBinding::from_rest(
		definition,
		vec![Entity::PLACEHOLDER; len].into_boxed_slice(),
		PoseBuffer::identity(len),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::f32::consts::FRAC_PI_2;

	fn bone(binding: &RigBinding, name: &str) -> BoneId {
		binding.definition.id(name).expect(name)
	}

	#[test]
	fn walk_style_root_lean_is_sagittal() {
		let binding = identity_binding();
		let mut pose = HumanoidPose::default();
		pose.spine.add_root_forward(0.4);
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let tipped = out.rotation(bone(&binding, "root")) * Vec3::Y;
		assert!(tipped.z > 0.2, "forward lean goes to +Z, got {tipped:?}");
		assert!(tipped.x.abs() < 1e-3, "lean must not yaw, got {tipped:?}");
	}

	#[test]
	fn stack_and_root_helpers_share_the_sagittal_plane() {
		let binding = identity_binding();
		let mut spread = HumanoidPose::default();
		spread.spine.add_stack_forward(0.8);
		let mut root_only = HumanoidPose::default();
		root_only.spine.add_root_forward(0.8);
		let mut spread_out = PoseBuffer::identity(binding.definition.len());
		let mut root_out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&spread, &binding, &mut spread_out);
		resolve_humanoid(&root_only, &binding, &mut root_out);
		for name in ["root", "lumbar"] {
			let spread_tip = spread_out.rotation(bone(&binding, name)) * Vec3::Y;
			let root_tip = root_out.rotation(bone(&binding, name)) * Vec3::Y;
			assert!(spread_tip.x.abs() < 1e-3, "{name} spread yaw {spread_tip:?}");
			assert!(root_tip.x.abs() < 1e-3, "{name} root yaw {root_tip:?}");
		}
		assert!(
			(spread_out.rotation(bone(&binding, "root")) * Vec3::Y).z > 0.05,
			"spread still pitches the root"
		);
	}

	#[test]
	fn hip_and_knee_flexion_move_endpoints_sagittally() {
		let binding = identity_binding();
		let mut pose = HumanoidPose::default();
		pose.leg_mut(Side::Left).hip_flexion = 0.5;
		pose.leg_mut(Side::Left).knee_flexion = 0.7;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let thigh = out.rotation(bone(&binding, "femur.L")) * Vec3::Y;
		let shin = out.rotation(bone(&binding, "shin.L")) * Vec3::Y;
		assert!(thigh.z > 0.2 && thigh.x.abs() < 1e-3, "hip {thigh:?}");
		assert!(shin.z > 0.2 && shin.x.abs() < 1e-3, "knee {shin:?}");
	}

	#[test]
	fn left_and_right_knees_share_positive_flexion() {
		let binding = identity_binding();
		let mut pose = HumanoidPose::default();
		pose.leg_mut(Side::Left).knee_flexion = 0.6;
		pose.leg_mut(Side::Right).knee_flexion = 0.6;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let left = out.rotation(bone(&binding, "shin.L")) * Vec3::Y;
		let right = out.rotation(bone(&binding, "shin.R")) * Vec3::Y;
		assert!(
			(left - right).length() < 1e-4,
			"same flexion, same sagittal bend: {left:?} vs {right:?}"
		);
	}

	#[test]
	fn imported_pelvis_bind_keeps_hip_flexion_sagittal() {
		let definition = humanoid_v0_definition();
		let mut rest = PoseBuffer::identity(definition.len());
		let pelvis = definition.id("pelvis.L").expect("pelvis");
		let femur = definition.id("femur.L").expect("femur");
		let shin = definition.id("shin.L").expect("shin");
		// Inspected humanoid_rig.glb: pelvis.L permutes +X onto character +Z, and
		// femur.L aims local +Y along pelvis −Z so the thigh hangs down.
		rest.local[pelvis.index()].rotation = Quat::from_xyzw(-0.5, -0.5, -0.5, 0.5);
		rest.local[femur.index()].rotation = Quat::from_rotation_x(-FRAC_PI_2);
		let binding = RigBinding::from_rest(
			definition,
			vec![Entity::PLACEHOLDER; rest.len()].into_boxed_slice(),
			rest,
		);
		let mut pose = HumanoidPose::default();
		pose.leg_mut(Side::Left).hip_flexion = 0.5;
		pose.leg_mut(Side::Left).knee_flexion = 0.6;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let thigh = binding.definition.rotation_in_character(&out, femur) * Vec3::Y;
		let shin_dir = binding.definition.rotation_in_character(&out, shin) * Vec3::Y;
		assert!(thigh.z.abs() > 0.3, "stride must move back/front, got {thigh:?}");
		assert!(thigh.x.abs() < 0.08, "stride must not swing sideways, got {thigh:?}");
		assert!(shin_dir.z.abs() > shin_dir.x.abs(), "knee stays sagittal, got {shin_dir:?}");
	}

	#[test]
	fn femur_bind_redirecting_length_still_flexes_sagittally() {
		let definition = humanoid_v0_definition();
		let mut rest = PoseBuffer::identity(definition.len());
		let femur = definition.id("femur.L").expect("femur");
		rest.local[femur.index()].rotation = Quat::from_rotation_x(-FRAC_PI_2);
		let binding = RigBinding::from_rest(
			definition,
			vec![Entity::PLACEHOLDER; rest.len()].into_boxed_slice(),
			rest,
		);
		let mut pose = HumanoidPose::default();
		pose.leg_mut(Side::Left).hip_flexion = 0.5;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let dir = out.rotation(femur) * Vec3::Y;
		assert!(dir.x.abs() < 0.08, "non-identity femur flexion stays sagittal, got {dir:?}");
		assert!((dir - Vec3::NEG_Z).length() > 0.15, "endpoint moved, got {dir:?}");
	}

	#[test]
	fn neck_nod_turn_and_tilt_are_independent() {
		let binding = identity_binding();
		let mut pose = HumanoidPose::default();
		pose.neck.lower.nod = 0.4;
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let nodded = out.rotation(bone(&binding, "lower_neck")) * Vec3::Y;
		assert!(nodded.z > 0.1 && nodded.x.abs() < 1e-3, "{nodded:?}");

		let mut pose = HumanoidPose::default();
		pose.neck.lower.turn = 0.4;
		resolve_humanoid(&pose, &binding, &mut out);
		let turned = out.rotation(bone(&binding, "lower_neck")) * Vec3::Z;
		assert!(turned.x.abs() > 0.1 && turned.y.abs() < 1e-3, "{turned:?}");

		let mut pose = HumanoidPose::default();
		pose.neck.lower.side_tilt = 0.4;
		resolve_humanoid(&pose, &binding, &mut out);
		let tilted = out.rotation(bone(&binding, "lower_neck")) * Vec3::Y;
		assert!(tilted.x.abs() > 0.1 && tilted.z.abs() < 1e-3, "{tilted:?}");
	}

	#[test]
	fn repeated_resolve_matches_a_fresh_buffer() {
		let binding = identity_binding();
		let mut pose = HumanoidPose::default();
		pose.spine.add_root_forward(0.2);
		pose.leg_mut(Side::Right).hip_flexion = -0.4;
		let mut first = PoseBuffer::identity(binding.definition.len());
		let mut second = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut first);
		resolve_humanoid(&pose, &binding, &mut first);
		resolve_humanoid(&pose, &binding, &mut second);
		assert_eq!(first, second);
	}

	#[test]
	fn aim_sees_the_shoulder_written_in_this_sample() {
		let binding = identity_binding();
		let mut pose = HumanoidPose::default();
		pose.arm_mut(Side::Right).shoulder_forward = 0.0;
		pose.arm_mut(Side::Right).aim =
			Some(ArmAim { along: Vec3::new(0.0, -0.2, 1.0).normalize(), roll: 0.0 });
		let mut out = PoseBuffer::identity(binding.definition.len());
		resolve_humanoid(&pose, &binding, &mut out);
		let humerus = bone(&binding, "humerus.R");
		let parent = binding.definition.parent_rotation(&out, humerus);
		let aimed = (parent * out.rotation(humerus) * BONE_LENGTH_AXIS).normalize();
		assert!(aimed.dot(Vec3::new(0.0, -0.2, 1.0).normalize()) > 0.99, "aim {aimed:?}");
	}
}
