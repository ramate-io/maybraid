//! Spawn the selected [`FirearmKit`] via LodScene host fulfill.

use bevy::prelude::*;
use firearms::{firearm_bounds, spawn_firearm_components, FirearmConcept, FirearmKit, FirearmRoot};

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub struct PreviewConfig {
	pub kit: FirearmKit,
}

impl Default for PreviewConfig {
	fn default() -> Self {
		Self { kit: FirearmConcept::Bullpup.kit() }
	}
}

#[derive(Resource, Default)]
pub(crate) struct PreviewSyncState {
	spawned: Option<FirearmKit>,
}

pub(crate) fn sync_preview(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	mut sync: Local<PreviewSyncState>,
	roots: Query<Entity, With<FirearmRoot>>,
) {
	if sync.spawned == Some(config.kit) {
		return;
	}

	for entity in &roots {
		commands.entity(entity).try_despawn();
	}

	let kit = config.kit;
	spawn_firearm_components(
		&mut commands,
		&kit,
		Transform::from_xyz(0.0, 1.0, 0.0),
		firearm_bounds(&kit),
	);
	sync.spawned = Some(kit);
}
