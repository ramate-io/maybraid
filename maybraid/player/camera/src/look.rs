//! Look intents, POV, body cone, and copy onto [`PlayerLook`].

use crate::{CameraInputSource, CameraTarget, FollowCamera};
use bevy::prelude::*;
use characters::CharacterHeading;
use maybraid_character_controller::{ActorControlEvent, ActorIntentEvent, CharacterIntent};
use maybraid_virtual_pad_controller::PrimaryVirtualPad;
use player::{
	clamp_aim_pitch, wrap_pi, CameraFollow, PlayerLook, PlayerOpticAds, PlayerVisual,
	PlayerYawOwner,
};

/// When `true`, [`CharacterIntent::SwapPov`] is ignored (in-game inventory edit).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CameraPovLocked(pub bool);

/// When `true`, look / focus / ADS do not drive the follow camera (world map).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CameraLookSuppressed(pub bool);

/// Recoil step produced from [`firearms::WeaponFired`]. Camera is the only
/// writer of follow yaw / pitch for this kick.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct CameraRecoilHandoff {
	shooter: Entity,
	step: Vec2,
	follow: bool,
}

impl CameraRecoilHandoff {
	pub const fn new(shooter: Entity, step: Vec2, follow: bool) -> Self {
		Self { shooter, step, follow }
	}

	pub const fn shooter(self) -> Entity {
		self.shooter
	}

	pub const fn step(self) -> Vec2 {
		self.step
	}

	pub const fn follows_camera(self) -> bool {
		self.follow
	}
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CameraPov {
	#[default]
	ThirdPerson,
	FirstPerson,
	/// North-up overhead. Same `Camera3d` as orbit / hip-fire.
	Map,
}

impl CameraPov {
	pub fn toggle(&mut self) {
		*self = match self {
			Self::ThirdPerson => Self::FirstPerson,
			Self::FirstPerson => Self::ThirdPerson,
			Self::Map => Self::Map,
		};
	}

	pub fn is_map(self) -> bool {
		self == Self::Map
	}

	pub fn is_first_person(self) -> bool {
		self == Self::FirstPerson
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
	/// XZ the overhead map looks at. Follow writes this pose when [`CameraPov::Map`].
	pub map_focus: Vec2,
	pub map_height: f32,
	pub map_ground_y: f32,
	/// POV restored when leaving [`CameraPov::Map`].
	pub resume_pov: CameraPov,
}

impl Default for CameraController {
	fn default() -> Self {
		Self {
			yaw: 0.0,
			pitch: 0.0,
			pov: CameraPov::ThirdPerson,
			focus: 0.0,
			ads: 0.0,
			focus_blend: 0.0,
			map_focus: Vec2::ZERO,
			map_height: 420.0,
			map_ground_y: 0.0,
			resume_pov: CameraPov::ThirdPerson,
		}
	}
}

impl CameraController {
	pub fn enter_map(&mut self, focus: Vec2, height: f32, ground_y: f32) {
		if !self.pov.is_map() {
			self.resume_pov = self.pov;
		}
		self.pov = CameraPov::Map;
		self.map_focus = focus;
		self.map_height = height.max(1.0);
		self.map_ground_y = ground_y;
		self.focus = 0.0;
		self.ads = 0.0;
	}

	pub fn exit_map(&mut self) {
		if self.pov.is_map() {
			self.pov = match self.resume_pov {
				CameraPov::Map => CameraPov::ThirdPerson,
				resume => resume,
			};
		}
	}

	/// New body after death. Leave the map and start in third person.
	pub fn begin_life(&mut self) {
		self.exit_map();
		self.pov = CameraPov::ThirdPerson;
		self.resume_pov = CameraPov::ThirdPerson;
		self.focus = 0.0;
		self.ads = 0.0;
		self.focus_blend = 0.0;
		self.pitch = -0.12;
	}
}

pub fn apply_recoil_handoff(
	mut handoffs: MessageReader<CameraRecoilHandoff>,
	mut cameras: Query<(&CameraTarget, &mut CameraController), With<FollowCamera>>,
	mut looks: Query<&mut PlayerLook>,
) {
	for handoff in handoffs.read() {
		if handoff.follows_camera() {
			for (target, mut camera) in &mut cameras {
				if target.0 != Some(handoff.shooter()) {
					continue;
				}
				camera.yaw += handoff.step().x;
				camera.pitch = clamp_aim_pitch(camera.pitch + handoff.step().y);
			}
		}
		if let Ok(mut look) = looks.get_mut(handoff.shooter()) {
			look.yaw += handoff.step().x;
			look.pitch = clamp_aim_pitch(look.pitch + handoff.step().y);
		}
	}
}

/// A delivered neutral frame releases focus and ADS for its own camera.
pub(crate) fn reset_look_frame(
	event: On<ActorControlEvent>,
	primary: Option<Res<PrimaryVirtualPad>>,
	mut cameras: Query<
		(&mut CameraTarget, Option<&CameraInputSource>, &mut CameraController),
		With<FollowCamera>,
	>,
) {
	for (mut target, source, mut controller) in &mut cameras {
		if source
			.is_some_and(|source| source_matches(source, event.input.source(), primary.as_deref()))
			|| target.0 == Some(event.entity)
		{
			target.0 = Some(event.entity);
			controller.focus = 0.0;
			controller.ads = 0.0;
		}
	}
}

/// Apply one already-addressed semantic look event to matching cameras.
pub(crate) fn apply_look_intent(
	event: On<ActorIntentEvent>,
	primary: Option<Res<PrimaryVirtualPad>>,
	locked: Option<Res<CameraPovLocked>>,
	suppressed: Option<Res<CameraLookSuppressed>>,
	optic_users: Query<(), With<PlayerOpticAds>>,
	mut cameras: Query<
		(
			&mut CameraTarget,
			Option<&CameraInputSource>,
			&mut CameraController,
			&FollowCamera,
			Option<&Projection>,
		),
		With<FollowCamera>,
	>,
) {
	if suppressed.is_some_and(|suppressed| suppressed.0) {
		return;
	}
	for (mut target, source, mut controller, follow, projection) in &mut cameras {
		if !source
			.is_some_and(|source| source_matches(source, event.input.source(), primary.as_deref()))
			&& target.0 != Some(event.entity)
		{
			continue;
		}
		target.0 = Some(event.entity);
		if controller.pov.is_map() {
			continue;
		}
		match *event.input.payload() {
			CharacterIntent::Look(value) => {
				let fov = match projection {
					Some(Projection::Perspective(perspective)) => perspective.fov,
					_ => follow.hip_fov(controller.pov),
				};
				let sensitivity = follow.look_sensitivity(controller.pov, fov);
				controller.yaw -= value.x * sensitivity;
				controller.pitch = clamp_aim_pitch(controller.pitch - value.y * sensitivity);
			}
			CharacterIntent::Focus(_) | CharacterIntent::Ads(_)
				if !optic_users.contains(event.entity) =>
			{
				controller.focus = 0.0;
				controller.ads = 0.0;
				controller.focus_blend = 0.0;
			}
			CharacterIntent::Focus(value) => {
				controller.focus = controller.focus.max(value).clamp(0.0, 1.0)
			}
			CharacterIntent::Ads(value) => {
				controller.ads = controller.ads.max(value).clamp(0.0, 1.0)
			}
			CharacterIntent::SkillMap => controller.ads = 0.0,
			CharacterIntent::SwapPov if !locked.as_ref().is_some_and(|locked| locked.0) => {
				controller.pov.toggle()
			}
			_ => {}
		}
	}
}

fn source_matches(
	source: &CameraInputSource,
	delivered: maybraid_character_controller::InputSource,
	primary: Option<&PrimaryVirtualPad>,
) -> bool {
	source.0 == delivered
		|| (source.0 == maybraid_character_controller::InputSource::local_pad()
			&& primary.is_some_and(|primary| delivered.entity() == Some(primary.0)))
}

pub(crate) fn sync_player_look(
	cameras: Query<(&CameraTarget, &CameraController), With<FollowCamera>>,
	mut looks: Query<&mut PlayerLook, With<CameraFollow>>,
) {
	for (target, controller) in &cameras {
		let Some(actor) = target.0 else { continue };
		let Ok(mut look) = looks.get_mut(actor) else { continue };
		look.yaw = controller.yaw;
		look.pitch = controller.pitch;
		look.first_person = controller.pov.is_first_person();
		look.focus = controller.focus.max(controller.ads);
	}
}

pub(crate) fn sync_yaw_owner(
	cameras: Query<(&CameraTarget, &CameraController), With<FollowCamera>>,
	children: Query<&Children>,
	mut owners: Query<&mut PlayerYawOwner>,
) {
	for (target, controller) in &cameras {
		let Some(follower) = target.0 else { continue };
		let owner = match controller.pov {
			CameraPov::FirstPerson => PlayerYawOwner::Look,
			CameraPov::ThirdPerson | CameraPov::Map => PlayerYawOwner::Wish,
		};
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
	mut cameras: Query<(&CameraTarget, &mut CameraController, &FollowCamera), With<FollowCamera>>,
	followers: Query<(), With<CameraFollow>>,
	mut visuals: Query<
		(Entity, &ChildOf, &mut Transform, &mut CharacterHeading),
		(With<PlayerVisual>, Without<Camera3d>),
	>,
	owners: Query<&PlayerYawOwner>,
) {
	for (target, mut controller, follow) in &mut cameras {
		let Some(actor) = target.0 else { continue };
		if !followers.contains(actor) || !controller.pov.is_first_person() {
			continue;
		}
		for (entity, parent, mut visual, mut heading) in &mut visuals {
			if parent.parent() != actor
				|| owners.get(entity).ok().copied().unwrap_or(PlayerYawOwner::Look)
					!= PlayerYawOwner::Look
			{
				continue;
			}
			let body = body_yaw(&heading, &visual);
			let target = follow_body_yaw(controller.yaw, body, follow.max_look_yaw);
			let step = wrap_pi(target - body);
			let max_step = follow.body_turn_rate * time.delta_secs();
			let applied = step.abs().min(max_step).copysign(step);
			if applied.abs() > 1e-5 {
				set_body_yaw(&mut heading, &mut visual, body + applied);
			}
			controller.yaw =
				clamp_look_yaw(controller.yaw, body_yaw(&heading, &visual), follow.max_look_yaw);
			break;
		}
	}
}

fn body_yaw(heading: &CharacterHeading, visual: &Transform) -> f32 {
	camera_yaw_of_forward(heading.facing(visual))
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

fn follow_body_yaw(look_yaw: f32, body_yaw: f32, max_delta: f32) -> f32 {
	let delta = wrap_pi(look_yaw - body_yaw);
	look_yaw - delta.clamp(-max_delta, max_delta)
}

fn clamp_look_yaw(look_yaw: f32, body_yaw: f32, max_delta: f32) -> f32 {
	let delta = wrap_pi(look_yaw - body_yaw);
	body_yaw + delta.clamp(-max_delta, max_delta)
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_character_controller::{ActorTarget, InputSource, Routed};
	use std::f32::consts::FRAC_PI_2;

	fn send(app: &mut App, actor: Entity, intent: CharacterIntent) {
		app.world_mut().trigger(ActorIntentEvent {
			entity: actor,
			input: Routed::to_actor(InputSource::local_pad(), ActorTarget::new(actor), intent),
		});
	}

	#[test]
	fn focus_and_ads_require_the_addressed_actors_held_firearm() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_observer(apply_look_intent);
		let armed = app.world_mut().spawn(PlayerOpticAds).id();
		let unarmed = app.world_mut().spawn_empty().id();
		let first = app
			.world_mut()
			.spawn((
				FollowCamera::default(),
				CameraTarget(Some(armed)),
				CameraController::default(),
			))
			.id();
		let second = app
			.world_mut()
			.spawn((
				FollowCamera::default(),
				CameraTarget(Some(unarmed)),
				CameraController::default(),
			))
			.id();
		for intent in [CharacterIntent::Focus(1.0), CharacterIntent::Ads(1.0)] {
			send(&mut app, armed, intent);
			send(&mut app, unarmed, intent);
		}
		let first_controller = app
			.world()
			.get::<CameraController>(first)
			.ok_or_else(|| anyhow::anyhow!("armed camera"))?;
		assert_eq!(first_controller.focus, 1.0);
		assert_eq!(first_controller.ads, 1.0);
		let second_controller = app
			.world()
			.get::<CameraController>(second)
			.ok_or_else(|| anyhow::anyhow!("unarmed camera"))?;
		assert_eq!(second_controller.focus, 0.0);
		assert_eq!(second_controller.ads, 0.0);
		app.world_mut().entity_mut(armed).remove::<PlayerOpticAds>();
		send(&mut app, armed, CharacterIntent::Focus(1.0));
		let first_controller = app
			.world()
			.get::<CameraController>(first)
			.ok_or_else(|| anyhow::anyhow!("camera after drop"))?;
		assert_eq!(first_controller.focus, 0.0);
		assert_eq!(first_controller.ads, 0.0);
		assert_eq!(first_controller.focus_blend, 0.0);
		Ok(())
	}

	#[test]
	fn look_and_recoil_touch_only_the_target_camera() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_observer(apply_look_intent).add_message::<CameraRecoilHandoff>();
		let a = app.world_mut().spawn(PlayerLook::default()).id();
		let b = app.world_mut().spawn(PlayerLook::default()).id();
		let camera_a = app
			.world_mut()
			.spawn((FollowCamera::default(), CameraTarget(Some(a)), CameraController::default()))
			.id();
		let camera_b = app
			.world_mut()
			.spawn((FollowCamera::default(), CameraTarget(Some(b)), CameraController::default()))
			.id();
		send(&mut app, a, CharacterIntent::Look(Vec2::X));
		assert!(
			app.world()
				.get::<CameraController>(camera_a)
				.ok_or_else(|| anyhow::anyhow!("first camera"))?
				.yaw < 0.0
		);
		assert_eq!(
			app.world()
				.get::<CameraController>(camera_b)
				.ok_or_else(|| anyhow::anyhow!("second camera"))?
				.yaw,
			0.0
		);
		app.world_mut()
			.write_message(CameraRecoilHandoff::new(b, Vec2::new(0.1, 0.2), true));
		use bevy::ecs::system::RunSystemOnce;
		app.world_mut()
			.run_system_once(apply_recoil_handoff)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			app.world()
				.get::<CameraController>(camera_a)
				.ok_or_else(|| anyhow::anyhow!("first camera"))?
				.pitch,
			0.0
		);
		assert_eq!(
			app.world()
				.get::<CameraController>(camera_b)
				.ok_or_else(|| anyhow::anyhow!("second camera"))?
				.pitch,
			0.2
		);
		Ok(())
	}

	#[test]
	fn locked_pov_ignores_swap() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_observer(apply_look_intent).insert_resource(CameraPovLocked(true));
		let actor = app.world_mut().spawn_empty().id();
		let camera = app
			.world_mut()
			.spawn((
				FollowCamera::default(),
				CameraTarget(Some(actor)),
				CameraController::default(),
			))
			.id();
		send(&mut app, actor, CharacterIntent::SwapPov);
		assert_eq!(
			app.world()
				.get::<CameraController>(camera)
				.ok_or_else(|| anyhow::anyhow!("camera"))?
				.pov,
			CameraPov::ThirdPerson
		);
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
		let mut app = App::new();
		app.add_observer(apply_look_intent);
		let actor = app.world_mut().spawn_empty().id();
		let camera = app
			.world_mut()
			.spawn((
				FollowCamera::default(),
				CameraTarget(Some(actor)),
				Projection::Perspective(PerspectiveProjection { fov, ..default() }),
				CameraController { pov, ..default() },
			))
			.id();
		send(&mut app, actor, CharacterIntent::Look(Vec2::X));
		Ok(-app
			.world()
			.get::<CameraController>(camera)
			.ok_or_else(|| anyhow::anyhow!("camera"))?
			.yaw)
	}

	#[test]
	fn aiming_down_sights_scales_look_by_zoom() -> anyhow::Result<()> {
		let follow = FollowCamera::default();
		let hip = yaw_after_look(CameraPov::FirstPerson, follow.first_person_fov)?;
		let sight = yaw_after_look(CameraPov::FirstPerson, follow.sight_fov)?;
		let zoom = (0.5 * follow.sight_fov).tan() / (0.5 * follow.first_person_fov).tan();
		assert!((hip - follow.sensitivity).abs() < 1e-6);
		assert!((sight - follow.sensitivity * zoom).abs() < 1e-6);
		Ok(())
	}
}
