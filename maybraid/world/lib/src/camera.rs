//! World-specific extensions around the shared player follow camera.

use avian3d::prelude::{Collider, ShapeCastConfig, SpatialQuery, SpatialQueryFilter};
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::prelude::*;
use durham::WorldBaseTerrain;
use fisticuffs_user::{FisticuffsImpact, FisticuffsStrike};
use game_commands::command::TextEntryFocus;
use lod_avian::PhysicsInteractionLayer;
use maybraid_character_controller::{ActorControlStatus, RoutedActorControl};
use maybraid_session::WorldGameplayEnabled;
use maybraid_virtual_pad_controller::PrimaryVirtualPad;
use player::{CameraFollow, Player, PlayerSystems};
use player_camera::{
	spawn_follow_camera, CameraController, CameraLookSuppressed, CameraPov, CameraPovLocked,
	CameraPresentationImpulse, FollowCamera, PlayerCameraSystems,
};
use world_player::{
	player::holding_elevation, Player as VegetationPlayer, PlayerSpawnXz, PlaygroundMode,
};

use crate::control::InventoryEditCameraFollow;
use crate::map_view::{WorldMapSet, WorldMapView};

const CAMERA_COLLISION_RADIUS: f32 = 0.18;
const CAMERA_COLLISION_SKIN: f32 = 0.08;
const CAMERA_COLLISION_MIN: f32 = 0.12;
const WORLD_CAMERA_FAR: f32 = 8_000.0;
const WORLD_CAMERA_NEAR: f32 = 0.1;
const WORLD_FREE_CAMERA_SPEED: f32 = 40.0;

type PlayerFollowQuery<'w, 's> = Query<
	'w,
	's,
	(Entity, &'static Transform),
	(With<Player>, With<CameraFollow>, Without<Camera3d>),
>;
type CameraFollowQuery<'w, 's> = Query<
	'w,
	's,
	(&'static mut Transform, &'static CameraController, &'static FollowCamera),
	(With<Camera3d>, Without<Player>),
>;

#[derive(Component)]
struct WorldFreeCamera {
	speed: f32,
}

fn world_follow_camera() -> FollowCamera {
	FollowCamera { near: WORLD_CAMERA_NEAR, far: WORLD_CAMERA_FAR, ..default() }
}

pub(crate) fn spawn_world_camera(
	mut commands: Commands,
	spawn: Res<PlayerSpawnXz>,
	base: Option<Res<WorldBaseTerrain>>,
) {
	let follow = world_follow_camera();
	let camera = spawn_follow_camera(&mut commands);
	if let Some(xz) = spawn.0 {
		let y = base
			.as_ref()
			.map(|base| holding_elevation(&base.0, xz.x, xz.y))
			.unwrap_or(follow.height);
		let look = Vec3::new(xz.x, y + follow.look_height, xz.y);
		let eye = look + Vec3::new(-follow.distance, follow.height, 0.0);
		commands
			.entity(camera)
			.insert(Transform::from_translation(eye).looking_at(look, Vec3::Y));
	}
	commands.entity(camera).insert((
		follow,
		Projection::Perspective(PerspectiveProjection {
			fov: follow.third_person_fov,
			near: follow.near,
			far: follow.far,
			..default()
		}),
		DistanceFog {
			color: Color::srgba(0.55, 0.65, 0.72, 1.0),
			directional_light_color: Color::srgba(1.0, 0.92, 0.78, 0.35),
			directional_light_exponent: 24.0,
			falloff: FogFalloff::Linear { start: 700.0, end: 4500.0 },
		},
		Msaa::Off,
		DepthPrepass,
		WorldFreeCamera { speed: WORLD_FREE_CAMERA_SPEED },
	));
}

/// Enable shared gameplay follow only while the world owns gameplay input.
/// One writer for follow, POV lock, and look suppress.
pub(crate) fn sync_camera_session_state(
	mut commands: Commands,
	mode: Res<PlaygroundMode>,
	gameplay: Res<WorldGameplayEnabled>,
	primary: Option<Res<PrimaryVirtualPad>>,
	inventory_edit: Option<Res<InventoryEditCameraFollow>>,
	map: Option<Res<WorldMapView>>,
	players: Query<
		(Entity, Has<CameraFollow>, Option<&ActorControlStatus>),
		With<VegetationPlayer>,
	>,
	mut locked: Option<ResMut<CameraPovLocked>>,
	mut suppressed: Option<ResMut<CameraLookSuppressed>>,
	mut cameras: Query<&mut CameraController, With<FollowCamera>>,
) {
	let edit = inventory_edit.is_some_and(|edit| edit.0);
	let map_open = map.is_some_and(|map| map.open);
	let presenting = map_open && gameplay.0;
	let follow = *mode == PlaygroundMode::Character && (gameplay.0 || edit);
	for (entity, following, control) in &players {
		let primary_actor = primary.as_deref().is_some_and(|primary| {
			control.and_then(|control| control.source()).and_then(|source| source.entity())
				== Some(primary.0)
		});
		if follow && primary_actor && !following {
			commands.entity(entity).insert(CameraFollow);
		} else if (!follow || !primary_actor) && following {
			commands.entity(entity).remove::<CameraFollow>();
		}
	}
	if let Some(locked) = locked.as_deref_mut() {
		locked.0 = presenting || edit || map_open;
	}
	if let Some(suppressed) = suppressed.as_deref_mut() {
		suppressed.0 = presenting;
	}
	if !edit || map_open {
		return;
	}
	for mut controller in &mut cameras {
		if controller.pov.is_map() {
			continue;
		}
		controller.pov = CameraPov::ThirdPerson;
		controller.focus = 0.0;
		controller.ads = 0.0;
	}
}

/// Preserve the playground's debug free-fly mode on the sole shared camera.
fn fly_world_camera(
	primary: Option<Res<PrimaryVirtualPad>>,
	players: Query<&RoutedActorControl, With<VegetationPlayer>>,
	mode: Res<PlaygroundMode>,
	mut cameras: Query<
		(Entity, &CameraController, &FollowCamera, &WorldFreeCamera, &mut Projection),
		With<Camera3d>,
	>,
	mut commands: Commands,
) {
	if *mode != PlaygroundMode::Free {
		return;
	}
	let Ok((camera, controller, follow, free, mut projection)) = cameras.single_mut() else {
		return;
	};
	if let Projection::Perspective(perspective) = projection.as_mut() {
		perspective.fov = follow.third_person_fov;
	}
	let Some(input) = primary.as_deref().and_then(|primary| {
		players.iter().find_map(|control| {
			(control.source().and_then(|source| source.entity()) == Some(primary.0))
				.then(|| control.get().copied())
				.flatten()
		})
	}) else {
		return;
	};
	let (yaw, pitch, speed) = (controller.yaw, controller.pitch, free.speed);
	input.map(|sample| (camera, sample)).queue(&mut commands, move |world, input| {
		if world
			.get_resource::<PlaygroundMode>()
			.is_none_or(|mode| *mode != PlaygroundMode::Free)
		{
			return;
		}
		let can_move = world.get_resource::<TextEntryFocus>().is_some_and(|focus| !focus.0)
			&& world.get_resource::<WorldGameplayEnabled>().is_some_and(|gameplay| gameplay.0);
		let (camera, sample) = input.into_payload();
		let Ok(mut camera) = world.get_entity_mut(camera) else {
			return;
		};
		let Some(mut transform) = camera.get_mut::<Transform>() else {
			return;
		};
		transform.rotation = Quat::from_rotation_y(yaw) * Quat::from_rotation_x(pitch);
		if !can_move {
			return;
		}
		let mut movement = *transform.forward() * sample.move_stick().y
			+ *transform.right() * sample.move_stick().x;
		if sample.a().pressed() {
			movement += Vec3::Y;
		}
		if sample.shift_held() {
			movement -= Vec3::Y;
		}
		if movement.length_squared() > 0.0 {
			transform.translation += movement.normalize() * speed * sample.delta().as_secs_f32();
		}
	});
}

/// Pull only the third-person translation toward the player when Fixed geometry blocks it.
pub(crate) fn obstruct_world_camera(
	mode: Res<PlaygroundMode>,
	spatial: SpatialQuery,
	players: PlayerFollowQuery,
	mut cameras: CameraFollowQuery,
) {
	if *mode != PlaygroundMode::Character {
		return;
	}
	let Ok((player_entity, player)) = players.single() else {
		return;
	};
	let Ok((mut camera, controller, follow)) = cameras.single_mut() else {
		return;
	};
	if controller.pov != CameraPov::ThirdPerson {
		return;
	}

	let yaw = Quat::from_rotation_y(controller.yaw);
	let target =
		player.translation + Vec3::Y * follow.look_height + yaw * Vec3::X * follow.shoulder_offset;
	camera.translation =
		obstructed_camera_translation(&spatial, target, camera.translation, player_entity);
}

fn obstructed_camera_translation(
	spatial: &SpatialQuery,
	origin: Vec3,
	desired: Vec3,
	exclude: Entity,
) -> Vec3 {
	let delta = desired - origin;
	let Ok(direction) = Dir3::new(delta) else {
		return desired;
	};
	let distance = delta.length();
	let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed)
		.with_excluded_entities([exclude]);
	let shape = Collider::sphere(CAMERA_COLLISION_RADIUS);
	let config = ShapeCastConfig::from_max_distance(distance);
	let hit_distance = spatial
		.cast_shape(&shape, origin, Quat::IDENTITY, direction, &config, &filter)
		.map(|hit| hit.distance);
	let travel =
		camera_cast_travel(distance, hit_distance, CAMERA_COLLISION_SKIN, CAMERA_COLLISION_MIN);
	origin + *direction * travel
}

fn camera_cast_travel(
	desired_distance: f32,
	hit_distance: Option<f32>,
	skin: f32,
	min_distance: f32,
) -> f32 {
	match hit_distance {
		Some(distance) => (distance - skin).clamp(min_distance, desired_distance),
		None => desired_distance,
	}
}

/// Keep impact presentation in the world layer, which owns both combat and camera domains.
fn present_fisticuffs_impact(
	mode: Res<PlaygroundMode>,
	mut strikes: MessageReader<FisticuffsStrike>,
	mut impacts: MessageReader<FisticuffsImpact>,
	mut impulses: MessageWriter<CameraPresentationImpulse>,
) {
	if *mode != PlaygroundMode::Character {
		strikes.clear();
		impacts.clear();
		return;
	}
	for strike in strikes.read() {
		impulses.write(CameraPresentationImpulse {
			target: strike.attacker,
			rotation: Vec2::new(strike.side.sign() * 0.4_f32.to_radians(), -0.25_f32.to_radians()),
		});
	}
	for impact in impacts.read() {
		let yaw = impact.side.sign() * 0.8_f32.to_radians();
		let pitch = -0.3_f32.to_radians();
		impulses.write(CameraPresentationImpulse {
			target: impact.attacker,
			rotation: Vec2::new(yaw, pitch),
		});
	}
}

pub(crate) fn configure(app: &mut App) {
	app.add_systems(Startup, spawn_world_camera)
		.add_systems(
			Update,
			present_fisticuffs_impact
				.after(PlayerSystems::Intent)
				.before(PlayerCameraSystems::Follow),
		)
		.add_systems(
			Update,
			sync_camera_session_state
				.after(WorldMapSet::Toggle)
				.before(PlayerCameraSystems::Look),
		)
		.add_systems(
			Update,
			obstruct_world_camera
				.after(PlayerCameraSystems::Follow)
				.before(PlayerCameraSystems::Apply),
		)
		.add_systems(Update, fly_world_camera.after(PlayerCameraSystems::Apply));
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn world_camera_keeps_world_view_range() {
		let follow = world_follow_camera();
		assert_eq!(follow.near, WORLD_CAMERA_NEAR);
		assert_eq!(follow.far, WORLD_CAMERA_FAR);
	}

	#[test]
	fn world_camera_setup_spawns_one_shared_gameplay_camera() {
		let mut app = App::new();
		app.init_resource::<world_player::PlayerSpawnXz>();
		app.add_systems(Startup, spawn_world_camera);
		app.update();
		let mut cameras = app
			.world_mut()
			.query_filtered::<(&FollowCamera, &CameraController), With<Camera3d>>();
		assert_eq!(cameras.iter(app.world()).count(), 1);
	}

	#[test]
	fn miss_keeps_desired_distance() {
		assert_eq!(camera_cast_travel(3.6, None, 0.08, 0.12), 3.6);
	}

	#[test]
	fn hit_pulls_in_by_skin() {
		assert!((camera_cast_travel(3.6, Some(1.0), 0.08, 0.12) - 0.92).abs() < 1e-5);
	}

	#[test]
	fn near_hit_does_not_cross_the_target() {
		assert_eq!(camera_cast_travel(3.6, Some(0.05), 0.08, 0.12), 0.12);
	}

	#[test]
	fn camera_follow_tracks_session_state_before_look() -> anyhow::Result<()> {
		use player::{CameraFollow, Player};
		use player_camera::{CameraLookSuppressed, CameraPovLocked};
		use world_player::Player as VegetationPlayer;

		use crate::InventoryEditCameraFollow;

		#[derive(Resource, Default)]
		struct FollowAtLook(Vec<Vec<Entity>>);

		fn record_follow_at_look(
			players: Query<(Entity, Has<CameraFollow>), With<VegetationPlayer>>,
			mut frames: ResMut<FollowAtLook>,
		) {
			frames.0.push(
				players
					.iter()
					.filter_map(|(entity, follows)| follows.then_some(entity))
					.collect(),
			);
		}

		let mut app = App::new();
		app.insert_resource(PlaygroundMode::Character)
			.insert_resource(WorldGameplayEnabled(true))
			.insert_resource(InventoryEditCameraFollow(false))
			.init_resource::<CameraPovLocked>()
			.init_resource::<CameraLookSuppressed>()
			.init_resource::<FollowAtLook>()
			.add_systems(Update, sync_camera_session_state.before(PlayerCameraSystems::Look))
			.add_systems(Update, record_follow_at_look.in_set(PlayerCameraSystems::Look));

		let source = app.world_mut().spawn_empty().id();
		app.world_mut().insert_resource(PrimaryVirtualPad(source));
		let status = ActorControlStatus::assigned(
			maybraid_controllers_core::InputSource::virtual_pad(source),
		);
		let player = app.world_mut().spawn((VegetationPlayer, Player, status)).id();
		app.update();
		assert!(app
			.world()
			.resource::<FollowAtLook>()
			.0
			.last()
			.is_some_and(|following| following.len() == 1 && following[0] == player));
		assert!(!app.world().resource::<CameraPovLocked>().0);

		app.world_mut().resource_mut::<WorldGameplayEnabled>().0 = false;
		app.world_mut().resource_mut::<InventoryEditCameraFollow>().0 = true;
		app.update();
		assert!(app
			.world()
			.resource::<FollowAtLook>()
			.0
			.last()
			.is_some_and(|following| following.len() == 1 && following[0] == player));
		assert!(app.world().resource::<CameraPovLocked>().0);

		app.world_mut().despawn(player);
		let replacement = app.world_mut().spawn((VegetationPlayer, Player, status)).id();
		app.update();
		assert!(app
			.world()
			.resource::<FollowAtLook>()
			.0
			.last()
			.is_some_and(|following| following.len() == 1 && following[0] == replacement));
		assert!(app.world().resource::<CameraPovLocked>().0);

		app.world_mut().resource_mut::<InventoryEditCameraFollow>().0 = false;
		app.update();
		assert!(app.world().resource::<FollowAtLook>().0.last().is_some_and(Vec::is_empty));
		assert!(!app.world().resource::<CameraPovLocked>().0);

		app.world_mut().resource_mut::<WorldGameplayEnabled>().0 = true;
		app.update();
		assert!(app
			.world()
			.resource::<FollowAtLook>()
			.0
			.last()
			.is_some_and(|following| following.len() == 1 && following[0] == replacement));
		assert!(!app.world().resource::<CameraPovLocked>().0);
		Ok(())
	}
}
