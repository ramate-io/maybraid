//! Registers [`StandardMaterial`] for frond [`RenderItem`] types (merged [`Mesh3d`] children).

use bevy::prelude::*;

pub struct FrondRenderItemPlugin;

impl Default for FrondRenderItemPlugin {
	fn default() -> Self {
		Self
	}
}

impl Plugin for FrondRenderItemPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MaterialPlugin<StandardMaterial>>() {
			app.add_plugins(MaterialPlugin::<StandardMaterial>::default());
		}
	}
}
