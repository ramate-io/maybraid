//! Fill a [`HumanoidPose`] from authored angles.
//!
//! These helpers do not choose imported skeleton axes. Flexion, lateral bend, and
//! axial turn are the anatomical channels on [`HumanoidPose`]. Side mirroring lives
//! in the joint frames, so a positive knee flexion is the same number on both legs.

use character_rigs::authoring::HumanoidPose;
use character_rigs::Side;

/// Symmetric leg flexion. `femur_swing` is hip flexion; `shin_flex` is knee flexion.
pub fn apply_leg(pose: &mut HumanoidPose, side: Side, femur_swing: f32, shin_flex: f32) {
	let leg = pose.leg_mut(side);
	leg.hip_flexion += femur_swing;
	leg.knee_flexion += shin_flex;
}

/// Sagittal lean on the root only. Walk and other “lean” knobs use this so the
/// authored angle is a forward bend, not a yaw.
pub fn apply_root(pose: &mut HumanoidPose, root_swing: f32) {
	pose.spine.add_root_forward(root_swing);
}

/// Sagittal fold spread across the spine stack. Held and looping squat share it.
pub fn apply_spine_pitch(pose: &mut HumanoidPose, pitch: f32) {
	pose.spine.add_stack_forward(pitch);
}

/// Hip crease as pelvis flexion.
pub fn apply_hip_fold(pose: &mut HumanoidPose, side: Side, fold: f32) {
	pose.leg_mut(side).pelvis_flexion += fold;
}

pub fn apply_neck(
	pose: &mut HumanoidPose,
	lower_swing: f32,
	lower_flex: f32,
	upper_swing: f32,
	upper_flex: f32,
) {
	apply_neck_twisted(pose, lower_swing, lower_flex, 0.0, upper_swing, upper_flex, 0.0);
}

/// Neck channels: swing is turn, flex is side tilt, twist is nod.
pub fn apply_neck_twisted(
	pose: &mut HumanoidPose,
	lower_swing: f32,
	lower_flex: f32,
	lower_twist: f32,
	upper_swing: f32,
	upper_flex: f32,
	upper_twist: f32,
) {
	pose.neck.lower.turn += lower_swing;
	pose.neck.lower.side_tilt += lower_flex;
	pose.neck.lower.nod += lower_twist;
	pose.neck.upper.turn += upper_swing;
	pose.neck.upper.side_tilt += upper_flex;
	pose.neck.upper.nod += upper_twist;
}

pub fn apply_arm(
	pose: &mut HumanoidPose,
	side: Side,
	shoulder_swing: f32,
	shoulder_flex: f32,
	humerus_swing: f32,
	humerus_flex: f32,
	forearm_flex: f32,
) {
	apply_arm_twisted(
		pose,
		side,
		shoulder_swing,
		shoulder_flex,
		0.0,
		humerus_swing,
		humerus_flex,
		forearm_flex,
	);
}

/// Shoulder and humerus swing become forward flexion. Flex becomes lateral elevation.
/// Elbow flexion is the same positive direction on both arms.
pub fn apply_arm_twisted(
	pose: &mut HumanoidPose,
	side: Side,
	shoulder_swing: f32,
	shoulder_flex: f32,
	shoulder_twist: f32,
	humerus_swing: f32,
	humerus_flex: f32,
	forearm_flex: f32,
) {
	let arm = pose.arm_mut(side);
	arm.shoulder_forward += shoulder_swing;
	arm.shoulder_lift += shoulder_flex;
	arm.shoulder_twist += shoulder_twist;
	arm.forward_elevation += humerus_swing;
	arm.lateral_elevation += humerus_flex;
	arm.elbow_flexion += forearm_flex;
}
