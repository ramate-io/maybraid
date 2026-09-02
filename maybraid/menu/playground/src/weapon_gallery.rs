//! Playground-only gallery of generated firearm kits.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use camera_controls::look::CameraLookEnabled;
use crozon_character_items::{random_gallery_firearms, ItemRng};
use crozon_character_playground::CameraController;
use maybraid_menu_controller::MenuController;
use menu_components::{screen_back_scene, BrandModeLine};
use menu_screens::{add_menu_input, take_menu_show_request, MenuScreen};

use crate::preview::spawn_firearm;

const GALLERY_COUNT: usize = 20;
const GALLERY_COLS: usize = 5;
const SPACING_X: f32 = 1.2;
const SPACING_Z: f32 = 0.85;
const GUN_Y: f32 = 0.45;

/// Queue a generated-weapon gallery spawn.
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestShowWeapons;

/// Marker on the spawned weapon-gallery HUD root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct WeaponGalleryScreen;

/// Tag on spawned kit hosts so preview sync does not treat them as the
/// character preview (and so they can be cleaned up with the screen).
#[derive(Component, Debug, Clone, Copy)]
struct WeaponGalleryItem;

pub fn request_show_weapons(commands: &mut Commands) {
	commands.spawn(RequestShowWeapons);
}

pub struct WeaponGalleryPlugin;

impl Plugin for WeaponGalleryPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		app.add_systems(
			Update,
			(apply_show_weapons, despawn_weapon_gallery_items, sync_weapon_gallery_look),
		);
	}
}

fn apply_show_weapons(
	mut commands: Commands,
	requests: Query<Entity, With<RequestShowWeapons>>,
	existing: Query<Entity, With<MenuScreen>>,
	items: Query<Entity, With<WeaponGalleryItem>>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	if !take_menu_show_request(&mut commands, &requests, &existing) {
		return;
	}
	for entity in &items {
		commands.entity(entity).despawn();
	}
	commands.spawn_scene(weapon_gallery_scene());
	spawn_gallery_guns(&mut commands);
	if let Ok((mut transform, mut controller)) = cameras.single_mut() {
		frame_weapon_gallery(&mut transform, &mut controller);
	}
}

fn spawn_gallery_guns(commands: &mut Commands) {
	let items = random_gallery_firearms(&mut ItemRng::from_entropy(), GALLERY_COUNT);
	let rows = GALLERY_COUNT.div_ceil(GALLERY_COLS);
	for (index, item) in items.into_iter().enumerate() {
		let Some(spec) = item.firearm_spec() else {
			continue;
		};
		let transform = Transform::from_translation(cell_translation(index, rows))
			.with_rotation(Quat::from_rotation_y(FRAC_PI_2));
		for entity in spawn_firearm(commands, spec, transform) {
			commands.entity(entity).insert(WeaponGalleryItem);
		}
	}
}

fn cell_translation(index: usize, rows: usize) -> Vec3 {
	let col = (index % GALLERY_COLS) as f32;
	let row = (index / GALLERY_COLS) as f32;
	let cols = GALLERY_COLS as f32;
	let rows = rows.max(1) as f32;
	Vec3::new((col - (cols - 1.0) * 0.5) * SPACING_X, GUN_Y, (row - (rows - 1.0) * 0.5) * SPACING_Z)
}

fn frame_weapon_gallery(transform: &mut Transform, controller: &mut CameraController) {
	let look_at = Vec3::new(0.0, GUN_Y, 0.0);
	*transform = Transform::from_xyz(0.0, 1.5, 5.2).looking_at(look_at, Vec3::Y);
	let rotation = transform.rotation;
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
	let sin_yaw = 2.0 * (w * y + x * z);
	let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
	controller.yaw = sin_yaw.atan2(cos_yaw);
	let sin_pitch = 2.0 * (w * x - y * z);
	controller.pitch = sin_pitch.clamp(-1.0, 1.0).asin();
}

fn despawn_weapon_gallery_items(
	mut commands: Commands,
	screens: Query<(), With<WeaponGalleryScreen>>,
	pending: Query<(), With<RequestShowWeapons>>,
	items: Query<Entity, With<WeaponGalleryItem>>,
) {
	if !screens.is_empty() || !pending.is_empty() {
		return;
	}
	for entity in &items {
		commands.entity(entity).despawn();
	}
}

fn sync_weapon_gallery_look(
	screens: Query<(), With<WeaponGalleryScreen>>,
	mut look: ResMut<CameraLookEnabled>,
) {
	look.0 = !screens.is_empty();
}

fn weapon_gallery_scene() -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> =
		vec![Box::new(BrandModeLine::new("Weapons").scene()), Box::new(screen_back_scene())];
	bsn! {
		WeaponGalleryScreen
		MenuScreen
		MenuController
		BackgroundColor(Color::NONE)
		Node {
			width: percent(100),
			height: percent(100),
		}
		Pickable::IGNORE
		Children [ {children} ]
	}
}

#[cfg(test)]
mod tests {
	use super::{cell_translation, GALLERY_COLS, GALLERY_COUNT};

	#[test]
	fn grid_is_centered_on_the_origin() {
		let rows = GALLERY_COUNT.div_ceil(GALLERY_COLS);
		let first = cell_translation(0, rows);
		let last_col = cell_translation(GALLERY_COLS - 1, rows);
		assert!((first.x + last_col.x).abs() < 1e-5);
		assert!(first.y > 0.0);
	}
}
