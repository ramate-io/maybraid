//! Runtime PNG captures (optional; see `scripts/character-mesh-export-capture.sh`).

use std::path::Path;

use bevy::prelude::*;

pub fn setup_capture_world(world: &mut World, _root: Entity, _dir: std::path::PathBuf) {
	world.spawn((
		Camera3d::default(),
		Transform::from_xyz(0.0, 0.35, 3.2).looking_at(Vec3::new(0.0, 0.15, 0.0), Vec3::Y),
	));
	world.spawn((
		DirectionalLight { illuminance: 12_000.0, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.7, 0.4, 0.0)),
	));
}

pub fn request_runtime_shots(_world: &mut World, _dir: &Path) {
	// Screenshots are taken by the xvfb capture script after export.
}
