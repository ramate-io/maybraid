//! Mesh moon. The sun is the Cosimo glint on the field shader.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;
use std::f32::consts::PI;

use crate::dome::DomeSettings;
use crate::SkySun;

/// Just inside the dome so the moon does not clip the wash shell.
pub const CELESTIAL_DISTANCE_FACTOR: f32 = 0.82;
pub const MOON_RADIUS_M: f32 = 22.0;
pub const MOON_YAW_OFFSET: f32 = 120.0 * PI / 180.0;
pub const MOON_LIFT: f32 = 0.38;
pub const MOON_COLOR: Color = Color::hsla(215.0, 0.12, 0.78, 1.0);

/// Cooler, smaller companion opposite the key.
#[derive(Component, Debug, Clone, Copy)]
pub struct SkyMoon;

impl SkyMoon {
	/// Offset ~120° from the sun and lifted so it sits higher in the dome.
	pub fn direction_from_sun(sun_disk: Vec3) -> Vec3 {
		let yawed = Quat::from_axis_angle(Vec3::Y, MOON_YAW_OFFSET) * sun_disk;
		(yawed + Vec3::Y * MOON_LIFT).normalize_or_zero()
	}
}

/// Cooler fill opposite the key. Does not cast shadows.
#[derive(Component, Debug, Clone, Copy)]
pub struct SkyFill;

pub(crate) fn spawn_sky_celestial(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	settings: Res<DomeSettings>,
	sun: Query<&Transform, With<SkySun>>,
	dome: Query<Entity, With<crate::SkyDome>>,
) {
	let Ok(sun) = sun.single() else {
		return;
	};
	let Ok(parent) = dome.single() else {
		return;
	};
	let distance = settings.sphere_radius_m * CELESTIAL_DISTANCE_FACTOR;
	let sun_dir = SkySun::disk_direction(sun);
	let moon_dir = SkyMoon::direction_from_sun(sun_dir);

	commands.spawn((
		Name::new("sky-moon"),
		SkyMoon,
		Mesh3d(meshes.add(Sphere::new(MOON_RADIUS_M))),
		MeshMaterial3d(materials.add(unlit_blend(MOON_COLOR, 0.55))),
		Transform::from_translation(moon_dir * distance),
		Visibility::Inherited,
		NotShadowCaster,
		ChildOf(parent),
	));
}

fn unlit_blend(color: Color, alpha: f32) -> StandardMaterial {
	let mut linear = color.to_linear();
	linear.alpha = alpha;
	StandardMaterial {
		base_color: Color::from(linear),
		unlit: true,
		alpha_mode: AlphaMode::Blend,
		cull_mode: None,
		fog_enabled: false,
		..default()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sun_disk_sits_opposite_the_shine_ray() {
		let sun = SkySun::pose();
		let dir = SkySun::disk_direction(&sun);
		assert!(dir.y > 0.0, "disk should sit in the sky");
		assert!(SkySun::shine_direction(&sun).dot(dir) < -0.99);
	}

	#[test]
	fn moon_is_offset_and_higher() {
		let sun = SkySun::disk_direction(&SkySun::pose());
		let moon = SkyMoon::direction_from_sun(sun);
		assert!(moon.y > sun.y);
		let sun_xz = Vec3::new(sun.x, 0.0, sun.z).normalize_or_zero();
		let moon_xz = Vec3::new(moon.x, 0.0, moon.z).normalize_or_zero();
		assert!(sun_xz.dot(moon_xz) < 0.0, "moon should sit away from the sun on XZ");
	}
}
