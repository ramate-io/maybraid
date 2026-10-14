//! Third-person look-stick zoom lock. A flick parks the camera 10 m or 20 m
//! from the player in the throw direction; the follow pose eases back when
//! cancelled.

use bevy::prelude::*;
use maybraid_input::{FlickLane, PadButton, VirtualPad};
use player::PlayerCameraPose;

use crate::look::{CameraController, CameraLookSuppressed, CameraPov};
use crate::FollowCamera;

/// First lock, and the step-in distance after an opposite flick from 20 m.
pub const ZOOM_NEAR: f32 = 10.0;
/// Second flick in the same camera-relative direction.
pub const ZOOM_FAR: f32 = 20.0;
/// Exponential blend toward / away from the lock (same curve as ADS focus).
pub const ZOOM_BLEND_SPEED: f32 = 6.0;
/// Same / opposite stick cone (~60°).
const SAME_DIR_DOT: f32 = 0.5;

/// Committed zoom lock: world XZ offset plus the stick that produced it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomLock {
	pub world_xz: Vec2,
	pub stick: Vec2,
	pub distance: f32,
}

/// Runtime zoom state on the follow camera.
#[derive(Component, Clone, Debug, Default)]
pub struct ThirdPersonZoom {
	pub lock: Option<ZoomLock>,
	last: Option<ZoomLock>,
	pub blend: f32,
	lane: FlickLane,
}

impl ThirdPersonZoom {
	pub fn is_active(&self) -> bool {
		self.lock.is_some() || self.blend > 1e-3
	}

	pub fn release(&mut self) {
		self.lock = None;
		self.lane.reset();
	}

	pub fn reset_lane(&mut self) {
		self.lane.reset();
	}

	pub fn apply(&mut self, next: Option<ZoomLock>) {
		self.lock = next;
		if let Some(lock) = next {
			self.last = Some(lock);
		}
	}

	pub fn sample_look(&mut self, t: f32, look_analog: Vec2) -> Option<Vec2> {
		self.lane.sample(t, look_analog)
	}

	pub fn tick_blend(&mut self, dt: f32) {
		let target = if self.lock.is_some() { 1.0 } else { 0.0 };
		self.blend = blend_toward(self.blend, target, ZOOM_BLEND_SPEED, dt);
		if self.lock.is_none() && self.blend < 1e-3 {
			self.blend = 0.0;
			self.last = None;
		}
	}
}

/// Right-stick zoom flick is allowed only when Skill Map / bumper chords are not
/// using the sticks. Tanzer (when present) keeps the right stick as look, so
/// this still fires there; it never reads the left stick.
pub fn zoom_flick_allowed(pad: &VirtualPad) -> bool {
	!pad.pressed(PadButton::BumperFire) && !pad.bumper_chord_held()
}

/// Step 10 → 20 on a matching flick; opposite flick steps in; a new direction
/// starts a 10 m lock.
pub fn next_zoom_lock(current: Option<ZoomLock>, stick: Vec2, world_xz: Vec2) -> Option<ZoomLock> {
	let stick = normalize_or_zero(stick);
	let world_xz = normalize_or_zero(world_xz);
	if stick == Vec2::ZERO || world_xz == Vec2::ZERO {
		return current;
	}
	match current {
		None => Some(ZoomLock { world_xz, stick, distance: ZOOM_NEAR }),
		Some(lock) if lock.stick.dot(stick) >= SAME_DIR_DOT => {
			Some(ZoomLock { world_xz, stick, distance: ZOOM_FAR })
		}
		Some(lock) if lock.stick.dot(stick) <= -SAME_DIR_DOT => {
			if lock.distance >= ZOOM_FAR - 0.1 {
				Some(ZoomLock { world_xz: lock.world_xz, stick: lock.stick, distance: ZOOM_NEAR })
			} else {
				None
			}
		}
		Some(_) => Some(ZoomLock { world_xz, stick, distance: ZOOM_NEAR }),
	}
}

/// Camera-relative stick → world XZ. +X is camera right, +Y is camera forward.
pub fn flick_world_xz(yaw: f32, stick: Vec2) -> Vec2 {
	let yaw = Quat::from_axis_angle(Vec3::Y, yaw);
	let world = yaw * Vec3::X * stick.x + yaw * -Vec3::Z * stick.y;
	normalize_or_zero(Vec2::new(world.x, world.z))
}

pub fn zoom_lock_pose(
	player: Vec3,
	follow: &FollowCamera,
	height_scale: f32,
	lock: &ZoomLock,
) -> PlayerCameraPose {
	let target = player + Vec3::Y * follow.look_height * height_scale;
	let dir = Vec3::new(lock.world_xz.x, 0.0, lock.world_xz.y);
	let eye = target + dir * lock.distance + Vec3::Y * follow.height * height_scale;
	let mut pose = Transform::from_translation(eye);
	pose.look_at(target, Vec3::Y);
	PlayerCameraPose { translation: pose.translation, rotation: pose.rotation }
}

pub fn blend_third_person_pose(
	follow: PlayerCameraPose,
	player: Vec3,
	follow_cam: &FollowCamera,
	height_scale: f32,
	zoom: &ThirdPersonZoom,
) -> PlayerCameraPose {
	let Some(lock) = zoom.lock.or(zoom.last) else {
		return follow;
	};
	if zoom.blend <= 1e-4 {
		return follow;
	}
	follow.interpolate(zoom_lock_pose(player, follow_cam, height_scale, &lock), zoom.blend)
}

/// Look-at point used by the world shapecast. Lerps off the shoulder as the
/// zoom lock takes over so the clamp stays along the visible ray.
pub fn third_person_obstruction_origin(
	player: Vec3,
	yaw: Quat,
	follow: &FollowCamera,
	zoom: Option<&ThirdPersonZoom>,
) -> Vec3 {
	let follow_origin =
		player + Vec3::Y * follow.look_height + yaw * Vec3::X * follow.shoulder_offset;
	let zoom_origin = player + Vec3::Y * follow.look_height;
	match zoom {
		Some(zoom) if zoom.blend > 1e-4 => follow_origin.lerp(zoom_origin, zoom.blend),
		_ => follow_origin,
	}
}

fn blend_toward(current: f32, target: f32, speed: f32, dt: f32) -> f32 {
	let step = 1.0 - (-speed * dt.max(0.0)).exp();
	current + (target - current) * step
}

fn normalize_or_zero(value: Vec2) -> Vec2 {
	let len = value.length();
	if len < 1e-8 {
		Vec2::ZERO
	} else {
		value / len
	}
}

pub(crate) fn detect_zoom_flicks(
	time: Res<Time>,
	pad: Option<Res<VirtualPad>>,
	suppressed: Option<Res<CameraLookSuppressed>>,
	mut cameras: Query<(&CameraController, &mut ThirdPersonZoom), With<FollowCamera>>,
) {
	let Ok((controller, mut zoom)) = cameras.single_mut() else {
		return;
	};
	if suppressed.is_some_and(|suppressed| suppressed.0) || controller.pov != CameraPov::ThirdPerson
	{
		zoom.reset_lane();
		if controller.pov != CameraPov::ThirdPerson {
			zoom.release();
		}
		return;
	}
	let Some(pad) = pad else {
		return;
	};
	if !zoom_flick_allowed(&pad) {
		zoom.reset_lane();
		return;
	}
	if let Some(peak) = zoom.sample_look(time.elapsed_secs(), pad.look_analog) {
		let world = flick_world_xz(controller.yaw, peak);
		let next = next_zoom_lock(zoom.lock, peak, world);
		zoom.apply(next);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn lock(stick: Vec2, distance: f32) -> ZoomLock {
		ZoomLock { world_xz: Vec2::X, stick, distance }
	}

	#[test]
	fn first_flick_locks_ten_metres() {
		let next = next_zoom_lock(None, Vec2::X, Vec2::X).expect("lock");
		assert!((next.distance - ZOOM_NEAR).abs() < 1e-5);
	}

	#[test]
	fn same_direction_steps_out_to_twenty() {
		let near = next_zoom_lock(None, Vec2::X, Vec2::X);
		let far = next_zoom_lock(near, Vec2::X, Vec2::X).expect("far");
		assert!((far.distance - ZOOM_FAR).abs() < 1e-5);
	}

	#[test]
	fn opposite_flick_steps_in_then_releases() {
		let far = Some(lock(Vec2::X, ZOOM_FAR));
		let near = next_zoom_lock(far, -Vec2::X, -Vec2::X).expect("near");
		assert!((near.distance - ZOOM_NEAR).abs() < 1e-5);
		assert!(next_zoom_lock(Some(near), -Vec2::X, -Vec2::X).is_none());
	}

	#[test]
	fn a_new_direction_restarts_at_ten() {
		let far = Some(lock(Vec2::X, ZOOM_FAR));
		let next = next_zoom_lock(far, Vec2::Y, Vec2::Y).expect("turn");
		assert!((next.distance - ZOOM_NEAR).abs() < 1e-5);
		assert!(next.stick.y > 0.9);
	}

	#[test]
	fn stick_right_is_camera_right() {
		let xz = flick_world_xz(0.0, Vec2::X);
		assert!((xz - Vec2::X).length() < 1e-5);
	}

	#[test]
	fn stick_up_is_camera_forward() {
		let xz = flick_world_xz(0.0, Vec2::Y);
		assert!((xz - Vec2::new(0.0, -1.0)).length() < 1e-5);
	}

	#[test]
	fn rb_and_bumper_chord_block_zoom_flicks() {
		let mut pad = VirtualPad::default();
		assert!(zoom_flick_allowed(&pad));
		pad.begin_frame();
		pad.hold_digital(PadButton::BumperFire);
		pad.finish_digital();
		assert!(!zoom_flick_allowed(&pad));
		pad.begin_frame();
		pad.hold_digital(PadButton::BumperFocus);
		pad.hold_digital(PadButton::BumperFire);
		pad.finish_digital();
		assert!(!zoom_flick_allowed(&pad));
	}

	#[test]
	fn zoom_pose_looks_at_the_player_from_the_lock() {
		let follow = FollowCamera::default();
		let lock = ZoomLock { world_xz: Vec2::X, stick: Vec2::X, distance: ZOOM_NEAR };
		let pose = zoom_lock_pose(Vec3::ZERO, &follow, 1.0, &lock);
		assert!((pose.translation.x - ZOOM_NEAR).abs() < 1e-4);
		assert!(pose.translation.y > 0.0);
		let forward = pose.rotation * -Vec3::Z;
		assert!(forward.x < 0.0, "looks back toward the player, forward={forward}");
	}

	#[test]
	fn blend_eases_instead_of_snapping() {
		let mut zoom = ThirdPersonZoom::default();
		zoom.apply(next_zoom_lock(None, Vec2::X, Vec2::X));
		zoom.tick_blend(1.0 / 60.0);
		assert!(zoom.blend > 0.0 && zoom.blend < 1.0);
		zoom.release();
		let up = zoom.blend;
		zoom.tick_blend(1.0 / 60.0);
		assert!(zoom.blend < up);
	}
}
