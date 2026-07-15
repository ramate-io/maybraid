//! Shared biped wing-spread pose used by soaring and flapping.
//!
//! The humanoid rest pose is a T-pose (arms already out along ±X), so the held
//! flight pose should not add much shoulder flex about Bevy Z. The detail that
//! sells the wing is a modest shoulder swing about Bevy Y — angling each
//! shoulder bone away from the spine — with flap modulating a small Z stroke
//! around that hold.

use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::rigs::humanoid::apply::{apply_arm, apply_leg, apply_root};

/// Shoulder swing about Bevy Y: angle the wing root away from the spine.
pub(crate) const SOAR_SHOULDER_SWING: f32 = 0.35;
/// T-pose already spreads the arms; keep held Z flex near zero.
pub(crate) const SOAR_SHOULDER_FLEX: f32 = 0.0;
pub(crate) const SOAR_HUMERUS_SWING: f32 = 0.08;
pub(crate) const FOREARM_EXTEND: f32 = -0.05;
pub(crate) const LEG_TRAIL: f32 = -0.22;
pub(crate) const KNEE_SOFT: f32 = 0.28;
pub(crate) const ROOT_LEAN: f32 = -0.12;

/// Flap stroke about Bevy Z around the T-pose hold (downstroke negative).
pub(crate) const FLAP_SHOULDER_FLEX_AMP: f32 = 0.32;
pub(crate) const FLAP_HUMERUS_AMP: f32 = 0.18;
pub(crate) const FLAP_ELBOW_AMP: f32 = 0.12;

/// Mirrored lateral sign for bones that share the same local axis metadata.
fn lateral_sign(side: Side) -> f32 {
	match side {
		Side::Left => 1.0,
		Side::Right => -1.0,
	}
}

/// Apply trailing legs + slight forward lean for a flight silhouette.
pub(crate) fn apply_flight_body<R: HumanoidRig>(rig: &mut R) {
	apply_root(rig, ROOT_LEAN);
	apply_leg(rig, Side::Left, LEG_TRAIL, KNEE_SOFT);
	apply_leg(rig, Side::Right, LEG_TRAIL, KNEE_SOFT);
}

/// Hold a T-pose-relative wing spread with optional flap modulation.
///
/// `flap_amount` is typically in `[-range, range]`; negative is downstroke.
pub(crate) fn apply_flight_wings<R: HumanoidRig>(rig: &mut R, flap_amount: f32) {
	let shoulder_flex = SOAR_SHOULDER_FLEX + FLAP_SHOULDER_FLEX_AMP * flap_amount;
	let humerus_swing = SOAR_HUMERUS_SWING + FLAP_HUMERUS_AMP * flap_amount;
	let forearm_flex = FOREARM_EXTEND + FLAP_ELBOW_AMP * flap_amount.max(0.0);

	for side in [Side::Left, Side::Right] {
		let lateral = lateral_sign(side);
		apply_arm(
			rig,
			side,
			// Bevy Y: angle each shoulder symmetrically away from the spine.
			SOAR_SHOULDER_SWING * lateral,
			// Bevy Z: small flap stroke only — T-pose already holds the spread.
			shoulder_flex * lateral,
			humerus_swing * -lateral,
			0.0,
			forearm_flex,
		);
	}
}
