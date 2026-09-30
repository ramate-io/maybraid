//! [`VegetationGenerationPlugin`]: forest, grove, and bump-out selection.

use bevy::app::{App, Plugin};
use bevy::prelude::*;

use crate::config::VegetationLayerConfig;
use crate::stream::{configure_stream_systems, register_bump_out_generate, register_forest_generate};

/// Systems that arm forest and bump-out keep regions.
///
/// Playground `apply_commands` orders `.before(VegetationGenerationSystems)`,
/// which is today's `.after(apply_commands)` on the streams.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VegetationGenerationSystems;

/// Forest / grove / bump-out selection. No grow, no hosts, no terrain.
///
/// Budgets are the three fields of [`VegetationLayerConfig`], inserted here
/// and not again by the assembler.
pub struct VegetationGenerationPlugin {
	pub config: VegetationLayerConfig,
}

impl VegetationGenerationPlugin {
	pub fn new(config: VegetationLayerConfig) -> Self {
		Self { config }
	}
}

impl Default for VegetationGenerationPlugin {
	fn default() -> Self {
		Self::new(VegetationLayerConfig::default())
	}
}

impl Plugin for VegetationGenerationPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(self.config.clone());
		register_forest_generate(app, self.config.forest_budget);
		register_bump_out_generate(
			app,
			self.config.bump_out_budget,
			self.config.medium_bump_out_budget,
		);
		configure_stream_systems(app);
	}
}
