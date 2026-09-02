//! Create-a-character flow: three random garments, then the body editor.

use bevy::prelude::*;
use crozon_character_items::{
	InventoryItem, ItemRng, STARTER_CLOTHING_COUNT, random_starter_clothing,
};

use crate::spin_reveal::{
	SpinRevealFinished, SpinRevealScreenPlugin, SpinRevealSystems, request_show_spin_reveal,
};

/// Queue the create-a-character flow (starter reveal, then the body HUD).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestShowCreateCharacter;

/// Starter inventory is ready; the host should open the create-mode character HUD.
#[derive(Message, Clone, Debug)]
pub struct CreateCharacterReady {
	pub items: Vec<InventoryItem>,
}

pub fn request_show_create_character(commands: &mut Commands) {
	commands.spawn(RequestShowCreateCharacter);
}

pub struct CreateCharacterPlugin;

impl Plugin for CreateCharacterPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SpinRevealScreenPlugin>() {
			app.add_plugins(SpinRevealScreenPlugin);
		}
		app.add_message::<CreateCharacterReady>().add_systems(
			Update,
			(
				start_create_character.before(SpinRevealSystems::Apply),
				forward_create_character_ready.after(SpinRevealSystems::Apply),
			),
		);
	}
}

fn start_create_character(
	mut commands: Commands,
	requests: Query<Entity, With<RequestShowCreateCharacter>>,
) {
	if requests.is_empty() {
		return;
	}
	for entity in &requests {
		commands.entity(entity).despawn();
	}
	let items = random_starter_clothing(&mut ItemRng::from_entropy(), STARTER_CLOTHING_COUNT);
	request_show_spin_reveal(&mut commands, items);
}

fn forward_create_character_ready(
	mut finished: MessageReader<SpinRevealFinished>,
	mut ready: MessageWriter<CreateCharacterReady>,
) {
	for event in finished.read() {
		ready.write(CreateCharacterReady { items: event.items.clone() });
	}
}
