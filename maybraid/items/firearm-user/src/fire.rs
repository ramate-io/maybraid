//! Copy use-item onto the held gun's [`WeaponTrigger`].

use bevy::prelude::*;
use firearms::{WeaponFired, WeaponTrigger};
use maybraid_character_controller::CharacterIntent;

use player::{Player, PlayerLook};

use crate::FirearmUser;

pub(crate) fn apply_fire_intents(
	mouse: Res<ButtonInput<MouseButton>>,
	mut intents: MessageReader<CharacterIntent>,
	users: Query<&FirearmUser, With<Player>>,
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

pub(crate) fn apply_weapon_recoil(
	mut fired: MessageReader<WeaponFired>,
	mut looks: Query<&mut PlayerLook>,
) {
	for event in fired.read() {
		if event.recoil <= 0.0 {
			continue;
		}
		if let Ok(mut look) = looks.get_mut(event.shooter) {
			look.pitch += event.recoil;
		}
	}
}
