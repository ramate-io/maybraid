use crozon_rigs::{humanoid::HumanoidRig, Side};

/// Apply symmetric leg flexion (both legs share the same phase).
pub fn apply_leg<R: HumanoidRig>(rig: &mut R, side: Side, femur_swing: f32, shin_flex: f32) {
	let mut leg = rig.leg_pose(side);

	leg.femur = rig.articulate_on_rig(leg.femur, femur_swing, 0.0);
	leg.shin = rig.articulate_on_rig(leg.shin, 0.0, shin_flex);
	rig.pose_leg(leg);
}

pub fn apply_root<R: HumanoidRig>(rig: &mut R, root_swing: f32) {
	let mut spine = rig.spine_pose();
	spine.root = rig.articulate_on_rig(spine.root, root_swing, 0.0);
	rig.pose_spine(spine);
}

/// Sagittal fold on DEFAULT spine bones (`twist` ≈ pitch). Spreads the bend so
/// the torso reads as a fold, not a single-root yaw.
pub fn apply_spine_pitch<R: HumanoidRig>(rig: &mut R, pitch: f32) {
	let mut spine = rig.spine_pose();
	spine.root = rig.articulate_on_rig_twisted(spine.root, 0.0, 0.0, pitch * 0.28);
	spine.lumbar = rig.articulate_on_rig_twisted(spine.lumbar, 0.0, 0.0, pitch * 0.26);
	spine.midback = rig.articulate_on_rig_twisted(spine.midback, 0.0, 0.0, pitch * 0.24);
	spine.upper_back = rig.articulate_on_rig_twisted(spine.upper_back, 0.0, 0.0, pitch * 0.22);
	rig.pose_spine(spine);
}

/// Hip crease on DEFAULT pelvis (`twist` ≈ sagittal pitch).
pub fn apply_hip_fold<R: HumanoidRig>(rig: &mut R, side: Side, fold: f32) {
	let mut leg = rig.leg_pose(side);
	leg.pelvis = rig.articulate_on_rig_twisted(leg.pelvis, 0.0, 0.0, fold);
	rig.pose_leg(leg);
}

pub fn apply_neck<R: HumanoidRig>(
	rig: &mut R,
	lower_swing: f32,
	lower_flex: f32,
	upper_swing: f32,
	upper_flex: f32,
) {
	apply_neck_twisted(rig, lower_swing, lower_flex, 0.0, upper_swing, upper_flex, 0.0);
}

pub fn apply_neck_twisted<R: HumanoidRig>(
	rig: &mut R,
	lower_swing: f32,
	lower_flex: f32,
	lower_twist: f32,
	upper_swing: f32,
	upper_flex: f32,
	upper_twist: f32,
) {
	let mut neck = rig.neck_pose();
	neck.lower_neck =
		rig.articulate_on_rig_twisted(neck.lower_neck, lower_swing, lower_flex, lower_twist);
	neck.upper_neck =
		rig.articulate_on_rig_twisted(neck.upper_neck, upper_swing, upper_flex, upper_twist);
	rig.pose_neck(neck);
}

pub fn apply_arm<R: HumanoidRig>(
	rig: &mut R,
	side: Side,
	shoulder_swing: f32,
	shoulder_flex: f32,
	humerus_swing: f32,
	humerus_flex: f32,
	forearm_flex: f32,
) {
	apply_arm_twisted(
		rig,
		side,
		shoulder_swing,
		shoulder_flex,
		0.0,
		humerus_swing,
		humerus_flex,
		forearm_flex,
	);
}

/// Like [`apply_arm`], with shoulder long-axis twist (external/internal rotation).
pub fn apply_arm_twisted<R: HumanoidRig>(
	rig: &mut R,
	side: Side,
	shoulder_swing: f32,
	shoulder_flex: f32,
	shoulder_twist: f32,
	humerus_swing: f32,
	humerus_flex: f32,
	forearm_flex: f32,
) {
	let mut arm = rig.arm_pose(side);

	arm.shoulder =
		rig.articulate_on_rig_twisted(arm.shoulder, shoulder_swing, shoulder_flex, shoulder_twist);
	arm.humerus = rig.articulate_on_rig(arm.humerus, humerus_swing, humerus_flex);
	arm.forearm = rig.articulate_on_rig(arm.forearm, 0.0, forearm_flex);
	rig.pose_arm(arm);
}
