//! Ordinary locomotion: addressed character frames → actor-local [`player::MotorIntent`].

pub mod standard;

pub use standard::interpret_standard_locomotion;

use bevy::prelude::*;
use maybraid_controllers_core::ControllersCorePlugin;
use player::MotorRequestStage;

/// Writes each controlled actor's Standard motor slot from its addressed frame.
pub struct LocomotionControllerPlugin;

impl Plugin for LocomotionControllerPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ControllersCorePlugin>() {
			app.add_plugins(ControllersCorePlugin);
		}
		MotorRequestStage::configure(app);
		app.add_systems(Update, interpret_standard_locomotion.in_set(MotorRequestStage::Standard));
	}
}
