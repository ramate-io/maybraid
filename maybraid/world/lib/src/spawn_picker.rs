//! World adapter for the UI-owned spawn picker.
//!
//! Confirm and highlight live in [`maybraid_spawns_player`]. This module
//! mirrors the locked map sheet into the picker and re-exports the
//! controller types world already names.

use bevy::prelude::*;
use maybraid_controllers_core::{CaptureKind, InputRouting, InputSource};
use maybraid_spawns_player::WorldPlayerRespawnState;

use crate::map_view::WorldMapView;

pub use maybraid_spawns_player::{
	dispatch_spawn_picker, SpawnPickerCapture, SpawnPickerController, SpawnPickerSheet,
};

/// Copy the locked map sheet so the picker can run without depending on
/// `world/lib`.
pub fn sync_spawn_picker_sheet(map: Res<WorldMapView>, mut sheet: ResMut<SpawnPickerSheet>) {
	*sheet = SpawnPickerSheet::from_map(map.open, map.close_locked);
}

/// Sync the locked map, then claim or release picker MenuNav capture.
pub fn refresh_spawn_picker_focus(
	map: Res<WorldMapView>,
	mut sheet: ResMut<SpawnPickerSheet>,
	state: Res<WorldPlayerRespawnState>,
	mut controller: ResMut<SpawnPickerController>,
	mut capture: ResMut<SpawnPickerCapture>,
	mut router: InputRouting,
) {
	*sheet = SpawnPickerSheet::from_map(map.open, map.close_locked);
	let blocked = router.top_capture().is_some_and(|layer| {
		SpawnPickerController::higher_claim_blocks(layer)
			&& layer.owner() != SpawnPickerController::owner()
	});
	let owner = controller.apply_focus(*sheet, state.pending.is_some(), blocked);
	router.replace_owned_capture(
		&mut capture.token,
		InputSource::local_pad(),
		owner,
		CaptureKind::MenuNav,
	);
}
