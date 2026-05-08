//! Reacts to [`super::SettingsCheckerSize`] leaf commands.

use bevy::prelude::*;

use crate::checkerboard_material::CheckerboardMaterial;
use crate::ground::{GroundPlane, PlaygroundSettings};
use crate::input::CommandConsoleOutput;

#[derive(Component, Clone, Copy, Debug)]
pub struct SettingsCheckerSize {
	pub meters: f32,
}

pub fn react_settings_checker_size(
	mut commands: Commands,
	q: Query<(Entity, &SettingsCheckerSize), Added<SettingsCheckerSize>>,
	mut playground: ResMut<PlaygroundSettings>,
	mut materials: ResMut<Assets<CheckerboardMaterial>>,
	ground_mat_q: Query<&MeshMaterial3d<CheckerboardMaterial>, With<GroundPlane>>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for (entity, cmd) in &q {
		playground.checker_size_meters = cmd.meters;
		if let Some(mm) = ground_mat_q.iter().next() {
			if let Some(m) = materials.get_mut(&mm.0) {
				m.checker_size_m = cmd.meters;
			}
		}
		log::info!("checker size set to {} m", cmd.meters);
		console.0.clear();
		commands.entity(entity).despawn();
	}
}
