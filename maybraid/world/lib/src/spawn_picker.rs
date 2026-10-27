//! World adapter for the UI-owned spawn picker.
//!
//! Confirm and highlight live in [`maybraid_spawns_player`]. This module
//! mirrors the locked map sheet into the picker and re-exports the
//! controller types world already names.

use bevy::prelude::*;

use crate::map_view::WorldMapView;

pub use maybraid_spawns_player::{
	dispatch_spawn_picker, SpawnPickerCapture, SpawnPickerController, SpawnPickerSheet,
};

/// Copy the locked map sheet so the picker can run without depending on
/// `world/lib`.
pub fn sync_spawn_picker_sheet(
	map: Res<WorldMapView>,
	mut sheet: ResMut<SpawnPickerSheet>,
	roots: Query<Entity, With<crate::map_view::MapNameHud>>,
) {
	*sheet = SpawnPickerSheet::from_map(map.open, map.close_locked);
	sheet.map_capture_owner = roots.iter().next().map(maybraid_controllers_core::UiTarget::new);
}

pub use maybraid_spawns_player::refresh_spawn_picker_focus;
