//! Shared spawn path for full-screen menu roots.

use bevy::prelude::*;

/// If any request entities exist, despawn every [`crate::MenuScreen`] and those
/// requests. Returns whether the caller should spawn a replacement.
pub fn take_menu_show_request(
	commands: &mut Commands,
	request_entities: impl IntoIterator<Item = Entity>,
	existing: impl IntoIterator<Item = Entity>,
) -> bool {
	let requests: Vec<Entity> = request_entities.into_iter().collect();
	if requests.is_empty() {
		return false;
	}
	for entity in existing {
		commands.entity(entity).despawn();
	}
	for entity in requests {
		commands.entity(entity).despawn();
	}
	true
}
