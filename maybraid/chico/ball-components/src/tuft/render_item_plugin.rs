//! Registers [`StandardMaterial`] for tuft [`RenderItem`] types (merged [`Mesh3d`] children).

use bevy::prelude::*;

pub struct TuftRenderItemPlugin;

impl Default for TuftRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

impl Plugin for TuftRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MaterialPlugin<StandardMaterial>>() {
			app.add_plugins(MaterialPlugin::<StandardMaterial>::default());
		}
	}
}

/// Alias kept for tree assembly plugins that register succulent tufts today.
pub type SucculentTuftRenderItemPlugin = TuftRenderItemPlugin;
pub type ChicoTuftRenderItemPlugin = TuftRenderItemPlugin;
