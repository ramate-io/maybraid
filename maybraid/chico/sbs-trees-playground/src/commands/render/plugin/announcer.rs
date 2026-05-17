use bevy::prelude::*;

use crate::commands::render::Render;

pub fn despawn_render_command_announcer(mut commands: Commands, q: Query<Entity, Added<Render>>) {
	for e in &q {
		commands.entity(e).despawn();
	}
}
