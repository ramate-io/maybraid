use crozon_rigs::{quadruped::QuadrupedRig, Side};

use crate::rigs::quadruped::apply::{apply_front_leg, apply_hind_leg};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct KneeTuning {
	pub knee_neutral: f32,
	pub knee_contracted: f32,
	pub knee_extended: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LegStrideTuning {
	pub shoulder_swing: f32,
	pub shoulder_lift: f32,
	pub hip_swing: f32,
	pub hip_lift: f32,
	pub stride: f32,
	pub knee: KneeTuning,
}

pub(crate) fn thigh_swing(phase: f32) -> f32 {
	let p = phase.fract();
	if p < 0.5 {
		4.0 * p - 1.0
	} else {
		3.0 - 4.0 * p
	}
}

pub(crate) fn knee_flex(leg_phase: f32, knee: KneeTuning) -> f32 {
	let p = leg_phase.fract();
	let peak = if p < 0.5 { knee.knee_extended } else { knee.knee_contracted };
	let t = if p < 0.5 { p * 2.0 } else { (p - 0.5) * 2.0 };
	knee.knee_neutral + (t * std::f32::consts::PI).sin() * (peak - knee.knee_neutral)
}

pub(crate) fn leg_phase_from_strike(cycle: f32, strike: f32) -> f32 {
	(cycle - strike).rem_euclid(1.0)
}

/// Smooth bell envelope: 1 at `center`, 0 outside `center ± half_width`.
pub(crate) fn smooth_pulse(u: f32, center: f32, half_width: f32) -> f32 {
	if half_width <= 0.0 {
		return 0.0;
	}
	let d = ((u - center) / half_width).abs();
	if d >= 1.0 {
		return 0.0;
	}
	let t = 1.0 - d;
	t * t * (3.0 - 2.0 * t)
}

/// Smooth 0→1 transition used to crossfade hind- vs front-dominant bound poses.
pub(crate) fn smooth_blend(edge0: f32, edge1: f32, u: f32) -> f32 {
	if edge0 >= edge1 {
		return if u >= edge0 { 1.0 } else { 0.0 };
	}
	let t = ((u - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
	t * t * (3.0 - 2.0 * t)
}

pub(crate) fn apply_front_leg_enveloped<R: QuadrupedRig>(
	rig: &mut R,
	side: Side,
	leg_phase: f32,
	envelope: f32,
	tuning: LegStrideTuning,
) {
	if envelope <= 1e-5 {
		return;
	}
	let swing = thigh_swing(leg_phase) * envelope;
	let lift_sign = if side == Side::Left { -1.0 } else { 1.0 };
	let shin_flex = (knee_flex(leg_phase, tuning.knee) - tuning.knee.knee_extended) * envelope;

	apply_front_leg(
		rig,
		side,
		swing * tuning.shoulder_swing,
		swing * tuning.shoulder_lift * lift_sign,
		swing * tuning.stride,
		shin_flex,
	);
}

pub(crate) fn apply_hind_leg_enveloped<R: QuadrupedRig>(
	rig: &mut R,
	side: Side,
	leg_phase: f32,
	envelope: f32,
	tuning: LegStrideTuning,
) {
	if envelope <= 1e-5 {
		return;
	}
	let swing = thigh_swing(leg_phase) * envelope;
	let lift_sign = if side == Side::Left { -1.0 } else { 1.0 };
	let shin_flex = (knee_flex(leg_phase, tuning.knee) - tuning.knee.knee_extended) * envelope;

	apply_hind_leg(
		rig,
		side,
		swing * tuning.hip_swing,
		swing * tuning.hip_lift * lift_sign,
		swing * tuning.stride,
		shin_flex,
	);
}

pub(crate) fn apply_front_leg_at_strike<R: QuadrupedRig>(
	rig: &mut R,
	side: Side,
	cycle: f32,
	strike: f32,
	tuning: LegStrideTuning,
) {
	let leg_phase = leg_phase_from_strike(cycle, strike);
	let swing = thigh_swing(leg_phase);
	let lift_sign = if side == Side::Left { -1.0 } else { 1.0 };

	apply_front_leg(
		rig,
		side,
		swing * tuning.shoulder_swing,
		swing * tuning.shoulder_lift * lift_sign,
		swing * tuning.stride,
		knee_flex(leg_phase, tuning.knee) - tuning.knee.knee_extended,
	);
}

pub(crate) fn apply_hind_leg_at_strike<R: QuadrupedRig>(
	rig: &mut R,
	side: Side,
	cycle: f32,
	strike: f32,
	tuning: LegStrideTuning,
) {
	let leg_phase = leg_phase_from_strike(cycle, strike);
	let swing = thigh_swing(leg_phase);
	let lift_sign = if side == Side::Left { -1.0 } else { 1.0 };

	apply_hind_leg(
		rig,
		side,
		swing * tuning.hip_swing,
		swing * tuning.hip_lift * lift_sign,
		swing * tuning.stride,
		knee_flex(leg_phase, tuning.knee) - tuning.knee.knee_extended,
	);
}
