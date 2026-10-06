//! World-to-screen projection for HUD pins that clamp to the viewport edge.

use bevy::prelude::*;

/// Projects world positions to screen space and clamps off-screen pins to the viewport edge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenPin;

impl ScreenPin {
	/// Clamps `point` to the axis-aligned rectangle `[min, max]` along the ray from `center`.
	pub fn clamp_to_rect(center: Vec2, point: Vec2, min: Vec2, max: Vec2) -> Vec2 {
		let dir = point - center;
		if dir.length_squared() < 1e-6 {
			return Vec2::new(center.x.clamp(min.x, max.x), center.y.clamp(min.y, max.y));
		}
		let mut t = f32::INFINITY;
		if dir.x.abs() > 1e-6 {
			let edge = if dir.x > 0.0 { max.x } else { min.x };
			t = t.min((edge - center.x) / dir.x);
		}
		if dir.y.abs() > 1e-6 {
			let edge = if dir.y > 0.0 { max.y } else { min.y };
			t = t.min((edge - center.y) / dir.y);
		}
		center + dir * t.clamp(0.0, 1.0)
	}

	/// Projects `world` through `camera`, clamping to the inset viewport when off-screen.
	///
	/// Returns screen position and whether the point is fully inside the viewport.
	pub fn project(
		camera: &Camera,
		camera_transform: &GlobalTransform,
		world: Vec3,
		margin_px: f32,
	) -> Option<(Vec2, bool)> {
		let rect = camera.logical_viewport_rect()?;
		let mut ndc = camera.world_to_ndc(camera_transform, world)?;
		let in_frustum = ndc.z > 0.0 && ndc.z < 1.0;
		if !in_frustum {
			ndc.x = -ndc.x;
			ndc.y = -ndc.y;
		}
		ndc.y = -ndc.y;
		let mut screen = (ndc.truncate() + Vec2::ONE) / 2.0 * rect.size() + rect.min;
		let on_screen = in_frustum
			&& screen.x >= rect.min.x
			&& screen.x <= rect.max.x
			&& screen.y >= rect.min.y
			&& screen.y <= rect.max.y;
		if !on_screen {
			screen = Self::clamp_to_rect(
				rect.center(),
				screen,
				rect.min + Vec2::splat(margin_px),
				rect.max - Vec2::splat(margin_px),
			);
		}
		Some((screen, on_screen))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn clamp_hits_the_near_edge() {
		let clamped = ScreenPin::clamp_to_rect(
			Vec2::new(100.0, 100.0),
			Vec2::new(400.0, 100.0),
			Vec2::splat(20.0),
			Vec2::splat(180.0),
		);
		assert!((clamped.x - 180.0).abs() < 1e-3);
		assert!((clamped.y - 100.0).abs() < 1e-3);
	}

	#[test]
	fn clamp_keeps_an_interior_point() {
		let clamped = ScreenPin::clamp_to_rect(
			Vec2::new(100.0, 100.0),
			Vec2::new(120.0, 110.0),
			Vec2::splat(20.0),
			Vec2::splat(180.0),
		);
		assert!((clamped.x - 120.0).abs() < 1e-3);
		assert!((clamped.y - 110.0).abs() < 1e-3);
	}
}
