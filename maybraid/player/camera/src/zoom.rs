//! Third-person directional zoom lock, driven by actor-addressed stick throws.

use bevy::prelude::*;
use maybraid_character_controller::{ActorControlSample, InputSource, RoutedActorControl};
use maybraid_virtual_pad_controller::flick::FlickLane;
use player::PlayerCameraPose;

use crate::look::{CameraController, CameraLookSuppressed, CameraPov};
use crate::{CameraTarget, FollowCamera};

pub const ZOOM_NEAR: f32 = 10.0;
pub const ZOOM_FAR: f32 = 20.0;
const ZOOM_BLEND_SPEED: f32 = 6.0;
const SAME_DIR_DOT: f32 = 0.5;

pub fn zoom_flick_allowed(sample: ActorControlSample) -> bool {
	!sample.bumper_focus() && !sample.bumper_fire()
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomLock {
	pub world_xz: Vec2,
	pub stick: Vec2,
	pub distance: f32,
}

#[derive(Component, Clone, Debug, Default)]
pub struct ThirdPersonZoom {
	pub lock: Option<ZoomLock>,
	last: Option<ZoomLock>,
	pub blend: f32,
	lane: FlickLane,
	owner: Option<(Entity, InputSource)>,
}

impl ThirdPersonZoom {
	pub fn reset(&mut self) {
		*self = Self::default();
	}

	pub fn release(&mut self) {
		self.lock = None;
		self.lane.reset();
	}

	pub fn apply(&mut self, next: Option<ZoomLock>) {
		self.lock = next;
		if let Some(lock) = next {
			self.last = Some(lock);
		}
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

pub fn flick_world_xz(yaw: f32, stick: Vec2) -> Vec2 {
	let yaw = Quat::from_rotation_y(yaw);
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
	follow_pose: PlayerCameraPose,
	player: Vec3,
	follow: &FollowCamera,
	height_scale: f32,
	zoom: &ThirdPersonZoom,
) -> PlayerCameraPose {
	let Some(lock) = zoom.lock.or(zoom.last) else { return follow_pose };
	if zoom.blend <= 1e-4 {
		return follow_pose;
	}
	follow_pose.interpolate(zoom_lock_pose(player, follow, height_scale, &lock), zoom.blend)
}

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

pub(crate) fn detect_zoom_flicks(
	time: Res<Time>,
	suppressed: Option<Res<CameraLookSuppressed>>,
	actors: Query<&RoutedActorControl>,
	mut cameras: Query<
		(&CameraTarget, &CameraController, &mut ThirdPersonZoom),
		With<FollowCamera>,
	>,
) {
	for (target, controller, mut zoom) in &mut cameras {
		if zoom.owner.is_some_and(|(actor, _)| target.0 != Some(actor)) {
			zoom.reset();
		}
		if suppressed.as_deref().is_some_and(|value| value.0)
			|| controller.pov != CameraPov::ThirdPerson
		{
			zoom.reset();
			continue;
		}
		let delivered = target.0.and_then(|actor| {
			let routed = actors.get(actor).ok()?;
			routed.addresses(actor).then_some((actor, routed.source()?, routed.payload()?))
		});
		let Some((actor, source, sample)) = delivered else {
			zoom.lane.reset();
			continue;
		};
		let owner = Some((actor, source));
		if zoom.owner != owner {
			zoom.reset();
			zoom.owner = owner;
		}
		if !zoom_flick_allowed(sample) {
			zoom.lane.reset();
			continue;
		}
		if let Some(peak) = zoom.lane.sample(time.elapsed_secs(), sample.look_throw()) {
			let world = flick_world_xz(controller.yaw, peak);
			let next = next_zoom_lock(zoom.lock, peak, world);
			zoom.apply(next);
		}
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

#[cfg(test)]
mod tests {
	use super::*;

	use bevy::ecs::system::RunSystemOnce;
	use maybraid_character_controller::{ActorTarget, Routed};

	fn deliver(world: &mut World, actor: Entity, sample: ActorControlSample) -> anyhow::Result<()> {
		world
			.resource_mut::<Time>()
			.advance_by(std::time::Duration::from_secs_f32(0.04));
		world.entity_mut(actor).insert(RoutedActorControl::new(Routed::to_actor(
			InputSource::local_pad(),
			ActorTarget::new(actor),
			sample,
		)));
		world
			.run_system_once(detect_zoom_flicks)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn routed_flick_honors_bumpers_target_changes_and_map_reset() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		let actor = world.spawn_empty().id();
		let camera = world
			.spawn((
				FollowCamera::default(),
				CameraTarget(Some(actor)),
				CameraController::default(),
			))
			.id();
		for (left, right) in [(true, false), (false, true), (true, true)] {
			deliver(
				&mut world,
				actor,
				ActorControlSample::default().with_bumpers(left, right).with_look_throw(Vec2::X),
			)?;
			deliver(&mut world, actor, ActorControlSample::default())?;
			assert!(world.get::<ThirdPersonZoom>(camera).is_some_and(|zoom| zoom.lock.is_none()));
		}
		deliver(&mut world, actor, ActorControlSample::default().with_look_throw(Vec2::X))?;
		deliver(&mut world, actor, ActorControlSample::default())?;
		assert!(world.get::<ThirdPersonZoom>(camera).is_some_and(|zoom| zoom.lock.is_some()));
		let next = world.spawn_empty().id();
		world.entity_mut(camera).insert(CameraTarget(Some(next)));
		deliver(&mut world, next, ActorControlSample::default())?;
		assert!(world
			.get::<ThirdPersonZoom>(camera)
			.is_some_and(|zoom| zoom.lock.is_none() && zoom.blend == 0.0));
		deliver(&mut world, next, ActorControlSample::default().with_look_throw(Vec2::X))?;
		deliver(&mut world, next, ActorControlSample::default())?;
		assert!(world.get::<ThirdPersonZoom>(camera).is_some_and(|zoom| zoom.lock.is_some()));
		world
			.get_mut::<CameraController>(camera)
			.ok_or_else(|| anyhow::anyhow!("camera"))?
			.enter_map(Vec2::ZERO, 100.0, 0.0);
		deliver(&mut world, next, ActorControlSample::default())?;
		assert!(world
			.get::<ThirdPersonZoom>(camera)
			.is_some_and(|zoom| zoom.lock.is_none() && zoom.blend == 0.0));
		Ok(())
	}

	fn lock(stick: Vec2, distance: f32) -> ZoomLock {
		ZoomLock { world_xz: Vec2::X, stick, distance }
	}

	#[test]
	fn directional_flicks_step_out_step_in_then_release() -> anyhow::Result<()> {
		let near =
			next_zoom_lock(None, Vec2::X, Vec2::X).ok_or_else(|| anyhow::anyhow!("near lock"))?;
		assert_eq!(near.distance, ZOOM_NEAR);
		let far = next_zoom_lock(Some(near), Vec2::X, Vec2::X)
			.ok_or_else(|| anyhow::anyhow!("far lock"))?;
		assert_eq!(far.distance, ZOOM_FAR);
		let near_again = next_zoom_lock(Some(far), -Vec2::X, -Vec2::X)
			.ok_or_else(|| anyhow::anyhow!("step in"))?;
		assert_eq!(near_again.distance, ZOOM_NEAR);
		assert!(next_zoom_lock(Some(near_again), -Vec2::X, -Vec2::X).is_none());
		Ok(())
	}

	#[test]
	fn new_direction_starts_a_near_lock_in_camera_space() -> anyhow::Result<()> {
		let turned = next_zoom_lock(Some(lock(Vec2::X, ZOOM_FAR)), Vec2::Y, Vec2::Y)
			.ok_or_else(|| anyhow::anyhow!("new direction"))?;
		assert_eq!(turned.distance, ZOOM_NEAR);
		let forward = flick_world_xz(0.0, Vec2::Y);
		assert!((forward - Vec2::new(0.0, -1.0)).length() < 1e-5);
		Ok(())
	}

	#[test]
	fn bumper_chord_blocks_zoom_and_release_blends_home() {
		assert!(zoom_flick_allowed(ActorControlSample::default()));
		for (left, right) in [(true, false), (false, true), (true, true)] {
			assert!(!zoom_flick_allowed(ActorControlSample::default().with_bumpers(left, right)));
		}
		let mut zoom = ThirdPersonZoom::default();
		zoom.apply(Some(lock(Vec2::X, ZOOM_NEAR)));
		zoom.tick_blend(1.0 / 60.0);
		zoom.release();
		zoom.tick_blend(1.0);
		assert_eq!(zoom.blend, 0.0);
		assert!(zoom.lock.is_none());
	}

	#[test]
	fn held_look_throw_does_not_count_as_a_flick() {
		let mut lane = FlickLane::default();
		assert!(lane.sample(0.0, Vec2::X).is_none());
		assert!(lane.sample(0.3, Vec2::X).is_none());
		assert!(lane.sample(0.35, Vec2::ZERO).is_none());
	}

	#[test]
	fn lock_blends_in_and_eases_home_after_release() {
		let mut zoom = ThirdPersonZoom::default();
		zoom.apply(Some(lock(Vec2::X, ZOOM_NEAR)));
		zoom.tick_blend(1.0 / 60.0);
		assert!(zoom.blend > 0.0 && zoom.blend < 1.0);
		let before_release = zoom.blend;
		zoom.release();
		zoom.tick_blend(1.0 / 60.0);
		assert!(zoom.blend < before_release);
	}
}
