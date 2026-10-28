//! Bare-handed combat: trigger edges → [`AttackState`] → `Jab` clip + strike / impact events.

mod aim;
mod attack;
mod drive;
mod runtime;
mod sensor;
mod sound;

use bevy::prelude::*;
use characters::CharacterMotionSystems;
use player::PlayerSystems;

pub use aim::{
	body_camera_yaw, quantize, quantize_clamped, JabAim, AIM_STEP, PITCH_MAX, PITCH_MIN, YAW_LIMIT,
};
pub use attack::{AttackKind, AttackPhase, AttackSpec, AttackState, AttackTiming, JAB_STARTUP_END};
pub use character_rigs::Side;
pub use drive::apply_fisticuffs_visual;
pub use runtime::{register_fisticuffs_user_runtime, teardown_fisticuffs};
pub use sensor::{
	character_to_world, draw_fist_sensors, fist_collider, fist_local, fist_world_rotation,
	resolve_fist_hits, sync_fist_sensors, FisticuffsHitSensor, FisticuffsHitSensorOf,
	FisticuffsSensorDebug, PLACEHOLDER_FIST_RADIUS, PLACEHOLDER_FIST_REACH,
	PLACEHOLDER_FOREARM_LENGTH, PLACEHOLDER_JAB_DAMAGE,
};

/// Player-side fisticuffs machine. Inserted by equipment when Fisticuffs is the
/// active primary item; removed when another tool is selected.
#[derive(Component, Clone, Debug, Default)]
pub struct FisticuffsUser {
	pub state: AttackState,
	pending_attacks: Vec<maybraid_controllers_core::Routed<AttackSpec>>,
}

/// Gameplay event at the start of the active (hit) window.
///
/// `target` is the same quantized character-space point written to
/// `JabParams` (`+Z` fight-forward). The fist sensor is posed from the
/// attacker, heading, and latched [`JabAim`]; it writes [`damage::Hit`]
/// with [`PLACEHOLDER_JAB_DAMAGE`] (`allow_self: false`).
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct FisticuffsStrike {
	pub attacker: Entity,
	pub side: Side,
	pub kind: AttackKind,
	/// Rotates while the player chains jabs (`0 .. COMBO_STEP_COUNT-1`).
	/// Clip choice uses the shared flinch bank, not this step.
	pub combo_step: u8,
	pub progress: f32,
	/// Quantized character-space aim (`+X` character left, `+Y` up, `+Z` forward).
	pub target: Vec3,
}

/// The first solid surface reached by a jab's fist-volume sweep.
///
/// `collider` is the physical collider that stopped the jab. `target` is the
/// collider itself or its health-owning parent when damage can be applied.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct FisticuffsImpact {
	pub attacker: Entity,
	pub side: Side,
	pub collider: Entity,
	pub target: Option<Entity>,
	pub point: Vec3,
	pub normal: Vec3,
}

pub struct FisticuffsUserPlugin;

/// Fisticuffs presentation timing, so a composing application can set its
/// priority against other full-body action overrides.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FisticuffsUserSystems {
	Animate,
}

impl Plugin for FisticuffsUserPlugin {
	fn build(&self, app: &mut App) {
		register_fisticuffs_user_runtime(app);
		sound::configure_strike_audio(app);
		app.init_resource::<FisticuffsSensorDebug>()
			.configure_sets(
				Update,
				FisticuffsUserSystems::Animate
					.after(PlayerSystems::Locomotion)
					.before(CharacterMotionSystems::Anim),
			)
			.add_systems(
				Update,
				drive::apply_fisticuffs_visual.in_set(FisticuffsUserSystems::Animate),
			)
			.add_systems(
				Update,
				sensor::draw_fist_sensors.run_if(resource_equals(FisticuffsSensorDebug(true))),
			);
	}
}

#[cfg(test)]
mod lifetime;

#[cfg(test)]
mod pose_tests {
	use bevy::prelude::Vec3;
	use character_animations::animations::{Jab, JAB_HOLD_END};
	use character_animations::Animation;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
	use character_rigs::Side;
	use maybraid_character_controller::produce::collect;
	use maybraid_character_controller::CharacterIntent;
	use maybraid_virtual_pad_controller::VirtualPad;

	use super::*;

	const FOREARM_SEGMENT: f32 = 0.48;
	const HOLD: f32 = (character_animations::animations::EXTEND_END + JAB_HOLD_END) * 0.5;

	fn finish(pad: &mut VirtualPad) {
		pad.apply_trigger_digital(0.5);
		pad.finish_digital();
	}

	fn attack_from_trigger(focus: f32, fire: f32) -> CharacterIntent {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.max_triggers(focus, fire);
		finish(&mut pad);
		collect(&pad, 0.5, 0.0, true, true, false)
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

	fn pose_rig(side: Side, aim: JabAim) -> HumanoidV0Rig {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Jab::default().with_side(side).with_target(aim.target).apply(&mut rig, HOLD);
		rig
	}

	fn pose_jab_aim(side: Side, aim: JabAim) -> (Vec3, Vec3) {
		let rig = pose_rig(side, aim);
		(forearm_hand_tip(&rig, side), forearm_hand_tip(&rig, side.opposite()))
	}

	fn shoulder(rig: &HumanoidV0Rig, side: Side) -> Vec3 {
		let name = match side {
			Side::Left => "humerus.L",
			Side::Right => "humerus.R",
		};
		rig.character_point(name)
	}

	fn yaw_pitch(dir: Vec3) -> (f32, f32) {
		let yaw = dir.x.atan2(dir.z);
		let pitch = dir.y.atan2(Vec3::new(dir.x, 0.0, dir.z).length());
		(yaw, pitch)
	}

	fn shoulder_to_tip(side: Side, aim: JabAim) -> (Vec3, Vec3, f32, f32) {
		let rig = pose_rig(side, aim);
		let tip = forearm_hand_tip(&rig, side);
		let dir = tip - shoulder(&rig, side);
		let (yaw, pitch) = yaw_pitch(dir);
		(tip, dir, yaw, pitch)
	}

	const AIM_ANGLE_SLACK: f32 = 8.0 * std::f32::consts::PI / 180.0;

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
			assert!(up.y > level.y + 0.35, "{side:?} up {up:?} level {level:?}");
			assert!(down.y < level.y - 0.25, "{side:?} down {down:?} level {level:?}");
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
	fn look_left_is_character_plus_x_from_the_player_view() -> anyhow::Result<()> {
		// Camera +yaw = player looks left. Posed +X is the left-hand side.
		for side in [Side::Left, Side::Right] {
			let (level, _, level_yaw, _) = shoulder_to_tip(side, JabAim::level());
			let (look_left, _, left_yaw, _) =
				shoulder_to_tip(side, JabAim::from_camera_angles(0.0, YAW_LIMIT));
			let (look_right, _, right_yaw, _) =
				shoulder_to_tip(side, JabAim::from_camera_angles(0.0, -YAW_LIMIT));
			assert!(
				look_left.x > level.x + 0.08,
				"{side:?} look-left must move toward +X, {look_left:?} vs {level:?}"
			);
			assert!(
				look_right.x < level.x - 0.08,
				"{side:?} look-right must move toward −X, {look_right:?} vs {level:?}"
			);
			assert!(
				left_yaw > level_yaw + 0.15,
				"{side:?} look-left yaw {left_yaw} vs {level_yaw}"
			);
			assert!(
				right_yaw < level_yaw - 0.15,
				"{side:?} look-right yaw {right_yaw} vs {level_yaw}"
			);
		}
		Ok(())
	}

	#[test]
	fn level_fists_sit_outboard_of_the_midline() -> anyhow::Result<()> {
		let (left, _) = pose_jab(Side::Left);
		let (right, _) = pose_jab(Side::Right);
		assert!(left.x > 0.12, "left level should sit outboard on +X, {left:?}");
		assert!(right.x < -0.08, "right level should sit outboard on −X, {right:?}");
		Ok(())
	}

	#[test]
	fn active_fist_tracks_look_angle_within_eight_degrees() -> anyhow::Result<()> {
		let cases = [
			("level", JabAim::level()),
			("up", JabAim::from_camera_angles(PITCH_MAX, 0.0)),
			("down", JabAim::from_camera_angles(PITCH_MIN, 0.0)),
			("yaw_left", JabAim::from_camera_angles(0.0, YAW_LIMIT)),
			("yaw_right", JabAim::from_camera_angles(0.0, -YAW_LIMIT)),
		];
		for side in [Side::Left, Side::Right] {
			for (label, aim) in cases {
				let (tip, _, yaw, pitch) = shoulder_to_tip(side, aim);
				let yaw_err = (yaw - aim.yaw).abs();
				let pitch_err = (pitch - aim.pitch).abs();
				assert!(
					yaw_err < AIM_ANGLE_SLACK,
					"{side:?} {label} yaw err {:.1}° (got {:.1} want {:.1}) tip={tip:?}",
					yaw_err.to_degrees(),
					yaw.to_degrees(),
					aim.yaw.to_degrees()
				);
				assert!(
					pitch_err < AIM_ANGLE_SLACK,
					"{side:?} {label} pitch err {:.1}° (got {:.1} want {:.1}) tip={tip:?}",
					pitch_err.to_degrees(),
					pitch.to_degrees(),
					aim.pitch.to_degrees()
				);
			}
			let (level, _) = pose_jab(side);
			let (up, _) = pose_jab_aim(side, JabAim::from_camera_angles(PITCH_MAX, 0.0));
			let (down, _) = pose_jab_aim(side, JabAim::from_camera_angles(PITCH_MIN, 0.0));
			match side {
				Side::Left => {
					assert!(level.x > 0.12 && up.x > 0.08 && down.x > 0.08, "left {level:?}");
				}
				Side::Right => {
					assert!(level.x < -0.08 && up.x < -0.08 && down.x < -0.08, "right {level:?}");
				}
			}
		}
		Ok(())
	}

	#[test]
	fn active_hand_tips_match_look_cone_table() -> anyhow::Result<()> {
		let cases = [
			(Side::Left, "level", JabAim::level(), Vec3::new(0.1674, 0.6390, 1.4267)),
			(
				Side::Left,
				"up",
				JabAim::from_camera_angles(PITCH_MAX, 0.0),
				Vec3::new(0.2566, 1.2431, 1.1420),
			),
			(
				Side::Left,
				"down",
				JabAim::from_camera_angles(PITCH_MIN, 0.0),
				Vec3::new(0.1722, 0.2643, 1.3289),
			),
			(
				Side::Left,
				"yaw_left",
				JabAim::from_camera_angles(0.0, YAW_LIMIT),
				Vec3::new(0.5419, 0.6437, 1.3504),
			),
			(
				Side::Left,
				"yaw_right",
				JabAim::from_camera_angles(0.0, -YAW_LIMIT),
				Vec3::new(-0.1537, 0.6699, 1.2625),
			),
			(Side::Right, "level", JabAim::level(), Vec3::new(-0.0973, 0.6169, 1.4668)),
			(
				Side::Right,
				"up",
				JabAim::from_camera_angles(PITCH_MAX, 0.0),
				Vec3::new(-0.1314, 1.2039, 1.2480),
			),
			(
				Side::Right,
				"down",
				JabAim::from_camera_angles(PITCH_MIN, 0.0),
				Vec3::new(-0.1532, 0.2577, 1.3423),
			),
			(
				Side::Right,
				"yaw_left",
				JabAim::from_camera_angles(0.0, YAW_LIMIT),
				Vec3::new(0.2109, 0.6465, 1.3185),
			),
			(
				Side::Right,
				"yaw_right",
				JabAim::from_camera_angles(0.0, -YAW_LIMIT),
				Vec3::new(-0.4687, 0.6242, 1.3866),
			),
		];
		for (side, label, aim, expected) in cases {
			let (punch, _) = pose_jab_aim(side, aim);
			assert!(
				(punch - expected).length() < 1e-3,
				"{side:?} {label} tip {punch} expected {expected}"
			);
		}
		Ok(())
	}
}
