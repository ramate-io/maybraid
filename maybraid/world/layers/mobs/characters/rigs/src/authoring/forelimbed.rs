//! Forelimbed (axial swimmer) semantic pose.
//!
//! Lateral undulation is a yaw about character +Y. Dorsoventral bend is flexion
//! about +X. Fin sweep is shoulder yaw; fin flap is shoulder lateral bend.

use std::sync::{Arc, OnceLock};

use bevy::prelude::*;

use super::binding::{BoneId, RigBinding, RigDefinition, SkeletonFamily};
use super::buffer::PoseBuffer;
use super::frame::{JointAngles, JointFrame};

/// upper_mid, upper, lower_mid, lower, tailbone.
pub const FORELIMBED_SPINE: usize = 5;

pub const FORELIMBED_V0_BONES: &[&str] = &[
	"upper_mid_spine",
	"upper_spine",
	"lower_mid_spine",
	"lower_spine",
	"tailbone",
	"back_ridge",
	"shoulder.L",
	"upper_arm.L",
	"lower_arm.L",
	"shoulder.R",
	"upper_arm.R",
	"lower_arm.R",
];

pub const FORELIMBED_V0_COUNT: usize = FORELIMBED_V0_BONES.len();

const PARENTS: &[(&str, &str)] = &[
	("upper_spine", "upper_mid_spine"),
	("lower_spine", "lower_mid_spine"),
	("tailbone", "lower_spine"),
	("shoulder.L", "upper_spine"),
	("upper_arm.L", "shoulder.L"),
	("lower_arm.L", "upper_arm.L"),
	("shoulder.R", "upper_spine"),
	("upper_arm.R", "shoulder.R"),
	("lower_arm.R", "upper_arm.R"),
];

const SPINE_NAMES: [&str; FORELIMBED_SPINE] =
	["upper_mid_spine", "upper_spine", "lower_mid_spine", "lower_spine", "tailbone"];

pub fn forelimbed_v0_definition() -> Arc<RigDefinition> {
	static DEFINITION: OnceLock<Arc<RigDefinition>> = OnceLock::new();
	DEFINITION
		.get_or_init(|| {
			Arc::new(RigDefinition::from_names(
				SkeletonFamily::Forelimbed,
				FORELIMBED_V0_BONES,
				PARENTS,
			))
		})
		.clone()
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ForelimbedPose {
	/// Lateral yaw per spine bone, radians about +Y.
	pub lateral: [f32; FORELIMBED_SPINE],
	/// Dorsoventral bend per spine bone, radians about +X.
	pub dorsoventral: [f32; FORELIMBED_SPINE],
	/// Long-axis twist per spine bone.
	pub axial: [f32; FORELIMBED_SPINE],
	/// Pectoral sweep, left = 0, right = 1.
	pub fin_sweep: [f32; 2],
	/// Pectoral flap, left = 0, right = 1.
	pub fin_flap: [f32; 2],
}

pub(crate) fn frame_for(rest: Quat, parent: Quat) -> JointFrame {
	JointFrame::calibrate_in_character(rest, parent).unwrap_or(JointFrame::IDENTITY)
}

pub fn resolve_forelimbed(pose: &ForelimbedPose, binding: &RigBinding, out: &mut PoseBuffer) {
	apply(pose, &binding.definition, &binding.frames, &binding.effective_rest, out);
}

pub(crate) fn apply(
	pose: &ForelimbedPose,
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

fn angles_for(name: &str, pose: &ForelimbedPose) -> Option<JointAngles> {
	if let Some(slot) = SPINE_NAMES.iter().position(|candidate| *candidate == name) {
		// Lateral undulation is yaw (+Y axial). Dorsoventral bend is flexion (+X).
		return Some(JointAngles {
			flexion: pose.dorsoventral[slot],
			lateral: pose.axial[slot],
			axial: pose.lateral[slot],
		});
	}
	let fin = |side: usize, sweep_scale: f32, flap_scale: f32| JointAngles {
		flexion: 0.0,
		lateral: pose.fin_flap[side] * flap_scale,
		axial: pose.fin_sweep[side] * sweep_scale,
	};
	match name {
		"shoulder.L" => Some(fin(0, 1.0, 1.0)),
		"upper_arm.L" => Some(fin(0, 0.6, 0.6)),
		"shoulder.R" => Some(fin(1, 1.0, 1.0)),
		"upper_arm.R" => Some(fin(1, 0.6, 0.6)),
		_ => None,
	}
}
