//! [`VirtualPad`] → [`CharacterIntent`].

use bevy::prelude::*;
use maybraid_input::{PadButton, VirtualPad, VirtualPadConfig};

use crate::intent::CharacterIntent;

const ANALOG_EPS: f32 = 1e-6;

pub fn produce_character_intents(
	pad: Res<VirtualPad>,
	config: Res<VirtualPadConfig>,
	mut intents: MessageWriter<CharacterIntent>,
) {
	for intent in collect(&pad, config.trigger_press_threshold) {
		intents.write(intent);
	}
}

pub fn collect(pad: &VirtualPad, trigger_threshold: f32) -> Vec<CharacterIntent> {
	let mut out = Vec::new();
	if pad.move_stick.length_squared() > ANALOG_EPS {
		out.push(CharacterIntent::Move(pad.move_stick));
	}
	if pad.look_stick.length_squared() > ANALOG_EPS {
		out.push(CharacterIntent::Look(pad.look_stick));
	}
	if pad.trigger_focus > ANALOG_EPS {
		out.push(CharacterIntent::Focus(pad.trigger_focus));
	}
	if pad.trigger_fire > ANALOG_EPS {
		out.push(CharacterIntent::UseItem(pad.trigger_fire));
	}

	if pad.just_pressed(PadButton::StickClickMove) {
		out.push(CharacterIntent::StartSprint);
	}
	if pad.just_released(PadButton::StickClickMove) {
		out.push(CharacterIntent::StopSprint);
	}
	if pad.just_pressed(PadButton::StickClickLook) {
		out.push(CharacterIntent::SwapPov);
	}
	if pad.just_pressed(PadButton::A) {
		out.push(CharacterIntent::Jump);
	}
	if pad.just_pressed(PadButton::B) {
		out.push(CharacterIntent::ExitInteraction);
	}
	if pad.just_pressed(PadButton::X) {
		if pad.pressed(PadButton::TriggerFire) || pad.trigger_fire >= trigger_threshold {
			out.push(CharacterIntent::PowerUseItem);
		} else {
			out.push(CharacterIntent::StartInteraction);
		}
	}
	if pad.just_pressed(PadButton::Y) {
		out.push(CharacterIntent::SwapActive);
	}
	if pad.just_pressed(PadButton::Start) {
		out.push(CharacterIntent::InGameMenu);
	}
	if pad.just_pressed(PadButton::Select) {
		out.push(CharacterIntent::Inventory);
	}
	out
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_input::VirtualPad;

	fn finish(pad: &mut VirtualPad) {
		pad.finish_digital();
	}

	#[test]
	fn move_and_look_are_analog() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.add_move(Vec2::new(1.0, 0.0));
		pad.add_look(Vec2::new(0.0, -0.5));
		finish(&mut pad);
		let intents = collect(&pad, 0.5);
		assert_eq!(intents[0], CharacterIntent::Move(Vec2::new(1.0, 0.0)));
		assert_eq!(intents[1], CharacterIntent::Look(Vec2::new(0.0, -0.5)));
		Ok(())
	}

	#[test]
	fn l3_hold_is_sprint_edges() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::StickClickMove);
		finish(&mut pad);
		assert_eq!(collect(&pad, 0.5), vec![CharacterIntent::StartSprint]);

		pad.begin_frame();
		finish(&mut pad);
		assert_eq!(collect(&pad, 0.5), vec![CharacterIntent::StopSprint]);
		Ok(())
	}

	#[test]
	fn r3_click_swaps_pov() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::StickClickLook);
		finish(&mut pad);
		assert_eq!(collect(&pad, 0.5), vec![CharacterIntent::SwapPov]);
		Ok(())
	}

	#[test]
	fn face_and_menu_buttons() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::A);
		pad.hold_digital(PadButton::B);
		pad.hold_digital(PadButton::Y);
		pad.hold_digital(PadButton::Start);
		pad.hold_digital(PadButton::Select);
		finish(&mut pad);
		assert_eq!(
			collect(&pad, 0.5),
			vec![
				CharacterIntent::Jump,
				CharacterIntent::ExitInteraction,
				CharacterIntent::SwapActive,
				CharacterIntent::InGameMenu,
				CharacterIntent::Inventory,
			]
		);
		Ok(())
	}

	#[test]
	fn x_without_rt_starts_interaction() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.hold_digital(PadButton::X);
		finish(&mut pad);
		assert_eq!(collect(&pad, 0.5), vec![CharacterIntent::StartInteraction]);
		Ok(())
	}

	#[test]
	fn rt_plus_x_is_power_use_not_interact() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.max_triggers(0.0, 0.9);
		pad.apply_trigger_digital(0.5);
		pad.hold_digital(PadButton::X);
		finish(&mut pad);
		let intents = collect(&pad, 0.5);
		assert!(intents.contains(&CharacterIntent::UseItem(0.9)));
		assert!(intents.contains(&CharacterIntent::PowerUseItem));
		assert!(!intents.contains(&CharacterIntent::StartInteraction));
		Ok(())
	}

	#[test]
	fn left_trigger_is_focus() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.max_triggers(0.4, 0.0);
		finish(&mut pad);
		assert_eq!(collect(&pad, 0.5), vec![CharacterIntent::Focus(0.4)]);
		Ok(())
	}
}
