//! Training Ground is a seeded FinePatch of the Maybraid world.
//!
//! The shell requests Training on the world's mode. This plugin is the mode
//! marker; sources register here in later issues.

use bevy::prelude::*;

/// Marker so the executable loads this mode beside the others.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct TrainingGroundMode;

pub struct TrainingGroundPlugin;

impl Plugin for TrainingGroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<TrainingGroundMode>();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn plugin_inserts_its_marker() {
		let mut app = App::new();
		app.add_plugins(TrainingGroundPlugin);
		assert!(app.world().get_resource::<TrainingGroundMode>().is_some());
	}
}
