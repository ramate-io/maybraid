//! Training Ground is a seeded FinePatch of the Maybraid world.
//!
//! The shell sets [`TrainingGroundActive`]. [`maybraid_world::TrainingGrounds`]
//! retargets Durham, the forest, and hopscotch. A Training pose is not written.

use bevy::prelude::*;

/// The game shell sets this while Training Ground is the live world session.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrainingGroundActive(pub bool);

pub struct TrainingGroundPlugin;

impl Plugin for TrainingGroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<TrainingGroundActive>();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn plugin_starts_inactive() {
		let mut app = App::new();
		app.add_plugins(TrainingGroundPlugin);
		assert!(!app.world().resource::<TrainingGroundActive>().0);
	}
}
