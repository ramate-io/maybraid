//! Curved frond spine sampling.

use bevy::prelude::*;

use super::config::FrondConfig;

const TANGENT_EPS: f32 = 1e-3;

/// Spine position in frond-local space (+X along length, −Y droop).
pub fn spine_at(config: &FrondConfig, t: f32) -> Vec3 {
	let t = t.clamp(0.0, 1.0);
	let x = t * config.length;
	let y = -config.droop * t * t;
	Vec3::new(x, y, 0.0)
}

/// Unit tangent along the spine at normalized height `t`.
pub fn tangent_at(config: &FrondConfig, t: f32) -> Vec3 {
	let t_lo = (t - TANGENT_EPS).clamp(0.0, 1.0);
	let t_hi = (t + TANGENT_EPS).clamp(0.0, 1.0);
	let delta = spine_at(config, t_hi) - spine_at(config, t_lo);
	delta.normalize_or_zero()
}

/// Orthonormal frame at `t`: tangent, lateral (leaflet width axis), binormal.
pub fn frame_at(config: &FrondConfig, t: f32, twist: f32) -> (Vec3, Vec3, Vec3) {
	let tangent = tangent_at(config, t);
	let mut lateral = Vec3::Z.cross(tangent);
	if lateral.length_squared() < 1e-10 {
		lateral = Vec3::Y.cross(tangent);
	}
	lateral = lateral.normalize_or_zero();
	let binormal = tangent.cross(lateral).normalize_or_zero();
	let roll = Quat::from_axis_angle(tangent, twist);
	(roll * tangent, roll * lateral, roll * binormal)
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn spine_starts_at_origin_and_drops() -> Result<()> {
		let config = FrondConfig::default();
		assert!(spine_at(&config, 0.0).length() < 1e-5);
		assert!(spine_at(&config, 1.0).y < spine_at(&config, 0.5).y);
		Ok(())
	}
}
