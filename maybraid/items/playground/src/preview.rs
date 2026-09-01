//! Spawn the selected [`FirearmConcept`] via LodScene host fulfill.

use bevy::prelude::*;
use firearms::{firearm_bounds, spawn_firearm_components, FirearmConcept, FirearmRoot};

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub struct PreviewConfig {
	pub concept: FirearmConcept,
}

impl Default for PreviewConfig {
	fn default() -> Self {
		Self { concept: FirearmConcept::Bullpup }
	}
}

#[derive(Resource, Default)]
pub(crate) struct PreviewSyncState {
	spawned: Option<FirearmConcept>,
}

pub(crate) fn sync_preview(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	mut sync: Local<PreviewSyncState>,
	roots: Query<Entity, With<FirearmRoot>>,
) {
	if sync.spawned == Some(config.concept) {
		return;
	}

	for entity in &roots {
		commands.entity(entity).try_despawn();
	}

	let concept = config.concept;
	spawn_firearm_components(
		&mut commands,
		&concept,
		Transform::from_xyz(0.0, 1.0, 0.0),
		firearm_bounds(&concept),
	);
	sync.spawned = Some(concept);
}
