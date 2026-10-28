//! Default follow + optional [`PlayerCameraAim`] blend.

use bevy::prelude::*;
use bevy::transform::helper::TransformHelper;
use characters::LocomotionCapsule;
use characters::{hide_socketed_parts, BoneMap, CharacterMembers, CharacterPartSlot, PartNode};
use player::{
	CameraFollow, PlayerCameraAim, PlayerCameraPose, PlayerLook, PlayerVisual,
	RestLocomotionCapsule,
};

use crate::look::{CameraController, CameraLookSuppressed, CameraPov};
use crate::{CameraTarget, FollowCamera};
use std::collections::HashMap;

/// A short-lived, camera-only rotation applied to a follow pose.
///
/// Values are local yaw/pitch in radians. This message never changes look or aim state.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct CameraPresentationImpulse {
	pub target: Entity,
	pub rotation: Vec2,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct PresentationState {
	target: Option<Entity>,
	rotation: Vec2,
}

const PRESENTATION_ROTATION_CAP: f32 = 1.5_f32.to_radians();
const PRESENTATION_DECAY_SECS: f32 = 0.15;

pub(crate) fn follow_character_camera(
	time: Res<Time>,
	real_time: Res<Time<Real>>,
	look_suppressed: Option<Res<CameraLookSuppressed>>,
	mut impulses: MessageReader<CameraPresentationImpulse>,
	mut presentation: Local<HashMap<Entity, PresentationState>>,
	players: Query<
		(
			Entity,
			&PlayerCameraAim,
			&PlayerLook,
			Option<&LocomotionCapsule>,
			Option<&RestLocomotionCapsule>,
		),
		(With<CameraFollow>, Without<Camera3d>),
	>,
	visuals: Query<(&ChildOf, &CharacterMembers), With<PlayerVisual>>,
	maps: Query<&BoneMap>,
	mut transforms: ParamSet<(
		TransformHelper,
		Query<
			(Entity, &CameraTarget, &mut Transform, &mut CameraController, &FollowCamera),
			With<Camera3d>,
		>,
	)>,
) {
	let pending: Vec<_> = impulses.read().copied().collect();
	let cameras: Vec<_> = transforms
		.p1()
		.iter_mut()
		.map(|(entity, target, _, _, _)| (entity, target.0))
		.collect();
	for &(camera_entity, target) in &cameras {
		let state = presentation.entry(camera_entity).or_default();
		if state.target != target {
			state.target = target;
			state.rotation = Vec2::ZERO;
		}
		state.decay(real_time.delta_secs());
		{
			let mut cameras = transforms.p1();
			let Ok((_, _, mut camera_transform, controller, _)) = cameras.get_mut(camera_entity)
			else {
				continue;
			};
			if controller.pov.is_map() {
				state.rotation = Vec2::ZERO;
				*camera_transform =
					map_pose(controller.map_focus, controller.map_ground_y, controller.map_height)
						.transform();
				continue;
			}
		}
		let Some(actor) = target else { continue };
		let Ok((_, aim, look, live_hull, rest_hull)) = players.get(actor) else { continue };
		let height_scale = stance_height_scale(rest_hull, live_hull);
		let current = transforms.p0();
		let Ok(player) = current.compute_global_transform(actor) else { continue };
		let head = head_camera_translation(actor, &visuals, &maps, &current);
		drop(current);
		let mut camera = transforms.p1();
		let Ok((_, _, mut camera_transform, mut controller, follow)) =
			camera.get_mut(camera_entity)
		else {
			continue;
		};
		let look_suppressed = look_suppressed.as_deref().is_some_and(|suppressed| suppressed.0);
		if look_suppressed {
			state.rotation = Vec2::ZERO;
		}
		accumulate_presentation(state, actor, controller.pov, look_suppressed, &pending);

		let yaw = Quat::from_axis_angle(Vec3::Y, controller.yaw);
		let pitch = Quat::from_axis_angle(Vec3::X, controller.pitch);
		let look_rotation = yaw * pitch;

		let focus_target = if look.first_person {
			aim.focus.max(controller.focus).max(controller.ads)
		} else {
			0.0
		};
		controller.focus_blend = focus_blend_toward(
			controller.focus_blend,
			focus_target,
			follow.focus_blend_speed,
			time.delta_secs(),
		);

		let mut pose = match controller.pov {
			CameraPov::ThirdPerson => {
				third_person_pose(player.translation(), yaw, look_rotation, follow, height_scale)
			}
			CameraPov::FirstPerson => {
				first_person_pose(player.translation(), head, look_rotation, follow, height_scale)
			}
			CameraPov::Map => {
				map_pose(controller.map_focus, controller.map_ground_y, controller.map_height)
			}
		};
		if controller.pov.is_first_person() {
			if let Some(sight) = aim.pose {
				pose = pose.interpolate(sight, controller.focus_blend);
			}
		}
		if !controller.pov.is_map() && !look_suppressed {
			pose.rotation *=
				Quat::from_rotation_y(state.rotation.x) * Quat::from_rotation_x(state.rotation.y);
		}
		*camera_transform = pose.transform();
	}
	presentation.retain(|entity, _| cameras.iter().any(|(camera, _)| camera == entity));
}

impl PresentationState {
	fn add(&mut self, rotation: Vec2) {
		self.rotation = (self.rotation + rotation)
			.clamp(Vec2::splat(-PRESENTATION_ROTATION_CAP), Vec2::splat(PRESENTATION_ROTATION_CAP));
	}

	fn decay(&mut self, delta_secs: f32) {
		self.rotation *= (-delta_secs.max(0.0) / PRESENTATION_DECAY_SECS).exp();
		if self.rotation.length_squared() < 1e-8 {
			self.rotation = Vec2::ZERO;
		}
	}
}

fn accumulate_presentation(
	state: &mut PresentationState,
	actor: Entity,
	pov: CameraPov,
	look_suppressed: bool,
	pending: &[CameraPresentationImpulse],
) {
	if pov.is_map() || look_suppressed {
		return;
	}
	for impulse in pending.iter().filter(|impulse| impulse.target == actor) {
		state.add(impulse.rotation);
	}
}

fn stance_height_scale(
	rest: Option<&RestLocomotionCapsule>,
	live: Option<&LocomotionCapsule>,
) -> f32 {
	let Some(rest) = rest else {
		return 1.0;
	};
	let Some(live) = live else {
		return 1.0;
	};
	(live.half_height() / rest.0.half_height().max(1e-4)).clamp(0.15, 1.0)
}

fn third_person_pose(
	player: Vec3,
	yaw: Quat,
	look_rotation: Quat,
	follow: &FollowCamera,
	height_scale: f32,
) -> PlayerCameraPose {
	let offset = look_rotation * Vec3::new(0.0, 0.0, follow.distance)
		+ Vec3::Y * follow.height * height_scale;
	let target = player
		+ Vec3::Y * follow.look_height * height_scale
		+ yaw * Vec3::X * follow.shoulder_offset;
	let mut pose = Transform::from_translation(target + offset);
	pose.look_at(target, Vec3::Y);
	PlayerCameraPose { translation: pose.translation, rotation: pose.rotation }
}

pub fn map_pose(focus: Vec2, ground_y: f32, height: f32) -> PlayerCameraPose {
	let look = Vec3::new(focus.x, ground_y, focus.y);
	let eye = look + Vec3::Y * height.max(1.0);
	let mut pose = Transform::from_translation(eye);
	pose.look_at(look, Vec3::Z);
	PlayerCameraPose { translation: pose.translation, rotation: pose.rotation }
}

fn first_person_pose(
	player: Vec3,
	head: Option<Vec3>,
	look_rotation: Quat,
	follow: &FollowCamera,
	height_scale: f32,
) -> PlayerCameraPose {
	let head_translation =
		head.unwrap_or(player + Vec3::Y * (follow.height + follow.look_height) * height_scale);
	PlayerCameraPose {
		translation: head_translation + look_rotation * -Vec3::Z * follow.eye_forward,
		rotation: look_rotation,
	}
}

pub fn sync_camera_fov(
	followers: Query<&PlayerCameraAim, With<CameraFollow>>,
	mut cameras: Query<
		(&CameraTarget, &CameraController, &FollowCamera, &mut Projection),
		With<FollowCamera>,
	>,
) {
	for (target, controller, follow, mut projection) in &mut cameras {
		let aim = target.0.and_then(|actor| followers.get(actor).ok());
		if aim.is_none() && !controller.pov.is_map() {
			continue;
		}
		let Projection::Perspective(perspective) = projection.as_mut() else { continue };
		let sight_fov = aim.and_then(|aim| aim.sight_fov);
		let optic_zoom = controller.focus > 1e-4;
		perspective.fov =
			vertical_fov(controller.pov, controller.focus_blend, follow, sight_fov, optic_zoom);
	}
}

fn vertical_fov(
	pov: CameraPov,
	focus_blend: f32,
	follow: &FollowCamera,
	sight_fov: Option<f32>,
	optic_zoom: bool,
) -> f32 {
	let ads_fov = if optic_zoom { sight_fov.unwrap_or(follow.sight_fov) } else { follow.sight_fov };
	let hip = follow.hip_fov(pov);
	match pov {
		CameraPov::ThirdPerson | CameraPov::Map => hip,
		CameraPov::FirstPerson => hip + (ads_fov - hip) * focus_blend,
	}
}

fn focus_blend_toward(current: f32, target: f32, speed: f32, dt: f32) -> f32 {
	let step = 1.0 - (-speed * dt.max(0.0)).exp();
	current + (target - current) * step
}

pub fn sync_first_person_head_visibility(
	followers: Query<(), With<CameraFollow>>,
	cameras: Query<(&CameraTarget, &CameraController), With<FollowCamera>>,
	visuals: Query<(&ChildOf, &CharacterMembers), With<PlayerVisual>>,
	parts: Query<&PartNode>,
	mut visibilities: Query<&mut Visibility>,
) {
	for (parent, members) in &visuals {
		let hidden = cameras.iter().any(|(target, controller)| {
			target.0 == Some(parent.parent())
				&& followers.contains(parent.parent())
				&& controller.pov.is_first_person()
		});
		hide_socketed_parts(
			members,
			&parts,
			&mut visibilities,
			CharacterPartSlot::hides_in_first_person,
			hidden,
		);
	}
}

fn head_camera_translation(
	actor: Entity,
	visuals: &Query<(&ChildOf, &CharacterMembers), With<PlayerVisual>>,
	maps: &Query<&BoneMap>,
	transforms: &TransformHelper,
) -> Option<Vec3> {
	let (_, members) = visuals.iter().find(|(parent, _)| parent.parent() == actor)?;
	let left = member_landmark_translation(members.iter(), maps, transforms, "eye_socket.L")?;
	let right = member_landmark_translation(members.iter(), maps, transforms, "eye_socket.R")?;
	Some((left + right) * 0.5)
}

fn member_landmark_translation(
	members: impl Iterator<Item = Entity>,
	maps: &Query<&BoneMap>,
	transforms: &TransformHelper,
	name: &str,
) -> Option<Vec3> {
	for member in members {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let Some(&entity) = map.by_name.get(name) else {
			continue;
		};
		if let Ok(global) = transforms.compute_global_transform(entity) {
			return Some(global.translation());
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::time::Duration;

	#[test]
	fn follow_impulse_moves_only_its_actor_and_neutralizes_without_changing_aim(
	) -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_message::<CameraPresentationImpulse>()
			.init_resource::<Time>()
			.init_resource::<Time<Real>>()
			.init_resource::<CameraLookSuppressed>()
			.add_systems(Update, follow_character_camera);

		let actor_a = app
			.world_mut()
			.spawn((
				CameraFollow,
				Transform::from_xyz(0.0, 0.0, 0.0),
				GlobalTransform::default(),
				PlayerCameraAim::default(),
				PlayerLook::default(),
			))
			.id();
		let actor_b = app
			.world_mut()
			.spawn((
				CameraFollow,
				Transform::from_xyz(0.0, 0.0, 0.0),
				GlobalTransform::default(),
				PlayerCameraAim::default(),
				PlayerLook::default(),
			))
			.id();
		let controller_a_yaw = 0.4;
		let controller_a_pitch = -0.2;
		let camera_a = app
			.world_mut()
			.spawn((
				Camera3d::default(),
				FollowCamera::default(),
				CameraTarget(Some(actor_a)),
				CameraController { yaw: controller_a_yaw, pitch: controller_a_pitch, ..default() },
				Transform::default(),
			))
			.id();
		let controller_b_yaw = -0.3;
		let controller_b_pitch = 0.1;
		let camera_b = app
			.world_mut()
			.spawn((
				Camera3d::default(),
				FollowCamera::default(),
				CameraTarget(Some(actor_b)),
				CameraController { yaw: controller_b_yaw, pitch: controller_b_pitch, ..default() },
				Transform::default(),
			))
			.id();

		app.world_mut()
			.resource_mut::<Time<Real>>()
			.advance_by(Duration::from_millis(16));
		app.update();
		let neutral_a = *app
			.world()
			.get::<Transform>(camera_a)
			.ok_or_else(|| anyhow::anyhow!("missing camera A transform"))?;
		let neutral_b = *app
			.world()
			.get::<Transform>(camera_b)
			.ok_or_else(|| anyhow::anyhow!("missing camera B transform"))?;

		app.world_mut().write_message(CameraPresentationImpulse {
			target: actor_a,
			rotation: Vec2::new(0.02, -0.01),
		});
		app.world_mut()
			.resource_mut::<Time<Real>>()
			.advance_by(Duration::from_millis(16));
		app.update();
		let kicked_a = *app
			.world()
			.get::<Transform>(camera_a)
			.ok_or_else(|| anyhow::anyhow!("missing camera A transform"))?;
		let untouched_b = *app
			.world()
			.get::<Transform>(camera_b)
			.ok_or_else(|| anyhow::anyhow!("missing camera B transform"))?;
		assert!(kicked_a.rotation.angle_between(neutral_a.rotation) > 0.01);
		assert_eq!(untouched_b.rotation, neutral_b.rotation);
		assert!(app.world().get::<CameraController>(camera_a).is_some_and(|controller| {
			controller.yaw == controller_a_yaw && controller.pitch == controller_a_pitch
		}));
		assert!(app.world().get::<CameraController>(camera_b).is_some_and(|controller| {
			controller.yaw == controller_b_yaw && controller.pitch == controller_b_pitch
		}));
		assert!(app.world().get::<PlayerLook>(actor_a).is_some_and(|look| {
			look.yaw == 0.0 && look.pitch == 0.0 && !look.first_person && look.focus == 0.0
		}));
		assert!(app.world().get::<PlayerCameraAim>(actor_a).is_some_and(|aim| {
			aim.focus == 0.0 && aim.pose.is_none() && aim.sight_fov.is_none()
		}));

		app.world_mut().entity_mut(camera_a).insert(CameraTarget(Some(actor_b)));
		app.world_mut()
			.resource_mut::<Time<Real>>()
			.advance_by(Duration::from_millis(16));
		app.update();
		let retargeted_a = app
			.world()
			.get::<Transform>(camera_a)
			.ok_or_else(|| anyhow::anyhow!("missing camera A transform"))?;
		let current_b = app
			.world()
			.get::<Transform>(camera_b)
			.ok_or_else(|| anyhow::anyhow!("missing camera B transform"))?;
		assert_eq!(retargeted_a.rotation, neutral_a.rotation);
		assert_eq!(current_b.rotation, neutral_b.rotation);

		app.world_mut().resource_mut::<Time<Real>>().advance_by(Duration::from_secs(10));
		app.update();
		let neutral_after = app
			.world()
			.get::<Transform>(camera_b)
			.ok_or_else(|| anyhow::anyhow!("missing camera B transform"))?;
		assert_eq!(neutral_after.rotation, neutral_b.rotation);
		Ok(())
	}

	#[test]
	fn presentation_is_actor_addressed_bounded_and_decays_to_neutral() {
		let actor = Entity::from_bits(1);
		let other = Entity::from_bits(2);
		let events = [
			CameraPresentationImpulse { target: other, rotation: Vec2::splat(0.02) },
			CameraPresentationImpulse { target: actor, rotation: Vec2::splat(0.02) },
		];
		let mut state = PresentationState::default();
		accumulate_presentation(&mut state, actor, CameraPov::ThirdPerson, false, &events);
		assert_eq!(state.rotation, Vec2::splat(0.02));
		for _ in 0..20 {
			accumulate_presentation(
				&mut state,
				actor,
				CameraPov::ThirdPerson,
				false,
				&[CameraPresentationImpulse { target: actor, rotation: Vec2::splat(0.02) }],
			);
		}
		assert_eq!(state.rotation, Vec2::splat(PRESENTATION_ROTATION_CAP));
		state.decay(10.0);
		assert_eq!(state.rotation, Vec2::ZERO);
	}

	#[test]
	fn map_pov_ignores_presentation_impulses() {
		let actor = Entity::from_bits(1);
		let mut state = PresentationState::default();
		accumulate_presentation(
			&mut state,
			actor,
			CameraPov::Map,
			false,
			&[CameraPresentationImpulse { target: actor, rotation: Vec2::ONE }],
		);
		assert_eq!(state.rotation, Vec2::ZERO);
	}

	#[test]
	fn look_suppression_ignores_new_impulses() {
		let actor = Entity::from_bits(1);
		let mut state = PresentationState::default();
		accumulate_presentation(
			&mut state,
			actor,
			CameraPov::FirstPerson,
			true,
			&[CameraPresentationImpulse { target: actor, rotation: Vec2::ONE }],
		);
		assert_eq!(state.rotation, Vec2::ZERO);
	}

	#[test]
	fn first_person_hides_face_not_body() {
		assert!(CharacterPartSlot::HeadMesh.hides_in_first_person());
		assert!(!CharacterPartSlot::BodyMesh.hides_in_first_person());
	}

	#[test]
	fn first_person_hipfire_is_wider_than_orbit() {
		let follow = FollowCamera::default();
		assert!(
			vertical_fov(CameraPov::FirstPerson, 0.0, &follow, None, false)
				> vertical_fov(CameraPov::ThirdPerson, 0.0, &follow, None, false)
		);
		assert!(
			(vertical_fov(CameraPov::FirstPerson, 1.0, &follow, None, false) - follow.sight_fov)
				.abs() < 1e-5
		);
		let midpoint = vertical_fov(CameraPov::FirstPerson, 0.5, &follow, None, true);
		assert!((midpoint - (follow.first_person_fov + follow.sight_fov) * 0.5).abs() < 1e-5);
	}

	#[test]
	fn held_optic_overrides_iron_sight_fov() {
		let follow = FollowCamera::default();
		let optic = 18.0_f32.to_radians();
		assert!(
			(vertical_fov(CameraPov::FirstPerson, 1.0, &follow, Some(optic), true) - optic).abs()
				< 1e-5
		);
	}

	#[test]
	fn iron_ads_keeps_base_fov_when_an_optic_is_held() {
		let follow = FollowCamera::default();
		let optic = 18.0_f32.to_radians();
		assert!(
			(vertical_fov(CameraPov::FirstPerson, 1.0, &follow, Some(optic), false)
				- follow.sight_fov)
				.abs() < 1e-5
		);
	}

	#[test]
	fn map_pose_looks_down_north_up() {
		let pose = map_pose(Vec2::new(12.0, -8.0), 4.0, 420.0);
		assert!((pose.translation - Vec3::new(12.0, 424.0, -8.0)).length() < 1e-4);
		let forward = pose.rotation * -Vec3::Z;
		assert!(forward.y < -0.99, "map looks down, forward={forward}");
		let up = pose.rotation * Vec3::Y;
		assert!(up.z > 0.99, "map up is world +Z, up={up}");
	}

	#[test]
	fn enter_map_restores_first_person() {
		let mut controller = CameraController { pov: CameraPov::FirstPerson, ..default() };
		controller.enter_map(Vec2::ONE, 200.0, 3.0);
		assert_eq!(controller.pov, CameraPov::Map);
		assert_eq!(controller.resume_pov, CameraPov::FirstPerson);
		controller.exit_map();
		assert_eq!(controller.pov, CameraPov::FirstPerson);
	}

	#[test]
	fn begin_life_leaves_the_map_in_third_person() {
		let mut controller = CameraController { pov: CameraPov::FirstPerson, ..default() };
		controller.enter_map(Vec2::ONE, 200.0, 3.0);
		controller.begin_life();
		assert_eq!(controller.pov, CameraPov::ThirdPerson);
		assert_eq!(controller.resume_pov, CameraPov::ThirdPerson);
		assert_eq!(controller.focus, 0.0);
	}

	#[test]
	fn map_pose_applies_without_a_follow_target() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.init_resource::<Time>();
		world.init_resource::<Time<Real>>();
		world.init_resource::<Messages<CameraPresentationImpulse>>();
		world.spawn((
			Camera3d::default(),
			FollowCamera::default(),
			CameraTarget::default(),
			CameraController {
				pov: CameraPov::Map,
				map_focus: Vec2::new(4.0, 8.0),
				map_height: 200.0,
				map_ground_y: 2.0,
				..default()
			},
			Transform::default(),
		));
		world
			.run_system_once(follow_character_camera)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let transform = world.query::<&Transform>().single(&world)?;
		assert!((transform.translation - Vec3::new(4.0, 202.0, 8.0)).length() < 1e-3);
		Ok(())
	}

	#[test]
	fn focus_blend_moves_smoothly_to_and_from_sight() {
		let speed = FollowCamera::default().focus_blend_speed;
		let focused = focus_blend_toward(0.0, 1.0, speed, 1.0 / 60.0);
		assert!(focused > 0.0 && focused < 1.0);
		let released = focus_blend_toward(focused, 0.0, speed, 1.0 / 60.0);
		assert!(released >= 0.0 && released < focused);
	}
}
