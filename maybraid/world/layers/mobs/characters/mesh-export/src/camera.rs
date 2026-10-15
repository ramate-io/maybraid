//! Shared framing for runtime GPU captures and OBJ reprojection.

use bevy::math::{Mat4, Quat, Vec3};
use serde::Serialize;

pub const CAPTURE_WIDTH: u32 = 640;
pub const CAPTURE_HEIGHT: u32 = 640;

/// Vertical field of view (radians), matching Bevy `Camera3d` default perspective.
pub const CAPTURE_YFOV_RAD: f32 = std::f32::consts::FRAC_PI_4;

#[derive(Clone, Debug, Serialize)]
pub struct CaptureViewDef {
	pub name: &'static str,
	pub eye: [f32; 3],
	pub look_at: [f32; 3],
	pub up: [f32; 3],
}

#[derive(Serialize)]
pub struct CaptureCameraManifest {
	pub width: u32,
	pub height: u32,
	pub yfov_rad: f32,
	pub center: [f32; 3],
	pub extent_m: f32,
	pub views: Vec<CaptureViewDef>,
}

pub fn views_for_bounds(min: Vec3, max: Vec3) -> Vec<CaptureViewDef> {
	let center = (min + max) * 0.5;
	let extent = (max - min).max_element().max(0.5);
	let dist = extent * 2.35;
	let c = center.to_array();
	let front_eye = center + Vec3::new(0.0, extent * 0.08, dist);
	let side_eye = center + Vec3::new(dist, extent * 0.08, 0.0);
	let tq_eye = center + Vec3::new(dist * 0.72, extent * 0.12, dist * 0.72);
	vec![
		CaptureViewDef {
			name: "front",
			eye: front_eye.to_array(),
			look_at: c,
			up: [0.0, 1.0, 0.0],
		},
		CaptureViewDef {
			name: "side",
			eye: side_eye.to_array(),
			look_at: c,
			up: [0.0, 1.0, 0.0],
		},
		CaptureViewDef {
			name: "three_quarter",
			eye: tq_eye.to_array(),
			look_at: c,
			up: [0.0, 1.0, 0.0],
		},
	]
}

pub fn manifest(min: Vec3, max: Vec3) -> CaptureCameraManifest {
	let center = (min + max) * 0.5;
	let extent = (max - min).max_element().max(0.5);
	CaptureCameraManifest {
		width: CAPTURE_WIDTH,
		height: CAPTURE_HEIGHT,
		yfov_rad: CAPTURE_YFOV_RAD,
		center: center.to_array(),
		extent_m: extent,
		views: views_for_bounds(min, max),
	}
}

pub fn view_transform(def: &CaptureViewDef) -> bevy::prelude::Transform {
	let eye = Vec3::from_array(def.eye);
	let look_at = Vec3::from_array(def.look_at);
	let up = Vec3::from_array(def.up);
	bevy::prelude::Transform::from_translation(eye).looking_at(look_at, up)
}

/// OpenGL-style camera matrix for pyrender (camera looks down -Z).
pub fn view_matrix(def: &CaptureViewDef) -> Mat4 {
	let eye = Vec3::from_array(def.eye);
	let look_at = Vec3::from_array(def.look_at);
	let up = Vec3::from_array(def.up);
	Mat4::look_to_rh(eye, (look_at - eye).normalize(), up)
}
