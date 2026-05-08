//! Systems that react to entities spawned by [`super::PlaygroundCommand::react`],
//! [`super::Render::react`], and [`super::Settings::react`].
//!
//! Add more `react_*` systems here to compose behavior without coupling parsers to side effects.

use bevy::prelude::*;

use crate::checkerboard_material::CheckerboardMaterial;
use crate::commands::{PlaygroundCommand, Render, Settings};
use crate::ground::{GroundPlane, PlaygroundSettings};
use crate::input::CommandConsoleOutput;
use crate::preview::PreviewConfig;

/// Root announcement entities: `help` fills the HUD; all variants are despawned after this frame’s reactions.
pub fn react_playground_command_root(
	mut commands: Commands,
	q: Query<(Entity, &PlaygroundCommand), Added<PlaygroundCommand>>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for (entity, cmd) in &q {
		if matches!(cmd, PlaygroundCommand::Help) {
			console.0 = PlaygroundCommand::long_help_string();
		}
		commands.entity(entity).despawn();
	}
}

pub fn react_render_to_preview(
	mut commands: Commands,
	q: Query<(Entity, &Render), Added<Render>>,
	mut preview: ResMut<PreviewConfig>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for (entity, render) in &q {
		*preview = render.into_preview_config();
		console.0.clear();
		commands.entity(entity).despawn();
	}
}

pub fn react_settings_to_playground(
	mut commands: Commands,
	q: Query<(Entity, &Settings), Added<Settings>>,
	mut playground: ResMut<PlaygroundSettings>,
	mut materials: ResMut<Assets<CheckerboardMaterial>>,
	ground_mat_q: Query<&MeshMaterial3d<CheckerboardMaterial>, With<GroundPlane>>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for (entity, settings) in &q {
		match *settings {
			Settings::CheckerSize { meters } => {
				playground.checker_size_meters = meters;
				if let Some(mm) = ground_mat_q.iter().next() {
					if let Some(m) = materials.get_mut(&mm.0) {
						m.checker_size_m = meters;
					}
				}
				log::info!("checker size set to {meters} m");
			}
			Settings::Seed { value } => {
				playground.seed = value;
				log::info!("playground seed set to {value}");
			}
		}
		console.0.clear();
		commands.entity(entity).despawn();
	}
}
