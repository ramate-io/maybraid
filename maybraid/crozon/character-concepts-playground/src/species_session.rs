//! Per-species build memory and runtime reset when switching species.

use bevy::prelude::*;
use crozon_characters::species::{braidman::BraidmanConfig, brodler::BrodlerConfig, mygr::MygrConfig};

use crate::{
	camera_focus::{queue_species_default_camera_focus, PendingCameraFocus},
	focus_reference::FocusReferenceSyncState,
	preview::{
		ConceptPreviewConfig, ConceptPreviewSyncState, ConceptSpecies, PreviewRespawnCooldown,
	},
	ui::CreatorUiState,
};

/// Remembers each species' last config so switching tabs restores prior builds.
#[derive(Resource, Debug, Clone)]
pub struct SpeciesSessionState {
	pub braidman: BraidmanConfig,
	pub brodler: BrodlerConfig,
	pub mygr: MygrConfig,
	pub braidman_animation: crate::animation::ConceptAnimation,
	pub brodler_animation: crate::animation::ConceptAnimation,
	pub mygr_animation: crate::animation::ConceptAnimation,
}

impl Default for SpeciesSessionState {
	fn default() -> Self {
		Self {
			braidman: BraidmanConfig::default_preview(),
			brodler: BrodlerConfig::default_preview(),
			mygr: MygrConfig::default_preview(),
			braidman_animation: crate::animation::ConceptAnimation::default(),
			brodler_animation: crate::animation::ConceptAnimation::default(),
			mygr_animation: crate::animation::ConceptAnimation::default(),
		}
	}
}

impl SpeciesSessionState {
	pub fn persist(&mut self, config: &ConceptPreviewConfig) {
		match config {
			ConceptPreviewConfig::Braidman { config, animation } => {
				self.braidman.clone_from(config);
				self.braidman_animation = *animation;
			}
			ConceptPreviewConfig::Brodler { config, animation } => {
				self.brodler.clone_from(config);
				self.brodler_animation = *animation;
			}
			ConceptPreviewConfig::Mygr { config, animation } => {
				self.mygr.clone_from(config);
				self.mygr_animation = *animation;
			}
		}
	}

	pub fn load(&self, species: ConceptSpecies) -> ConceptPreviewConfig {
		match species {
			ConceptSpecies::Braidman => ConceptPreviewConfig::braidman_with_animation(
				self.braidman.clone(),
				self.braidman_animation,
			),
			ConceptSpecies::Brodler => ConceptPreviewConfig::brodler_with_animation(
				self.brodler.clone(),
				self.brodler_animation,
			),
			ConceptSpecies::Mygr => ConceptPreviewConfig::mygr_with_animation(
				self.mygr.clone(),
				self.mygr_animation,
			),
		}
	}
}

/// Tracks which species default camera framing was last applied for.
#[derive(Resource, Default)]
pub struct CameraFocusBootState {
	pub applied_for: Option<ConceptSpecies>,
}

pub fn persist_species_session(
	config: Res<ConceptPreviewConfig>,
	mut session: ResMut<SpeciesSessionState>,
) {
	if config.is_changed() {
		session.persist(&config);
	}
}

pub fn invalidate_species_runtime(
	preview_sync: &mut ConceptPreviewSyncState,
	focus_sync: &mut FocusReferenceSyncState,
	cooldown: &mut PreviewRespawnCooldown,
) {
	preview_sync.invalidate();
	focus_sync.invalidate();
	cooldown.frames_remaining = 0;
}

pub fn reset_for_species_switch(
	species: ConceptSpecies,
	session: &mut SpeciesSessionState,
	config: &mut ConceptPreviewConfig,
	ui_state: &mut CreatorUiState,
	preview_sync: &mut ConceptPreviewSyncState,
	focus_sync: &mut FocusReferenceSyncState,
	cooldown: &mut PreviewRespawnCooldown,
	pending_camera: &mut PendingCameraFocus,
	camera_boot: &mut CameraFocusBootState,
) {
	session.persist(config);
	*config = session.load(species);
	invalidate_species_runtime(preview_sync, focus_sync, cooldown);
	ui_state.hovered = None;
	ui_state.last_selected = None;
	ui_state.bump_layout_revision();
	pending_camera.focus = None;
	pending_camera.resolved_target = None;
	pending_camera.focus_trigger = None;
	camera_boot.applied_for = None;
	queue_species_default_camera_focus(pending_camera, ui_state, config, "species-switch");
	camera_boot.applied_for = Some(species);
}

pub fn ensure_species_camera_focus(
	config: Res<ConceptPreviewConfig>,
	mut ui_state: ResMut<CreatorUiState>,
	mut pending: ResMut<PendingCameraFocus>,
	mut boot: ResMut<CameraFocusBootState>,
) {
	let species = config.species();
	if boot.applied_for == Some(species) {
		return;
	}
	boot.applied_for = Some(species);
	queue_species_default_camera_focus(&mut pending, &mut ui_state, &config, "startup-default");
}
