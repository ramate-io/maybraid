//! Drops [`crate::commands::render::Render`] announcers after leaf helper reactors run.

use bevy::prelude::*;

use crate::commands::render::Render;

pub fn despawn_render_command_announcer(mut commands: Commands, q: Query<Entity, Added<Render>>) {
	for entity in &q {
		commands.entity(entity).despawn();
	}
}
