//! Moon direction and fill light. Sun and moon disks are paper rings on the shaders.

use bevy::prelude::*;
use std::f32::consts::PI;

/// Kept for callers that place extras just inside the wash shell.
pub const CELESTIAL_DISTANCE_FACTOR: f32 = 0.82;
pub const MOON_RADIUS_M: f32 = 22.0;
pub const MOON_YAW_OFFSET: f32 = 120.0 * PI / 180.0;
pub const MOON_LIFT: f32 = 0.38;
pub const MOON_COLOR: Color = Color::hsla(215.0, 0.12, 0.78, 1.0);

/// Cooler, smaller companion opposite the key.
#[derive(Component, Debug, Clone, Copy)]
pub struct SkyMoon;

impl SkyMoon {
	/// Offset ~120° from the sun. Yaw around Y is a no-op at zenith, so noon
	/// uses X instead — otherwise the moon sits in the sun and reads as a pupil.
	pub fn direction_from_sun(sun_disk: Vec3) -> Vec3 {
		let sun = sun_disk.normalize_or_zero();
		let pivot = if sun.cross(Vec3::Y).length_squared() < 0.04 { Vec3::X } else { Vec3::Y };
		let yawed = Quat::from_axis_angle(pivot, MOON_YAW_OFFSET) * sun;
		(yawed + Vec3::Y * MOON_LIFT).normalize_or_zero()
	}
}

/// Cooler fill opposite the key. Does not cast shadows.
#[derive(Component, Debug, Clone, Copy)]
pub struct SkyFill;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::SkySun;

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

	#[test]
	fn moon_leaves_the_zenith_sun() {
		let moon = SkyMoon::direction_from_sun(Vec3::Y);
		assert!(moon.dot(Vec3::Y) < 0.85, "moon={moon}");
	}
}
