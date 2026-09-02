//! Spawn the selected [`FirearmKit`] via LodScene host fulfill; pose live-updates.

use bevy::prelude::*;
use firearms::{
	firearm_bounds, spawn_firearm_components, ActiveRigPose, FirearmConcept, FirearmKit,
	FirearmMembers, FirearmPose, FirearmRoot, RigRoot,
};

#[derive(Resource, Clone, Copy, PartialEq)]
pub struct PreviewConfig {
	pub kit: FirearmKit,
	pub pose: FirearmPose,
}

impl Default for PreviewConfig {
	fn default() -> Self {
		Self { kit: FirearmConcept::Bullpup.kit(), pose: FirearmPose::default() }
	}
}

#[derive(Default)]
pub(crate) struct PreviewSyncState {
	spawned: Option<FirearmKit>,
	pose: Option<FirearmPose>,
}

pub(crate) fn sync_preview(
	mut commands: Commands,
	config: Res<PreviewConfig>,
	mut sync: Local<PreviewSyncState>,
	roots: Query<(Entity, Option<&FirearmMembers>), With<FirearmRoot>>,
	mut poses: Query<&mut ActiveRigPose, With<RigRoot>>,
) {
	if sync.spawned != Some(config.kit) {
		for (entity, _) in &roots {
			commands.entity(entity).try_despawn();
		}
		spawn_firearm_components(
			&mut commands,
			&config.kit,
			Transform::from_xyz(0.0, 1.0, 0.0),
			firearm_bounds(&config.kit),
		);
		sync.spawned = Some(config.kit);
		sync.pose = None;
		return;
	}

	if sync.pose != Some(config.pose) && write_pose(config.pose, &roots, &mut poses) {
		sync.pose = Some(config.pose);
	}
}

fn write_pose(
	pose: FirearmPose,
	roots: &Query<(Entity, Option<&FirearmMembers>), With<FirearmRoot>>,
	poses: &mut Query<&mut ActiveRigPose, With<RigRoot>>,
) -> bool {
	let resolved = pose.to_resolved();
	let mut wrote = false;
	for (_, members) in roots.iter() {
		let Some(members) = members else {
			return false;
		};
		for member in members.iter() {
			if let Ok(mut active) = poses.get_mut(member) {
				active.pose = resolved.clone();
				wrote = true;
			}
		}
	}
	wrote
}
