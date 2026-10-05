//! Quadruped semantic pose.
//!
//! Limb stride and hinge are sagittal flexion (+X). Proximal swing that used to
//! be parent-space yaw stays axial (+Y). Neck nod is flexion, side tilt is
//! lateral, and roll-about-length is axial.

use std::sync::{Arc, OnceLock};

use bevy::prelude::*;

use super::binding::{BoneId, RigBinding, RigDefinition, SkeletonFamily};
use super::buffer::PoseBuffer;
use super::frame::{JointAngles, JointFrame};
use crate::Side;

pub const QUADRUPED_V0_BONES: &[&str] = &[
	"back_ridge",
	"upper_back",
	"lumbar",
	"neck",
	"shoulder.L",
	"shoulder.R",
	"anterior_thigh.L",
	"anterior_shin.L",
	"anterior_thigh.R",
	"anterior_shin.R",
	"hip.L",
	"hip.R",
	"posterior_thigh.L",
	"posterior_shin.L",
	"posterior_thigh.R",
	"posterior_shin.R",
];

pub const QUADRUPED_V0_COUNT: usize = QUADRUPED_V0_BONES.len();

const PARENTS: &[(&str, &str)] = &[
	("upper_back", "back_ridge"),
	("lumbar", "back_ridge"),
	("neck", "upper_back"),
	("shoulder.L", "upper_back"),
	("shoulder.R", "upper_back"),
	("anterior_thigh.L", "shoulder.L"),
	("anterior_thigh.R", "shoulder.R"),
	("anterior_shin.L", "anterior_thigh.L"),
	("anterior_shin.R", "anterior_thigh.R"),
	("hip.L", "lumbar"),
	("hip.R", "lumbar"),
	("posterior_thigh.L", "hip.L"),
	("posterior_thigh.R", "hip.R"),
	("posterior_shin.L", "posterior_thigh.L"),
	("posterior_shin.R", "posterior_thigh.R"),
];

pub fn quadruped_v0_definition() -> Arc<RigDefinition> {
	static DEFINITION: OnceLock<Arc<RigDefinition>> = OnceLock::new();
	DEFINITION
		.get_or_init(|| {
			Arc::new(RigDefinition::from_names(
				SkeletonFamily::Quadruped,
				QUADRUPED_V0_BONES,
				PARENTS,
			))
		})
		.clone()
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct QuadrupedLimbPose {
	/// Shoulder or hip yaw, radians about +Y.
	pub proximal_turn: f32,
	/// Shoulder or hip lateral, radians about +Z.
	pub proximal_lateral: f32,
	/// Thigh sagittal stride.
	pub stride: f32,
	/// Shin sagittal hinge.
	pub hinge: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct QuadrupedPose {
	/// Back-ridge axial rotation (old swing).
	pub spine_axial: f32,
	/// Lumbar lateral bend (old flex). Gather clips still write this channel.
	pub spine_lateral: f32,
	/// Lumbar sagittal bend.
	pub spine_sagittal: f32,
	pub neck_nod: f32,
	pub neck_tilt: f32,
	pub neck_turn: f32,
	pub front: [QuadrupedLimbPose; 2],
	pub hind: [QuadrupedLimbPose; 2],
}

impl QuadrupedPose {
	pub fn front_mut(&mut self, side: Side) -> &mut QuadrupedLimbPose {
		&mut self.front[side.index()]
	}

	pub fn hind_mut(&mut self, side: Side) -> &mut QuadrupedLimbPose {
		&mut self.hind[side.index()]
	}
}

pub(crate) fn frame_for(
	definition: &RigDefinition,
	bone: BoneId,
	rest: Quat,
	parent: Quat,
) -> JointFrame {
	let _ = (definition, bone);
	JointFrame::calibrate_in_character(rest, parent).unwrap_or(JointFrame::IDENTITY)
}

pub fn resolve_quadruped(pose: &QuadrupedPose, binding: &RigBinding, out: &mut PoseBuffer) {
	apply(pose, &binding.definition, &binding.frames, &binding.effective_rest, out);
}

pub(crate) fn apply(
	pose: &QuadrupedPose,
	definition: &RigDefinition,
	frames: &[JointFrame],
	rest: &PoseBuffer,
	out: &mut PoseBuffer,
) {
	out.copy_from(rest);
	for (index, name) in definition.names.iter().enumerate() {
		let Some(angles) = angles_for(name, pose) else {
			continue;
		};
		let bone = BoneId(index as u16);
		let frame = frames.get(index).copied().unwrap_or(JointFrame::IDENTITY);
		out.set_rotation(bone, frame.local_rotation(rest.rotation(bone), angles));
	}
}

fn angles_for(name: &str, pose: &QuadrupedPose) -> Option<JointAngles> {
	let proximal = |limb: QuadrupedLimbPose| JointAngles {
		flexion: 0.0,
		lateral: limb.proximal_lateral,
		axial: limb.proximal_turn,
	};
	let stride =
		|limb: QuadrupedLimbPose| JointAngles { flexion: limb.stride, lateral: 0.0, axial: 0.0 };
	let hinge =
		|limb: QuadrupedLimbPose| JointAngles { flexion: limb.hinge, lateral: 0.0, axial: 0.0 };
	match name {
		"back_ridge" => Some(JointAngles { flexion: 0.0, lateral: 0.0, axial: pose.spine_axial }),
		"lumbar" => Some(JointAngles {
			flexion: pose.spine_sagittal,
			lateral: pose.spine_lateral,
			axial: 0.0,
		}),
		"neck" => Some(JointAngles {
			flexion: pose.neck_nod,
			lateral: pose.neck_tilt,
			axial: pose.neck_turn,
		}),
		"shoulder.L" => Some(proximal(pose.front[0])),
		"shoulder.R" => Some(proximal(pose.front[1])),
		"hip.L" => Some(proximal(pose.hind[0])),
		"hip.R" => Some(proximal(pose.hind[1])),
		"anterior_thigh.L" => Some(stride(pose.front[0])),
		"anterior_thigh.R" => Some(stride(pose.front[1])),
		"posterior_thigh.L" => Some(stride(pose.hind[0])),
		"posterior_thigh.R" => Some(stride(pose.hind[1])),
		"anterior_shin.L" => Some(hinge(pose.front[0])),
		"anterior_shin.R" => Some(hinge(pose.front[1])),
		"posterior_shin.L" => Some(hinge(pose.hind[0])),
		"posterior_shin.R" => Some(hinge(pose.hind[1])),
		_ => None,
	}
}

pub fn identity_binding() -> RigBinding {
	let definition = quadruped_v0_definition();
	let len = definition.len();
	RigBinding::from_rest(
		definition,
		vec![Entity::PLACEHOLDER; len].into_boxed_slice(),
		PoseBuffer::identity(len),
	)
}
