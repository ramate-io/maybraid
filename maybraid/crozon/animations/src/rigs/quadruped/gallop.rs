use std::f32::consts::PI;

use crozon_rigs::{quadruped::QuadrupedRig, Side};

use crate::animations::{Gallop, QuadrupedGallop};
use crate::rigs::quadruped::apply::{apply_neck, apply_spine};
use crate::rigs::quadruped::gait::{
	apply_front_leg_enveloped, apply_hind_leg_enveloped, smooth_blend, smooth_pulse,
	KneeTuning, LegStrideTuning,
};
use crate::{Animation, Effects, Progress};

/// Position within the current half-bound, in `[0, 1)`.
const BOUND_HALVES_PER_CYCLE: f32 = 2.0;

/// Hind pair fires early in the bound; front pair follows after a short gap.
const HIND_PULSE_CENTER: f32 = 0.14;
const FRONT_PULSE_CENTER: f32 = 0.40;
const PAIR_PULSE_HALF_WIDTH: f32 = 0.22;

/// Slight phase offset between paired legs on the same end.
const PAIR_PHASE_STAGGER: f32 = 0.06;

/// Hind legs complete a compressed stride early; front legs use a later, offset phase.
const HIND_PHASE_RATE: f32 = 2.4;
const FRONT_PHASE_RATE: f32 = 2.6;
const FRONT_PHASE_ONSET: f32 = 0.12;

/// Crossfade hind-dominant vs front-dominant bound poses.
const BOUND_MIX_START: f32 = 0.18;
const BOUND_MIX_END: f32 = 0.44;

impl<R: QuadrupedRig> Animation<R> for Gallop {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		QuadrupedGallop::from_gallop(self).apply(rig, progress)
	}
}

impl<R: QuadrupedRig> Animation<R> for QuadrupedGallop<R> {
	fn apply(&self, rig: &mut R, progress: f32) -> Effects {
		let cycle = Progress(progress).cycle();
		let bound_u = (cycle * BOUND_HALVES_PER_CYCLE).fract();
		let lead = lead_blend(cycle);
		let bound_mix = smooth_blend(BOUND_MIX_START, BOUND_MIX_END, bound_u);
		let tuning = leg_tuning(self);

		apply_hind_leg_enveloped(
			rig,
			Side::Left,
			hind_leg_phase(bound_u, Side::Left, lead),
			hind_envelope(bound_u, Side::Left, lead, bound_mix),
			tuning,
		);
		apply_hind_leg_enveloped(
			rig,
			Side::Right,
			hind_leg_phase(bound_u, Side::Right, lead),
			hind_envelope(bound_u, Side::Right, lead, bound_mix),
			tuning,
		);
		apply_front_leg_enveloped(
			rig,
			Side::Left,
			front_leg_phase(bound_u, Side::Left, lead),
			front_envelope(bound_u, Side::Left, lead, bound_mix),
			tuning,
		);
		apply_front_leg_enveloped(
			rig,
			Side::Right,
			front_leg_phase(bound_u, Side::Right, lead),
			front_envelope(bound_u, Side::Right, lead, bound_mix),
			tuning,
		);

		let spine_flex = bound_spine_flex(bound_u, bound_mix, self.hind_bound_pitch, self.front_bound_pitch);
		apply_spine(rig, spine_flex * 0.35, spine_flex);
		apply_neck(rig, -spine_flex * self.neck_follow);

		Effects::default()
	}
}

/// Continuous left↔right lead crossfade over the full cycle (`0` = left leads, `1` = right).
fn lead_blend(cycle: f32) -> f32 {
	0.5 - 0.5 * (cycle * PI * 2.0).cos()
}

/// Phase offset within a paired set; `lead` picks which side fires slightly earlier.
fn pair_phase_stagger(side: Side, lead: f32) -> f32 {
	let side_lead = match side {
		Side::Left => 1.0 - lead,
		Side::Right => lead,
	};
	side_lead * PAIR_PHASE_STAGGER
}

fn hind_leg_phase(bound_u: f32, side: Side, lead: f32) -> f32 {
	bound_u * HIND_PHASE_RATE + pair_phase_stagger(side, lead)
}

fn front_leg_phase(bound_u: f32, side: Side, lead: f32) -> f32 {
	(bound_u - FRONT_PHASE_ONSET).max(0.0) * FRONT_PHASE_RATE + pair_phase_stagger(side, lead)
}

/// Hind envelope: strong while `bound_mix` is low, fading as the front bound takes over.
fn hind_envelope(bound_u: f32, side: Side, lead: f32, bound_mix: f32) -> f32 {
	let pulse_center = HIND_PULSE_CENTER + pair_phase_stagger(side, lead) * 0.5;
	let pulse = smooth_pulse(bound_u, pulse_center, PAIR_PULSE_HALF_WIDTH);
	pulse * (1.0 - bound_mix)
}

/// Front envelope: rises as `bound_mix` increases.
fn front_envelope(bound_u: f32, side: Side, lead: f32, bound_mix: f32) -> f32 {
	let pulse_center = FRONT_PULSE_CENTER + pair_phase_stagger(side, lead) * 0.5;
	let pulse = smooth_pulse(bound_u, pulse_center, PAIR_PULSE_HALF_WIDTH);
	pulse * bound_mix
}

/// Single hump per bound (`0` at both ends of the half-cycle) blended between hind and front pitch.
fn bound_spine_flex(bound_u: f32, bound_mix: f32, hind_pitch: f32, front_pitch: f32) -> f32 {
	let bound_wave = (bound_u * PI).sin();
	let hind_component = bound_wave * (1.0 - bound_mix) * hind_pitch;
	let front_component = bound_wave * bound_mix * front_pitch;
	hind_component - front_component
}

fn leg_tuning<Rig>(gallop: &QuadrupedGallop<Rig>) -> LegStrideTuning {
	LegStrideTuning {
		shoulder_swing: gallop.shoulder_swing,
		shoulder_lift: gallop.shoulder_lift,
		hip_swing: gallop.hip_swing,
		hip_lift: gallop.hip_lift,
		stride: gallop.stride,
		knee: KneeTuning {
			knee_neutral: gallop.knee_neutral,
			knee_contracted: gallop.knee_contracted,
			knee_extended: gallop.knee_extended,
		},
	}
}

#[cfg(test)]
mod tests {
	use crozon_rigs::{rigs::quadruped_v0::QuadrupedV0Rig, Side};

	use super::*;

	#[test]
	fn gallop_delegates_to_quadruped_default() {
		for &phase in &[0.0, 0.12, 0.25, 0.62, 0.75] {
			let mut from_gallop = QuadrupedV0Rig::imported();
			let mut from_template = QuadrupedV0Rig::imported();
			Gallop::default().apply(&mut from_gallop, phase);
			QuadrupedGallop::<QuadrupedV0Rig>::default().apply(&mut from_template, phase);

			for bone in from_gallop.animation_bones() {
				let Some(gallop_pose) = from_gallop.pose().get(&bone) else {
					continue;
				};
				let template_pose = from_template.pose().get(&bone).expect("template pose");
				assert!(
					(gallop_pose.swing - template_pose.swing).abs() < 1e-5,
					"swing mismatch on {bone} at {phase}"
				);
			}
		}
	}

	#[test]
	fn gallop_hind_pair_stays_near_phase_during_hind_bound() {
		let mut rig = QuadrupedV0Rig::imported();
		QuadrupedGallop::<QuadrupedV0Rig>::default().apply(&mut rig, 0.06);

		let hind_left = rig
			.pose()
			.get(&rig.hind_leg(Side::Left).thigh.name)
			.expect("hind left");
		let hind_right = rig
			.pose()
			.get(&rig.hind_leg(Side::Right).thigh.name)
			.expect("hind right");
		assert!(
			(hind_left.swing - hind_right.swing).abs() < 0.35,
			"hind pair should stay near phase during the hind bound"
		);
	}

	#[test]
	fn gallop_lead_blend_is_continuous_across_half_cycle() {
		let before = lead_blend(0.49);
		let after = lead_blend(0.51);
		assert!((before - after).abs() < 0.15, "lead should not jump at half-cycle");
	}

	#[test]
	fn gallop_spine_is_continuous_between_adjacent_samples() {
		let gallop = QuadrupedGallop::<QuadrupedV0Rig>::default();
		let mut rig_prev = QuadrupedV0Rig::imported();
		gallop.apply(&mut rig_prev, 0.0);
		let mut prev = rig_prev
			.pose()
			.get(&rig_prev.spine().lumbar.name)
			.expect("lumbar")
			.flex;

		for step in 1..=200 {
			let phase = step as f32 / 200.0;
			let mut rig = QuadrupedV0Rig::imported();
			gallop.apply(&mut rig, phase);
			let flex = rig.pose().get(&rig.spine().lumbar.name).expect("lumbar").flex;
			let delta = (flex - prev).abs();
			assert!(
				delta < 0.08,
				"spine jerk at phase {phase}: delta={delta} prev={prev} flex={flex}"
			);
			prev = flex;
		}
	}

	#[test]
	fn gallop_bound_mix_crosses_hind_to_front_within_half_cycle() {
		let early = smooth_blend(BOUND_MIX_START, BOUND_MIX_END, 0.08);
		let late = smooth_blend(BOUND_MIX_START, BOUND_MIX_END, 0.50);
		assert!(early < late);
		assert!(early < 0.3);
		assert!(late > 0.7);
	}
}
