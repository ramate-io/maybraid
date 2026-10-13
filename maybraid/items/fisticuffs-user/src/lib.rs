//! Bare-handed combat: trigger edges → [`AttackState`] → `Jab` clip + strike event.

mod attack;
mod drive;

use bevy::prelude::*;
use characters::CharacterMotionSystems;
use player::PlayerSystems;

pub use attack::{AttackKind, AttackPhase, AttackSpec, AttackState, AttackTiming};
pub use character_rigs::Side;

/// Player-side fisticuffs machine. Inserted on every [`player::Player`].
#[derive(Component, Clone, Debug, Default)]
pub struct FisticuffsUser {
	pub state: AttackState,
}

/// Gameplay event at the start of the active (hit) window.
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
		let mut rig = HumanoidV0Rig::for_clip_test();
		Jab::default().with_side(side).apply(&mut rig, HOLD);
		(forearm_hand_tip(&rig, side), forearm_hand_tip(&rig, side.opposite()))
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
}
