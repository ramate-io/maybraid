//! Persistent preview sphere. `/show` restamps [`MaterialRefRoot`].

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use firearms::laser_hex_ref;
use material_ref::{MaterialRef, MaterialRefRoot};

use crate::commands::preview_label;

#[derive(Component)]
pub struct PreviewSphere;

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct PreviewConfig {
	pub material: MaterialRef,
}

impl Default for PreviewConfig {
	fn default() -> Self {
		Self { material: laser_hex_ref() }
	}
}

impl PreviewConfig {
	pub fn status_label(&self) -> String {
		format!("preview: {}", preview_label(&self.material))
	}
}

pub fn setup_preview(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	config: Res<PreviewConfig>,
) {
	commands.spawn((
		PreviewSphere,
		Name::new("preview-sphere"),
		Mesh3d(meshes.add(Sphere::new(0.55))),
		MaterialRefRoot(config.material.clone()),
		Transform::from_xyz(0.0, 1.0, 0.0),
		NotShadowCaster,
	));
}

pub fn sync_preview(
	config: Res<PreviewConfig>,
	mut roots: Query<&mut MaterialRefRoot, With<PreviewSphere>>,
) {
	if !config.is_changed() {
		return;
	}
	for mut root in &mut roots {
		if root.0 != config.material {
			root.0 = config.material.clone();
		}
	}
}
