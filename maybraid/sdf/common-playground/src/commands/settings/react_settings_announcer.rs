//! Drops [`super::Settings`] announcement entities after leaf settings reactors run.

use bevy::prelude::*;

use super::Settings;

pub fn despawn_settings_command_announcer(mut commands: Commands, q: Query<Entity, Added<Settings>>) {
	for entity in &q {
		commands.entity(entity).despawn();
	}
}
