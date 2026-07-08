use crozon_rigs::{quadruped::QuadrupedRig, Side};

pub fn apply_front_leg<R: QuadrupedRig>(
	rig: &mut R,
	side: Side,
	shoulder_swing: f32,
	shoulder_flex: f32,
	thigh_swing: f32,
	shin_flex: f32,
) {
	let mut leg = rig.front_leg_pose(side);

	leg.shoulder = rig.articulate_on_rig(leg.shoulder, shoulder_swing, shoulder_flex);
	leg.thigh = rig.articulate_on_rig(leg.thigh, thigh_swing, 0.0);
	leg.shin = rig.articulate_on_rig(leg.shin, 0.0, shin_flex);
	rig.pose_front_leg(leg);
}

pub fn apply_hind_leg<R: QuadrupedRig>(
	rig: &mut R,
	side: Side,
	hip_swing: f32,
	hip_flex: f32,
	thigh_swing: f32,
	shin_flex: f32,
) {
	let mut leg = rig.hind_leg_pose(side);

	leg.hip = rig.articulate_on_rig(leg.hip, hip_swing, hip_flex);
	leg.thigh = rig.articulate_on_rig(leg.thigh, thigh_swing, 0.0);
	leg.shin = rig.articulate_on_rig(leg.shin, 0.0, shin_flex);
	rig.pose_hind_leg(leg);
}

pub fn apply_spine<R: QuadrupedRig>(rig: &mut R, back_ridge_swing: f32, lumbar_flex: f32) {
	let mut spine = rig.spine_pose();
	spine.back_ridge = rig.articulate_on_rig(spine.back_ridge, back_ridge_swing, 0.0);
	spine.lumbar = rig.articulate_on_rig(spine.lumbar, 0.0, lumbar_flex);
	rig.pose_spine(spine);
}

pub fn apply_neck<R: QuadrupedRig>(rig: &mut R, neck_swing: f32) {
	let mut neck = rig.neck_pose();
	neck.neck = rig.articulate_on_rig(neck.neck, neck_swing, 0.0);
	rig.pose_neck(neck);
}
