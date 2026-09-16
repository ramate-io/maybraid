//! Push [`SkyClock`] onto lights, wash, field, disks, and distance fog.

use bevy::prelude::*;

use crate::celestial::{SkyFill, SkyMoon, SkySunDisk, CELESTIAL_DISTANCE_FACTOR};
use crate::clock::SkyClock;
use crate::dome::{DomeSettings, SkyWash};
use crate::field::{SkyField, SkyFieldMaterial};
use crate::SkySun;

pub(crate) fn apply_sky_mood(
	time: Res<Time>,
	mut clock: ResMut<SkyClock>,
	settings: Res<DomeSettings>,
	mut ambient: Option<ResMut<GlobalAmbientLight>>,
	mut sun: Query<(&mut Transform, &mut DirectionalLight), (With<SkySun>, Without<SkySunDisk>)>,
	mut fill: Query<&mut DirectionalLight, (With<SkyFill>, Without<SkySun>)>,
	mut disks: Query<(&mut Transform, &mut Visibility), (With<SkySunDisk>, Without<SkySun>)>,
	mut moons: Query<&mut Transform, (With<SkyMoon>, Without<SkySun>, Without<SkySunDisk>)>,
	field: Query<&MeshMaterial3d<SkyFieldMaterial>, With<SkyField>>,
	mut field_mats: ResMut<Assets<SkyFieldMaterial>>,
	wash: Query<&MeshMaterial3d<StandardMaterial>, With<SkyWash>>,
	mut wash_mats: ResMut<Assets<StandardMaterial>>,
	mut fog: Query<&mut DistanceFog>,
) {
	clock.advance(time.delta_secs());
	let mood = clock.sample();
	let pose = mood.sun_pose();
	let distance = settings.sphere_radius_m * CELESTIAL_DISTANCE_FACTOR;

	if let Some(ambient) = ambient.as_mut() {
		ambient.brightness = mood.ambient;
	}

	for (mut transform, mut light) in &mut sun {
		*transform = pose;
		light.color = mood.sun_color;
		light.illuminance = mood.sun_illuminance;
	}
	for mut light in &mut fill {
		light.color = mood.fill_color;
		light.illuminance = mood.fill_illuminance;
	}

	let disk_vis =
		if mood.sun_above_horizon() { Visibility::Inherited } else { Visibility::Hidden };
	for (mut transform, mut visibility) in &mut disks {
		*transform = Transform::from_translation(SkySun::disk_offset(&pose, distance));
		*visibility = disk_vis;
	}
	for mut transform in &mut moons {
		*transform = Transform::from_translation(mood.moon_dir() * distance);
	}

	for handle in &field {
		if let Some(mut material) = field_mats.get_mut(&handle.0) {
			material.apply_mood(mood);
		}
	}
	for handle in &wash {
		if let Some(mut material) = wash_mats.get_mut(&handle.0) {
			material.base_color = mood.horizon;
		}
	}
	for mut fog in &mut fog {
		fog.color = mood.fog;
		let mut sun_fog = mood.sun_color.to_linear();
		sun_fog.alpha = 0.4;
		fog.directional_light_color = Color::from(sun_fog);
	}
}
