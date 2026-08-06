//! Sope's Banyan no longer registers stick/ball RenderItem plugins.
//!
//! Presentation goes through [`chico_vegetation_components::VegetationProceduralPlugin`]
//! and [`VegetationComponents`](chico_vegetation_components::VegetationComponents).

use bevy::prelude::*;

pub struct SopesBanyanRenderItemPlugin;

impl Default for SopesBanyanRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

pub fn ensure_registered(app: &mut App) {
	if app.is_plugin_added::<SopesBanyanRenderItemPlugin>() {
		return;
	}
	app.add_plugins(SopesBanyanRenderItemPlugin);
}

impl Plugin for SopesBanyanRenderItemPlugin {
	fn build(&self, _app: &mut App) {}
}
