//! Bare-handed combat: trigger edges → [`AttackState`] → `Jab` clip + strike event.

mod aim;
mod attack;
mod drive;

use bevy::prelude::*;
use characters::CharacterMotionSystems;
use player::PlayerSystems;

pub use aim::{
	body_camera_yaw, quantize, quantize_clamped, JabAim, AIM_STEP, PITCH_MAX, PITCH_MIN, YAW_LIMIT,
};
pub use attack::{AttackKind, AttackPhase, AttackSpec, AttackState, AttackTiming, JAB_STARTUP_END};
pub use character_rigs::Side;

/// Player-side fisticuffs machine. Inserted on every [`player::Player`].
#[derive(Component, Clone, Debug, Default)]
pub struct FisticuffsUser {
	pub state: AttackState,
}

/// Gameplay event at the start of the active (hit) window.
///
/// `target` is the same quantized character-space point written to
/// `JabParams` (`+Z` fight-forward). Future punch volumes should use it.
///
/// TODO: resolve a punch volume against `damage::Health` and write
/// `damage::Hit { target, source: Some(attacker), amount, point, allow_self: false }`.
/// [`player::HitVolume`] / `HitCapsule` are receive-only; there is no fist sensor yet.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct FisticuffsStrike {
	pub attacker: Entity,
	pub side: Side,
	pub kind: AttackKind,
	pub progress: f32,
	/// Quantized character-space aim (`+X` right, `+Y` up, `+Z` forward).
	pub target: Vec3,
}

pub struct FisticuffsUserPlugin;

impl Plugin for FisticuffsUserPlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<FisticuffsStrike>()
			.add_systems(
				Update,
				(
					drive::ensure_fisticuffs_user,
					drive::tick_attack_state,
					drive::apply_attack_intents,
					drive::latch_jab_aim,
				)
					.chain()
					.in_set(PlayerSystems::Intent),
			)
			.add_systems(
				Update,
				drive::apply_fisticuffs_visual
					.after(PlayerSystems::Locomotion)
					.before(CharacterMotionSystems::Anim),
			);
	}
}

#[cfg(test)]
mod pose_tests {
	use bevy::prelude::Vec3;
	use character_animations::animations::{Jab, HOLD_END};
	use character_animations::Animation;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use character_rigs::Side;
	use maybraid_character_controller::produce::collect;
	use maybraid_character_controller::CharacterIntent;
	use maybraid_input::VirtualPad;

	use super::*;

	const FOREARM_SEGMENT: f32 = 0.48;
	const HOLD: f32 = (character_animations::animations::EXTEND_END + HOLD_END) * 0.5;

	fn finish(pad: &mut VirtualPad) {
		pad.apply_trigger_digital(0.5);
		pad.finish_digital();
	}

	fn attack_from_trigger(focus: f32, fire: f32) -> CharacterIntent {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.max_triggers(focus, fire);
		finish(&mut pad);
		collect(&pad, 0.5, 0.0, true)
			.into_iter()
			.find_map(|intent| match intent {
				CharacterIntent::AttackLeft | CharacterIntent::AttackRight => Some(intent),
				_ => None,
			})
			.expect("trigger edge should produce an attack intent")
	}

	fn forearm_hand_tip(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		let name = match side {
			Side::Left => "forearm.L",
			Side::Right => "forearm.R",
		};
		rig.character_point(name) + rig.character_length(name) * FOREARM_SEGMENT
	}

	fn pose_jab(side: Side) -> (Vec3, Vec3) {
		pose_jab_aim(side, JabAim::level())
	}

	fn pose_jab_aim(side: Side, aim: JabAim) -> (Vec3, Vec3) {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Jab::default().with_side(side).with_target(aim.target).apply(&mut rig, HOLD);
		(forearm_hand_tip(&rig, side), forearm_hand_tip(&rig, side.opposite()))
	}

	fn assert_tip(label: &str, got: Vec3, expected: Vec3) {
		assert!((got - expected).length() < 1e-3, "{label} tip {got} expected {expected}");
	}

	#[test]
	fn left_trigger_extends_left_hand_forward_on_plus_x() -> anyhow::Result<()> {
		let intent = attack_from_trigger(0.9, 0.0);
		assert_eq!(intent, CharacterIntent::AttackLeft);
		let spec = AttackSpec::from_intent(intent).expect("left attack");
		assert_eq!(spec.side, Side::Left);

		let (punch, cover) = pose_jab(spec.side);
		assert!(punch.z > 1.2, "left hand should extend forward (+Z), got {punch:?}");
		assert!(punch.x > 0.0, "left hand should sit on the +X side, got {punch:?}");
		assert!(
			punch.z > cover.z + 0.5,
			"cover hand should stay back, punch {punch:?} cover {cover:?}"
		);
		Ok(())
	}

	#[test]
	fn right_trigger_extends_right_hand_forward_on_minus_x() -> anyhow::Result<()> {
		let intent = attack_from_trigger(0.0, 0.9);
		assert_eq!(intent, CharacterIntent::AttackRight);
		let spec = AttackSpec::from_intent(intent).expect("right attack");
		assert_eq!(spec.side, Side::Right);

		let (punch, cover) = pose_jab(spec.side);
		assert!(punch.z > 1.2, "right hand should extend forward (+Z), got {punch:?}");
		assert!(punch.x < 0.0, "right hand should sit on the -X side, got {punch:?}");
		assert!(
			punch.z > cover.z + 0.5,
			"cover hand should stay back, punch {punch:?} cover {cover:?}"
		);
		Ok(())
	}

	#[test]
	fn look_pitch_raises_and_drops_both_hand_tips_on_their_own_side() -> anyhow::Result<()> {
		for side in [Side::Left, Side::Right] {
			let (level, _) = pose_jab_aim(side, JabAim::level());
			let (up, _) = pose_jab_aim(side, JabAim::from_camera_angles(PITCH_MAX, 0.0));
			let (down, _) = pose_jab_aim(side, JabAim::from_camera_angles(PITCH_MIN, 0.0));
			assert!(up.y > level.y + 0.04, "{side:?} up {up:?} level {level:?}");
			assert!(down.y < level.y - 0.03, "{side:?} down {down:?} level {level:?}");
			match side {
				Side::Left => {
					assert!(
						level.x > 0.0 && up.x > 0.0 && down.x > 0.0,
						"left crossed {level:?} {up:?} {down:?}"
					);
				}
				Side::Right => {
					assert!(
						level.x < 0.0 && up.x < 0.0 && down.x < 0.0,
						"right crossed {level:?} {up:?} {down:?}"
					);
				}
			}
		}
		Ok(())
	}

	#[test]
	fn look_yaw_shifts_tips_without_crossing_the_midline() -> anyhow::Result<()> {
		for side in [Side::Left, Side::Right] {
			let (level, _) = pose_jab_aim(side, JabAim::level());
			let (look_left, _) = pose_jab_aim(side, JabAim::from_camera_angles(0.0, YAW_LIMIT));
			let (look_right, _) = pose_jab_aim(side, JabAim::from_camera_angles(0.0, -YAW_LIMIT));
			assert!(
				look_left.x < level.x - 0.02,
				"{side:?} look-left {look_left:?} level {level:?}"
			);
			assert!(
				look_right.x > level.x + 0.02,
				"{side:?} look-right {look_right:?} level {level:?}"
			);
			match side {
				Side::Left => {
					assert!(
						look_left.x > 0.0 && look_right.x > 0.0,
						"left crossed {look_left:?} {look_right:?}"
					);
				}
				Side::Right => {
					assert!(
						look_left.x < 0.0 && look_right.x < 0.0,
						"right crossed {look_left:?} {look_right:?}"
					);
				}
			}
		}
		Ok(())
	}

	#[test]
	fn active_hand_tips_match_look_cone_table() -> anyhow::Result<()> {
		// `HumanoidV0Rig::for_clip_test()` punching-hand tips at HOLD (Active).
		let cases = [
			(Side::Left, "level", JabAim::level(), Vec3::new(0.1123, 0.4747, 1.4086)),
			(
				Side::Left,
				"up",
				JabAim::from_camera_angles(PITCH_MAX, 0.0),
				Vec3::new(0.1829, 0.5763, 1.3041),
			),
			(
				Side::Left,
				"down",
				JabAim::from_camera_angles(PITCH_MIN, 0.0),
				Vec3::new(0.1140, 0.3756, 1.3668),
			),
			(
				Side::Left,
				"yaw_left",
				JabAim::from_camera_angles(0.0, YAW_LIMIT),
				Vec3::new(0.0730, 0.4930, 1.3201),
			),
			(
				Side::Left,
				"yaw_right",
				JabAim::from_camera_angles(0.0, -YAW_LIMIT),
				Vec3::new(0.1932, 0.4646, 1.4058),
			),
			(Side::Right, "level", JabAim::level(), Vec3::new(-0.0424, 0.4521, 1.4488)),
			(
				Side::Right,
				"up",
				JabAim::from_camera_angles(PITCH_MAX, 0.0),
				Vec3::new(-0.0612, 0.5342, 1.4132),
			),
			(
				Side::Right,
				"down",
				JabAim::from_camera_angles(PITCH_MIN, 0.0),
				Vec3::new(-0.0950, 0.3690, 1.3803),
			),
			(
				Side::Right,
				"yaw_left",
				JabAim::from_camera_angles(0.0, YAW_LIMIT),
				Vec3::new(-0.1202, 0.4448, 1.4419),
			),
			(
				Side::Right,
				"yaw_right",
				JabAim::from_camera_angles(0.0, -YAW_LIMIT),
				Vec3::new(-0.0159, 0.4688, 1.3763),
			),
		];
		for (side, label, aim, expected) in cases {
			let (punch, _) = pose_jab_aim(side, aim);
			assert_tip(&format!("{side:?} {label}"), punch, expected);
		}
		Ok(())
	}
}
