//! Copy use-item onto the held gun's [`WeaponTrigger`].

use bevy::prelude::*;
use firearms::WeaponTrigger;
use maybraid_character_controller::CharacterIntent;

use crate::FirearmUser;

pub(crate) fn apply_fire_intents(
	mouse: Res<ButtonInput<MouseButton>>,
	mut intents: MessageReader<CharacterIntent>,
	users: Query<&FirearmUser>,
	mut triggers: Query<&mut WeaponTrigger>,
) {
	let mut fire = mouse.pressed(MouseButton::Left);
	for intent in intents.read() {
		if let CharacterIntent::UseItem(_) = *intent {
			fire = true;
		}
	}
	for user in &users {
		if let Ok(mut trigger) = triggers.get_mut(user.held) {
			trigger.0 = fire;
		}
	}
}
