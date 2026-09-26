//! Look intents, POV, body cone, and copy onto [`PlayerLook`].

use crate::FollowCamera;
use bevy::prelude::*;
use crozon_characters::CharacterHeading;
use maybraid_character_controller::CharacterIntent;
use player::{CameraFollow, PlayerLook, PlayerVisual, PlayerYawOwner};
use std::f32::consts::{FRAC_PI_2, PI};

/// When `true`, [`CharacterIntent::SwapPov`] is ignored (in-game inventory edit).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CameraPovLocked(pub bool);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CameraPov {
	#[default]
	ThirdPerson,
	FirstPerson,
}

impl CameraPov {
	pub fn toggle(&mut self) {
		*self = match self {
			Self::ThirdPerson => Self::FirstPerson,
			Self::FirstPerson => Self::ThirdPerson,
		};
	}
}

#[derive(Component)]
pub struct CameraController {
	pub yaw: f32,
	pub pitch: f32,
	pub pov: CameraPov,
	/// Optic ADS (LT / right mouse). Drives magnified sight FOV when > 0.
	pub focus: f32,
	/// Iron ADS (LB / middle mouse). Pose only; FOV stays [`crate::FollowCamera::sight_fov`].
	pub ads: f32,
	pub focus_blend: f32,
}

pub(crate) fn apply_look_intents(
	mouse: Res<ButtonInput<MouseButton>>,
	locked: Option<Res<CameraPovLocked>>,
	mut intents: MessageReader<CharacterIntent>,
	mut cameras: Query<(&mut CameraController, &FollowCamera, Option<&Projection>), With<Camera3d>>,
) {
	let mut focus = f32::from(mouse.pressed(MouseButton::Right));
	let mut ads = f32::from(mouse.pressed(MouseButton::Middle));
	let mut swap_pov = false;
	let mut skill_map = false;
	for intent in intents.read() {
		match *intent {
			CharacterIntent::Look(value) => {
				if let Ok((mut controller, follow, projection)) = cameras.single_mut() {
					let fov = match projection {
						Some(Projection::Perspective(perspective)) => perspective.fov,
						_ => follow.hip_fov(controller.pov),
					};
					let sensitivity = follow.look_sensitivity(controller.pov, fov);
					controller.yaw -= value.x * sensitivity;
					controller.pitch -= value.y * sensitivity;
					controller.pitch = controller.pitch.clamp(-FRAC_PI_2 + 0.1, FRAC_PI_2 - 0.1);
				}
			}
			CharacterIntent::Focus(value) => focus = focus.max(value),
			CharacterIntent::Ads(value) => ads = ads.max(value),
			CharacterIntent::SkillMap => skill_map = true,
			CharacterIntent::SwapPov => swap_pov = true,
			_ => {}
		}
	}
	if skill_map {
		ads = 0.0;
	}
	if let Ok((mut controller, _, _)) = cameras.single_mut() {
		controller.focus = focus.clamp(0.0, 1.0);
		controller.ads = ads.clamp(0.0, 1.0);
		if swap_pov && !locked.is_some_and(|locked| locked.0) {
			controller.pov.toggle();
		}
	}
}

pub(crate) fn sync_player_look(
	cameras: Query<&CameraController, With<Camera3d>>,
	mut looks: Query<&mut PlayerLook, With<CameraFollow>>,
) {
	let Ok(controller) = cameras.single() else {
		return;
	};
	for mut look in &mut looks {
		look.yaw = controller.yaw;
		look.pitch = controller.pitch;
		look.first_person = controller.pov == CameraPov::FirstPerson;
		look.focus = controller.focus.max(controller.ads);
	}
}

pub(crate) fn sync_yaw_owner(
	cameras: Query<&CameraController, With<Camera3d>>,
	followers: Query<Entity, With<CameraFollow>>,
	children: Query<&Children>,
	mut owners: Query<&mut PlayerYawOwner>,
) {
	let Ok(controller) = cameras.single() else {
		return;
	};
	let owner = match controller.pov {
		CameraPov::FirstPerson => PlayerYawOwner::Look,
		CameraPov::ThirdPerson => PlayerYawOwner::Wish,
	};
	for follower in &followers {
		set_yaw_owner(&mut owners, follower, owner);
		if let Ok(children) = children.get(follower) {
			for child in children.iter() {
				set_yaw_owner(&mut owners, child, owner);
			}
		}
	}
}

fn set_yaw_owner(owners: &mut Query<&mut PlayerYawOwner>, entity: Entity, owner: PlayerYawOwner) {
	if let Ok(mut yaw) = owners.get_mut(entity) {
		*yaw = owner;
	}
}

pub(crate) fn turn_body_with_look(
	time: Res<Time>,
	mut cameras: Query<(&mut CameraController, &FollowCamera), With<Camera3d>>,
	followers: Query<(), With<CameraFollow>>,
	mut visuals: Query<
		(Entity, &mut Transform, &mut CharacterHeading),
		(With<PlayerVisual>, Without<Camera3d>),
	>,
	owners: Query<&PlayerYawOwner>,
) {
	if followers.is_empty() {
		return;
	}
	let Ok((mut controller, follow)) = cameras.single_mut() else {
		return;
	};
	if controller.pov != CameraPov::FirstPerson {
		return;
	}
	let Ok((entity, mut visual, mut heading)) = visuals.single_mut() else {
		return;
	};
	if owners.get(entity).ok().copied().unwrap_or(PlayerYawOwner::Look) != PlayerYawOwner::Look {
		return;
	}

	let body = body_yaw(&mut heading, &visual);
	let target = follow_body_yaw(controller.yaw, body, follow.max_look_yaw);
	let step = wrap_to_pi(target - body);
	let max_step = follow.body_turn_rate * time.delta_secs();
	let applied = step.abs().min(max_step).copysign(step);
	if applied.abs() > 1e-5 {
		set_body_yaw(&mut heading, &mut visual, body + applied);
	}
	controller.yaw =
		clamp_look_yaw(controller.yaw, body_yaw(&mut heading, &visual), follow.max_look_yaw);
}

fn body_yaw(heading: &mut CharacterHeading, visual: &Transform) -> f32 {
	camera_yaw_of_forward(heading.resolve(visual))
}

fn set_body_yaw(heading: &mut CharacterHeading, visual: &mut Transform, yaw: f32) {
	let forward = Quat::from_axis_angle(Vec3::Y, yaw) * -Vec3::Z;
	heading.set(visual, forward);
}

fn camera_yaw_of_forward(dir: Vec3) -> f32 {
	let xz = Vec3::new(dir.x, 0.0, dir.z);
	if xz.length_squared() < 1e-8 {
		0.0
	} else {
		let n = xz.normalize();
		(-n.x).atan2(-n.z)
	}
}

fn wrap_to_pi(angle: f32) -> f32 {
	(angle + PI).rem_euclid(2.0 * PI) - PI
}

fn follow_body_yaw(look_yaw: f32, body_yaw: f32, max_delta: f32) -> f32 {
	let delta = wrap_to_pi(look_yaw - body_yaw);
	look_yaw - delta.clamp(-max_delta, max_delta)
}

fn clamp_look_yaw(look_yaw: f32, body_yaw: f32, max_delta: f32) -> f32 {
	let delta = wrap_to_pi(look_yaw - body_yaw);
	body_yaw + delta.clamp(-max_delta, max_delta)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn pov_toggle_round_trips() {
		let mut pov = CameraPov::ThirdPerson;
		pov.toggle();
		assert_eq!(pov, CameraPov::FirstPerson);
		pov.toggle();
		assert_eq!(pov, CameraPov::ThirdPerson);
	}

	#[test]
	fn locked_pov_ignores_swap() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;
		use maybraid_character_controller::CharacterIntent;

		let mut world = World::new();
		world.init_resource::<ButtonInput<MouseButton>>();
		world.init_resource::<Messages<CharacterIntent>>();
		world.insert_resource(CameraPovLocked(true));
		world.spawn((
			Camera3d::default(),
			FollowCamera::default(),
			CameraController {
				yaw: 0.0,
				pitch: 0.0,
				pov: CameraPov::ThirdPerson,
				focus: 0.0,
				ads: 0.0,
				focus_blend: 0.0,
			},
		));
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::SwapPov);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(apply_look_intents)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let pov = world
			.query::<&CameraController>()
			.iter(&world)
			.next()
			.map(|controller| controller.pov)
			.ok_or_else(|| anyhow::anyhow!("camera"))?;
		assert_eq!(pov, CameraPov::ThirdPerson);
		Ok(())
	}

	#[test]
	fn body_stays_put_inside_look_cone() {
		let max = FollowCamera::default().max_look_yaw;
		let look = -FRAC_PI_2 + max * 0.5;
		let body = -FRAC_PI_2;
		assert!((follow_body_yaw(look, body, max) - body).abs() < 1e-5);
	}

	#[test]
	fn body_follows_when_look_exceeds_cone() {
		let max = FollowCamera::default().max_look_yaw;
		let look = -FRAC_PI_2 + max + 0.4;
		let body = -FRAC_PI_2;
		let target = follow_body_yaw(look, body, max);
		assert!(target < look);
		assert!((clamp_look_yaw(look, target, max) - look).abs() < 1e-4);
	}

	fn yaw_after_look(pov: CameraPov, fov: f32) -> anyhow::Result<f32> {
		use bevy::ecs::system::RunSystemOnce;
		let mut world = World::new();
		world.init_resource::<ButtonInput<MouseButton>>();
		world.init_resource::<Messages<CharacterIntent>>();
		world.spawn((
			Camera3d::default(),
			FollowCamera::default(),
			Projection::Perspective(PerspectiveProjection { fov, ..default() }),
			CameraController { yaw: 0.0, pitch: 0.0, pov, focus: 0.0, ads: 0.0, focus_blend: 0.0 },
		));
		world.write_message(CharacterIntent::Look(Vec2::X));
		world
			.run_system_once(apply_look_intents)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.query::<&CameraController>()
			.iter(&world)
			.next()
			.map(|controller| -controller.yaw)
			.ok_or_else(|| anyhow::anyhow!("camera"))
	}

	#[test]
	fn aiming_down_sights_slows_look_by_the_zoom() -> anyhow::Result<()> {
		let follow = FollowCamera::default();
		let hip = yaw_after_look(CameraPov::FirstPerson, follow.first_person_fov)?;
		assert!((hip - follow.sensitivity).abs() < 1e-6, "hip fire keeps the base sensitivity");
		let sight = yaw_after_look(CameraPov::FirstPerson, follow.sight_fov)?;
		let zoom = (0.5 * follow.sight_fov).tan() / (0.5 * follow.first_person_fov).tan();
		assert!((sight - follow.sensitivity * zoom).abs() < 1e-6, "{sight} vs {zoom}");
		let optic = yaw_after_look(CameraPov::FirstPerson, 18.0_f32.to_radians())?;
		assert!(optic < sight, "a magnified optic turns slower than iron sights");
		let orbit = yaw_after_look(CameraPov::ThirdPerson, follow.third_person_fov)?;
		assert!((orbit - follow.sensitivity).abs() < 1e-6, "third person is unchanged");
		Ok(())
	}

	#[test]
	fn focus_intent_still_writes_controller_focus() {
		let mut app = App::new();
		app.init_resource::<ButtonInput<MouseButton>>()
			.add_message::<CharacterIntent>()
			.add_systems(Update, apply_look_intents);
		let camera = app
			.world_mut()
			.spawn((
				Camera3d::default(),
				CameraController {
					yaw: 0.0,
					pitch: 0.0,
					pov: CameraPov::ThirdPerson,
					focus: 0.0,
					ads: 0.0,
					focus_blend: 0.0,
				},
				FollowCamera::default(),
			))
			.id();
		app.world_mut().write_message(CharacterIntent::Focus(1.0));
		app.update();
		let focus = app.world().get::<CameraController>(camera).map(|controller| controller.focus);
		assert_eq!(focus, Some(1.0));
	}
}
