use std::f32::consts::PI;

use bevy::prelude::*;

use crate::preview::LightingMode;

#[derive(Component)]
pub struct PlaygroundSun;

#[derive(Component)]
pub struct PlaygroundFill;

pub fn setup_lighting(mut commands: Commands) {
	commands.insert_resource(GlobalAmbientLight {
		brightness: 220.0,
		color: Color::srgb(0.72, 0.78, 0.92),
		..default()
	});
	commands.spawn((
		PlaygroundSun,
		DirectionalLight {
			illuminance: 9800.0,
			color: Color::srgb(1.0, 0.92, 0.82),
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0)),
	));
	commands.spawn((
		PlaygroundFill,
		DirectionalLight {
			illuminance: 2400.0,
			color: Color::srgb(0.55, 0.68, 1.0),
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 5.0, -PI / 4.0, 0.0)),
	));
}

pub fn sync_lighting(
	mode: Res<LightingMode>,
	mut ambient: ResMut<GlobalAmbientLight>,
	mut sun: Query<&mut DirectionalLight, (With<PlaygroundSun>, Without<PlaygroundFill>)>,
	mut fill: Query<&mut DirectionalLight, (With<PlaygroundFill>, Without<PlaygroundSun>)>,
) {
	if !mode.is_changed() && !mode.is_added() {
		return;
	}
	match *mode {
		LightingMode::Day => {
			ambient.brightness = 220.0;
			ambient.color = Color::srgb(0.72, 0.78, 0.92);
			if let Ok(mut light) = sun.single_mut() {
				light.illuminance = 9800.0;
				light.color = Color::srgb(1.0, 0.92, 0.82);
			}
			if let Ok(mut light) = fill.single_mut() {
				light.illuminance = 2400.0;
				light.color = Color::srgb(0.55, 0.68, 1.0);
			}
		}
		LightingMode::Night => {
			ambient.brightness = 28.0;
			ambient.color = Color::srgb(0.22, 0.28, 0.48);
			if let Ok(mut light) = sun.single_mut() {
				light.illuminance = 700.0;
				light.color = Color::srgb(0.62, 0.72, 1.0);
			}
			if let Ok(mut light) = fill.single_mut() {
				light.illuminance = 180.0;
				light.color = Color::srgb(0.28, 0.36, 0.62);
			}
		}
	}
}
