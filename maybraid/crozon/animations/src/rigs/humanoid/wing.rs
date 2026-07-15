//! Shared biped wing-spread pose used by soaring and flapping.
//!
//! The humanoid rest pose is a T-pose (arms already out along ±X), so the held
//! flight pose should not add much shoulder flex about Bevy Z. Wing beats are a
//! front/back stroke about Bevy Y (shoulder swing), with a modest static Y bias
//! angling each shoulder away from the spine.

use crozon_rigs::{humanoid::HumanoidRig, Side};

use crate::rigs::humanoid::apply::{apply_arm, apply_leg, apply_root};

/// Held shoulder swing about Bevy Y: angle the wing root away from the spine.
pub(crate) const SOAR_SHOULDER_SWING: f32 = 0.35;
/// T-pose already spreads the arms; keep held Z flex near zero.
pub(crate) const SOAR_SHOULDER_FLEX: f32 = 0.0;
pub(crate) const SOAR_HUMERUS_SWING: f32 = 0.08;
pub(crate) const FOREARM_EXTEND: f32 = -0.05;
pub(crate) const LEG_TRAIL: f32 = -0.22;
pub(crate) const KNEE_SOFT: f32 = 0.28;
pub(crate) const ROOT_LEAN: f32 = -0.12;

/// Flap stroke about Bevy Y (front/back), not Z (up/down).
pub(crate) const FLAP_SHOULDER_SWING_AMP: f32 = 0.4;
pub(crate) const FLAP_HUMERUS_AMP: f32 = 0.22;
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

/// Hold a T-pose-relative wing spread with optional front/back flap modulation.
///
/// `flap_amount` is typically in `[-range, range]`; negative is the rearward stroke.
pub(crate) fn apply_flight_wings<R: HumanoidRig>(rig: &mut R, flap_amount: f32) {
	let shoulder_swing = SOAR_SHOULDER_SWING + FLAP_SHOULDER_SWING_AMP * flap_amount;
	let humerus_swing = SOAR_HUMERUS_SWING + FLAP_HUMERUS_AMP * flap_amount;
	let forearm_flex = FOREARM_EXTEND + FLAP_ELBOW_AMP * flap_amount.max(0.0);

	for side in [Side::Left, Side::Right] {
		let lateral = lateral_sign(side);
		apply_arm(
			rig,
			side,
			// Bevy Y: held spine angle + front/back wing beat.
			shoulder_swing * lateral,
			// Bevy Z: leave near rest — T-pose already holds the lateral spread.
			SOAR_SHOULDER_FLEX,
			humerus_swing * -lateral,
			0.0,
			forearm_flex,
		);
	}
}
