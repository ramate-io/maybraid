//! Fixture encounter instances. World placement is [#1117](https://github.com/ramate-io/maybraid/issues/1117).

use bevy::prelude::*;
use damage::Health;

use crate::identity::EncounterId;
use crate::instance::EncounterInstance;
use crate::plugin::EncountersPlugin;

/// Register the progress owner without world layers or HUD.
pub fn install(app: &mut App) {
	if !app.is_plugin_added::<EncountersPlugin>() {
		app.add_plugins(EncountersPlugin);
	}
}

/// Spawn a health-bearing stand-in keyed by `id`.
pub fn spawn_instance(world: &mut World, id: EncounterId, max_health: f32) -> Entity {
	world.spawn((EncounterInstance::new(id), Health::from_max(max_health))).id()
}

/// Default 100 HP fixture for `id`.
pub fn spawn(world: &mut World, id: EncounterId) -> Entity {
	spawn_instance(world, id, 100.0)
}
