//! Capsule player + third-person character mode for the terrain playground.
//!
//! Movement logic follows Avian's `dynamic_character_3d` example (dynamic body,
//! shape-cast grounded check, jump impulse), with walk direction relative to
//! the camera yaw.

use avian3d::prelude::*;
use bevy::{ecs::query::Has, prelude::*};
use characters::LocomotionCapsule;
use durham::{BaseTerrainNoise, DurhamSurface, TerrainCellLayout};
use game_commands::command::TextEntryFocus;
use lod::hcsg::HcsgStorage;
use lod_avian::PhysicsInteractionLayer;
use maybraid_character_controller::{ActorControlStatus, RoutedActorControl};
use maybraid_spawns_support::{
	register_actor_support, terrain_height_at, ActorSupportSystems, PendingActorSupport,
	PlacementSearch,
};
use maybraid_virtual_pad_controller::PrimaryVirtualPad;

use crate::{camera::CameraController, WorldBaseTerrain};

pub(crate) const CAPSULE_RADIUS: f32 = 0.4;
pub(crate) const CAPSULE_LENGTH: f32 = 1.0;
const MOVE_SPEED: f32 = 7.0;
const MOVE_ACCEL: f32 = 40.0;
const MOVE_BRAKE: f32 = 50.0;
const AIR_CONTROL: f32 = 0.25;
const JUMP_IMPULSE: f32 = 8.0;
/// Reject near-vertical side contacts as support while retaining steep cliffs.
const MIN_SUPPORT_NORMAL_Y: f32 = 0.05;
/// Third-person orbit for a ~2 m humanoid (capsule center is hip height).
pub const CAMERA_DISTANCE: f32 = 3.6;
pub const CAMERA_HEIGHT: f32 = 1.1;
pub const CAMERA_LOOK_HEIGHT: f32 = 0.65;
const GROUND_CAST_DISTANCE: f32 = 0.45;
const GROUND_SNAP_SPEED: f32 = 1.5;
const PLAY_GRAVITY_SCALE: f32 = 1.25;
/// Bedrock is `-4 * height_scale`. Recover only after falling through that floor.
const VOID_FLOOR_MARGIN: f32 = 500.0;
const VOID_RESPAWN_DELAY_SECS: f32 = 0.75;

/// Camera-relative WASD wish on XZ. Zero when no move input.
#[derive(Component, Default)]
pub struct MoveWish(pub Vec3);

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayerControlSystems;

// Preserve the world-player API while sharing the same slope policy as all actors.
pub use characters::CharacterLocomotion;

/// Playground interaction mode.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaygroundMode {
	/// Free-look fly camera (default).
	#[default]
	Free,
	/// Capsule character with third-person camera.
	Character,
}

#[derive(Component)]
pub struct Player;

/// Request shared support validation before releasing this player's first drop.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AwaitingTerrainSurface;

/// The legacy player arrival marker has entered the shared actor-support lifecycle.
#[derive(Component)]
pub(crate) struct PlayerSupportQueued;

/// Stand on a local collider instead of streamed terrain.
/// Fall recovery returns to [`Self::translation`].
#[derive(Component, Clone, Copy, Debug)]
pub struct OffTerrainAnchor {
	pub translation: Vec3,
}

/// Delayed recovery after falling through bedrock into the void.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PlayerRespawn {
	queued_at: Option<f32>,
	resume_gravity: Option<f32>,
}

/// Optional Discovery / playground spawn on XZ (metres). `None` uses the
/// terrain layout center.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerSpawnXz(pub Option<Vec2>);

/// Whether the player rigid body may participate in physics.
///
/// Playgrounds default this on. The game shell keeps it off until Discovery
/// drop-in (gravity 0, no `RigidBodyDisabled` / `ColliderDisabled`).
#[derive(Resource, Debug, Clone, Copy)]
pub struct PlayerPhysicsEnabled(pub bool);

impl Default for PlayerPhysicsEnabled {
	fn default() -> Self {
		Self(true)
	}
}

/// Debug capsule mesh parented to [`Player`] (hidden when a character visual is set).
#[derive(Component)]
pub struct PlayerCapsule;

#[derive(Component)]
struct CharacterController;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub(crate) struct Grounded;

/// Last valid support plane.
///
/// A descending capsule can briefly outrun its down-cast between terrain
/// facets. Reuse this only while [`Grounded`] bridges that probe miss.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct WalkableGround {
	pub(crate) normal: Vec3,
}

impl Default for WalkableGround {
	fn default() -> Self {
		Self { normal: Vec3::Y }
	}
}

/// Space jump is in flight. Cleared on landing, not on shapecast misses.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Jumping {
	left_ground: bool,
}

#[derive(Component)]
struct MovementAcceleration(f32);

#[derive(Component)]
struct JumpImpulse(f32);

#[derive(Component)]
struct MaxSlopeAngle(f32);

/// When false, this crate skips grounded / wish / jump on the capsule so a
/// downstream motor (world player buoyancy) can own velocity. Snap and
/// fall-respawn still run.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct VegetationPlayerMotor(pub bool);

impl Default for VegetationPlayerMotor {
	fn default() -> Self {
		Self(true)
	}
}

/// When false, a downstream controller writes [`MoveWish`].
#[derive(Resource, Clone, Copy, Debug)]
pub struct PadMovementEnabled(pub bool);

impl Default for PadMovementEnabled {
	fn default() -> Self {
		Self(true)
	}
}

/// When false, a downstream system owns the character camera (world first/third POV).
#[derive(Resource, Clone, Copy, Debug)]
pub struct CharacterCameraFollowEnabled(pub bool);

impl Default for CharacterCameraFollowEnabled {
	fn default() -> Self {
		Self(true)
	}
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
	fn build(&self, app: &mut App) {
		register_actor_support(app);
		app.add_systems(Update, finish_player_surface_arrivals.after(ActorSupportSystems::Resolve));
		app.init_resource::<PlaygroundMode>()
			.init_resource::<PadMovementEnabled>()
			.init_resource::<VegetationPlayerMotor>()
			.init_resource::<CharacterCameraFollowEnabled>()
			.init_resource::<CharacterLocomotion>()
			.init_resource::<PlayerRespawn>()
			.init_resource::<PlayerPhysicsEnabled>()
			.init_resource::<PlayerSpawnXz>()
			.add_systems(Startup, spawn_player)
			.add_systems(
				Update,
				(
					keyboard_movement_input,
					bevy::ecs::schedule::ApplyDeferred,
					update_grounded,
					apply_character_movement,
					follow_character_camera,
					queue_void_player_respawn,
					recover_void_player,
				)
					.chain()
					.in_set(PlayerControlSystems),
			);
	}
}

fn spawn_player(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	locomotion: Res<CharacterLocomotion>,
	spawn_xz: Res<PlayerSpawnXz>,
) {
	// Startup: composed cells are not in the store yet. Hold above base noise
	// so the capsule / follow-cam are not born under jersey plateaus.
	let xz = spawn_xz.0.unwrap_or(layout.region_center_xz().xz());
	let spawn = player_spawn_point_at(xz, holding_elevation(&base.0, xz.x, xz.y));
	let player =
		spawn_player_body(&mut commands, &mut meshes, &mut materials, locomotion.as_ref(), spawn);
	commands.entity(player).insert((
		AwaitingTerrainSurface,
		PendingActorSupport::new(spawn, LocomotionCapsule::HUMANOID, None)
			.with_resume_gravity(Some(PLAY_GRAVITY_SCALE)),
	));
}

/// Spawn the terrain player's dynamic capsule at a known body-center position.
///
/// Call [`player_position_above_surface`] when the input is a terrain/POI surface point.
pub fn spawn_player_body(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	locomotion: &CharacterLocomotion,
	spawn: Vec3,
) -> Entity {
	let collider = Collider::capsule(CAPSULE_RADIUS, CAPSULE_LENGTH);
	let mut caster_shape = collider.clone();
	caster_shape.set_scale(Vec3::splat(0.99), 10);

	let player = commands
		.spawn((
			Name::new("Player"),
			Player,
			CharacterController,
			Transform::from_translation(spawn),
			Visibility::default(),
			RigidBody::Dynamic,
			collider,
			PhysicsInteractionLayer::animated_layers(),
			ShapeCaster::new(caster_shape, Vec3::ZERO, Quat::IDENTITY, Dir3::NEG_Y)
				.with_max_distance(GROUND_CAST_DISTANCE)
				.with_query_filter(SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed)),
			LockedAxes::ROTATION_LOCKED,
			ActiveCollisionHooks::MODIFY_CONTACTS,
		))
		.insert((
			MovementAcceleration(MOVE_ACCEL),
			JumpImpulse(JUMP_IMPULSE),
			MaxSlopeAngle(locomotion.max_slope_angle),
			MoveWish::default(),
			WalkableGround::default(),
			Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
			Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
			GravityScale(0.0),
		))
		.id();
	commands.spawn((
		Name::new("PlayerCapsule"),
		PlayerCapsule,
		ChildOf(player),
		Mesh3d(meshes.add(Capsule3d::new(CAPSULE_RADIUS, CAPSULE_LENGTH))),
		MeshMaterial3d(materials.add(Color::srgb(0.85, 0.55, 0.35))),
	));
	player
}

pub(crate) fn capsule_half_height() -> f32 {
	CAPSULE_RADIUS + CAPSULE_LENGTH * 0.5
}

/// Lift a terrain/POI surface point to a safe player capsule center.
pub fn player_position_above_surface(surface: Vec3) -> Vec3 {
	surface + Vec3::Y * (capsule_half_height() + 0.5)
}

// Keep the player-facing API while sharing initial hints with all actor domains.
pub use maybraid_spawns_support::{holding_elevation, surface_or_hold_at};

pub fn player_spawn_point_at(xz: Vec2, elevation: f32) -> Vec3 {
	player_position_above_surface(Vec3::new(xz.x, elevation, xz.y))
}

pub fn player_spawn_point(layout: &TerrainCellLayout, elevation: f32) -> Vec3 {
	player_spawn_point_at(layout.region_center_xz().xz(), elevation)
}

/// Adapt player first-drop requests into the shared actor support lifecycle.
/// The shared resolver owns geometry checks, retrying, placement, and physics holds.
pub(crate) fn snap_player_to_composed_surface(
	mut commands: Commands,
	physics: Res<PlayerPhysicsEnabled>,
	surface: DurhamSurface,
	storage: Option<Res<HcsgStorage>>,
	mut players: Query<
		(
			Entity,
			&Transform,
			Ref<AwaitingTerrainSurface>,
			Option<&OffTerrainAnchor>,
			Option<&LocomotionCapsule>,
			Option<&GravityScale>,
			Option<&mut PendingActorSupport>,
			Has<PlayerSupportQueued>,
		),
		With<Player>,
	>,
) {
	for (entity, transform, awaiting, anchor, hull, gravity, pending, queued) in &mut players {
		if anchor.is_some() && physics.0 {
			let resume_gravity = pending
				.as_deref()
				.and_then(PendingActorSupport::resume_gravity)
				.unwrap_or_else(|| gravity.map(|gravity| gravity.0));
			let mut player = commands.entity(entity);
			player.remove::<(AwaitingTerrainSurface, PlayerSupportQueued, PendingActorSupport)>();
			match resume_gravity {
				Some(gravity) => {
					player.insert(GravityScale(gravity));
				}
				None => {
					player.remove::<GravityScale>();
				}
			}
			continue;
		}
		if pending.is_none() && queued && !awaiting.is_changed() {
			continue;
		}
		let mut support = if let Some(pending) = pending.as_deref() {
			let mut support = pending.clone();
			if awaiting.is_changed() {
				let hull = hull.copied().unwrap_or(support.hull());
				support.retarget(transform.translation, hull, None);
			}
			support
		} else {
			PendingActorSupport::new(
				transform.translation,
				hull.copied().unwrap_or(LocomotionCapsule::HUMANOID),
				None,
			)
		};
		support.set_enabled(physics.0);
		let xz = support.held_position().xz();
		let search = terrain_height_at(&surface, storage.as_deref(), xz)
			.map(|height| PlacementSearch::outdoor_hint(xz, height, support.hull()));
		support.observe_search(search);
		commands.entity(entity).insert((support, PlayerSupportQueued));
	}
}

fn finish_player_surface_arrivals(
	mut commands: Commands,
	players: Query<Entity, (With<Player>, With<PlayerSupportQueued>, Without<PendingActorSupport>)>,
) {
	for entity in &players {
		commands
			.entity(entity)
			.remove::<(AwaitingTerrainSurface, PlayerSupportQueued)>();
	}
}

fn void_floor_y(base: &BaseTerrainNoise) -> f32 {
	-base.height_scale * 4.0 - VOID_FLOOR_MARGIN
}

fn queue_void_player_respawn(
	time: Res<Time>,
	physics: Res<PlayerPhysicsEnabled>,
	base: Res<WorldBaseTerrain>,
	mut respawn: ResMut<PlayerRespawn>,
	mut player: Query<
		(&Transform, &mut LinearVelocity, &mut GravityScale),
		(With<Player>, Without<AwaitingTerrainSurface>),
	>,
) {
	if !physics.0 || respawn.queued_at.is_some() {
		return;
	}
	let Ok((transform, mut velocity, mut gravity)) = player.single_mut() else {
		return;
	};
	if transform.translation.y >= void_floor_y(&base.0) {
		return;
	}
	respawn.queued_at = Some(time.elapsed_secs() + VOID_RESPAWN_DELAY_SECS);
	respawn.resume_gravity = Some(gravity.0);
	gravity.0 = 0.0;
	**velocity = Vec3::ZERO;
}

fn recover_void_player(
	time: Res<Time>,
	surface: DurhamSurface,
	mut respawn: ResMut<PlayerRespawn>,
	mut commands: Commands,
	mut player: Query<
		(
			Entity,
			&mut Transform,
			&mut LinearVelocity,
			Option<&OffTerrainAnchor>,
			Option<&mut PendingActorSupport>,
		),
		With<Player>,
	>,
) {
	let Some(at) = respawn.queued_at else {
		return;
	};
	if time.elapsed_secs() < at {
		return;
	}
	let Ok((entity, mut transform, mut velocity, anchor, pending)) = player.single_mut() else {
		respawn.queued_at = None;
		return;
	};
	if let Some(anchor) = anchor {
		transform.translation = anchor.translation;
		**velocity = Vec3::ZERO;
		respawn.queued_at = None;
		commands
			.entity(entity)
			.insert(GravityScale(respawn.resume_gravity.take().unwrap_or(PLAY_GRAVITY_SCALE)));
		return;
	}
	let xz = transform.translation.xz();
	let elevation = match surface.height_at(xz) {
		Ok(Some(elevation)) => elevation,
		Ok(None) | Err(lod::hcsg::Busy) => holding_elevation(surface.base(), xz.x, xz.y),
	};
	transform.translation = player_spawn_point_at(xz, elevation);
	**velocity = Vec3::ZERO;
	respawn.queued_at = None;
	// Requeue the same actor without losing a pending destination to the fixed hold.
	let mut support = pending.as_deref().cloned().unwrap_or_else(|| {
		PendingActorSupport::new(transform.translation, LocomotionCapsule::HUMANOID, None)
			.with_resume_gravity(Some(respawn.resume_gravity.unwrap_or(PLAY_GRAVITY_SCALE)))
	});
	let hull = support.hull();
	support.retarget(transform.translation, hull, None);
	respawn.resume_gravity = None;
	commands.entity(entity).insert((AwaitingTerrainSurface, support));
}

/// Reposition the player after terrain layout regeneration.
pub fn respawn_player_on_layout(
	layout: &TerrainCellLayout,
	elevation: f32,
	transform: &mut Transform,
	velocity: &mut LinearVelocity,
) {
	transform.translation = player_spawn_point(layout, elevation);
	**velocity = Vec3::ZERO;
}

fn keyboard_movement_input(
	mode: Res<PlaygroundMode>,
	text_focus: Res<TextEntryFocus>,
	pad_movement: Res<PadMovementEnabled>,
	primary: Option<Res<PrimaryVirtualPad>>,
	cameras: Query<&CameraController, With<Camera3d>>,
	mut wishes: Query<
		(Entity, &mut MoveWish, Option<&RoutedActorControl>, Option<&ActorControlStatus>),
		With<Player>,
	>,
	mut commands: Commands,
) {
	if !pad_movement.0 {
		return;
	}
	if *mode != PlaygroundMode::Character || text_focus.0 {
		for (_, mut wish, _, _) in &mut wishes {
			wish.0 = Vec3::ZERO;
		}
		return;
	}
	let Some(primary) = primary.as_deref() else {
		return;
	};
	for (entity, mut wish, control, status) in &mut wishes {
		if status.and_then(|status| status.source()).and_then(|source| source.entity())
			!= Some(primary.0)
		{
			continue;
		}
		// Neutral is immediate; positive motion must keep the source's delivery grant.
		wish.0 = Vec3::ZERO;
		let Some(input) = control.and_then(RoutedActorControl::get).copied() else {
			continue;
		};
		let direction = input.payload().move_stick().clamp_length_max(1.0);
		let wish_dir = if direction != Vec2::ZERO {
			if let Ok(camera) = cameras.single() {
				let yaw = Quat::from_axis_angle(Vec3::Y, camera.yaw);
				let forward = yaw * -Vec3::Z;
				let right_dir = yaw * Vec3::X;
				(right_dir * direction.x + forward * direction.y).normalize_or_zero()
			} else {
				Vec3::ZERO
			}
		} else {
			Vec3::ZERO
		};
		if wish_dir != Vec3::ZERO {
			input.map(|_| (entity, wish_dir)).queue(&mut commands, |world, input| {
				let (entity, direction) = input.into_payload();
				if let Some(mut wish) = world.get_mut::<MoveWish>(entity) {
					wish.0 = direction;
				}
			});
		}
	}
}

fn update_grounded(
	mode: Res<PlaygroundMode>,
	motor: Res<VegetationPlayerMotor>,
	mut commands: Commands,
	mut query: Query<
		(
			Entity,
			&ShapeHits,
			&LinearVelocity,
			Has<Grounded>,
			Option<&mut Jumping>,
			&mut WalkableGround,
		),
		With<CharacterController>,
	>,
) {
	if !motor.0 || *mode != PlaygroundMode::Character {
		return;
	}

	for (entity, hits, velocity, was_grounded, jumping, mut walkable) in &mut query {
		let support = support_ground_normal(hits);
		if let Some(normal) = support {
			walkable.normal = normal;
		}
		let mut is_grounded = support.is_some();
		if !is_grounded
			&& was_grounded
			&& jumping.is_none()
			&& velocity.y > -GROUND_SNAP_SPEED
			&& velocity.y < GROUND_SNAP_SPEED
		{
			is_grounded = true;
		}
		let landed = jumping.as_ref().is_some_and(|jump| jump.left_ground);
		if is_grounded {
			commands.entity(entity).insert(Grounded);
			if landed {
				commands.entity(entity).remove::<Jumping>();
			}
		} else {
			commands.entity(entity).remove::<Grounded>();
			if let Some(mut jump) = jumping {
				jump.left_ground = true;
			}
		}
	}
}

/// Most upright support under the capsule, including slopes too steep to climb.
fn support_ground_normal(hits: &ShapeHits) -> Option<Vec3> {
	let mut best: Option<(f32, Vec3)> = None;
	for hit in hits.iter() {
		let normal = hit.normal1.normalize_or_zero();
		if normal.length_squared() < 1e-8 || normal.y <= MIN_SUPPORT_NORMAL_Y {
			continue;
		}
		let angle = normal.angle_between(Vec3::Y).abs();
		if best.is_none_or(|(best_angle, _)| angle < best_angle) {
			best = Some((angle, normal));
		}
	}
	best.map(|(_, normal)| normal)
}

/// Unit surface drive. Only the uphill component is removed above the climb cap.
fn ground_drive(wish: Vec3, normal: Vec3, max_slope: f32) -> Vec3 {
	let wish = Vec3::new(wish.x, 0.0, wish.z).normalize_or_zero();
	let normal = normal.normalize_or_zero();
	if wish.length_squared() < 1e-8 || normal.length_squared() < 1e-8 {
		return Vec3::ZERO;
	}
	let mut drive = wish - normal * wish.dot(normal);
	if normal.angle_between(Vec3::Y).abs() > max_slope {
		let uphill = (Vec3::Y - normal * normal.y).normalize_or_zero();
		let uphill_amount = drive.dot(uphill);
		if uphill_amount > 0.0 {
			drive -= uphill * uphill_amount;
		}
	}
	if drive.length_squared() < 1e-8 {
		Vec3::ZERO
	} else {
		drive.normalize()
	}
}

fn move_toward(current: Vec3, target: Vec3, max_delta: f32) -> Vec3 {
	let delta = target - current;
	if delta.length_squared() <= max_delta * max_delta {
		target
	} else {
		current + delta.normalize_or_zero() * max_delta
	}
}

fn control_ground_velocity(
	velocity: &mut LinearVelocity,
	wish: Vec3,
	normal: Vec3,
	max_slope: f32,
	accel: f32,
	dt: f32,
) {
	let normal = normal.normalize_or_zero();
	let tangent = **velocity - normal * velocity.dot(normal);
	let target = ground_drive(wish, normal, max_slope) * MOVE_SPEED;
	let rate = if target.length_squared() > 1e-8 { accel } else { MOVE_BRAKE };
	**velocity = move_toward(tangent, target, rate * dt);
}

fn control_air_velocity(velocity: &mut LinearVelocity, wish: Vec3, accel: f32, dt: f32) {
	let wish = Vec3::new(wish.x, 0.0, wish.z).normalize_or_zero();
	if wish.length_squared() < 1e-8 {
		return;
	}
	let horizontal = Vec3::new(velocity.x, 0.0, velocity.z);
	let next = move_toward(horizontal, wish * MOVE_SPEED, accel * AIR_CONTROL * dt);
	velocity.x = next.x;
	velocity.z = next.z;
}

fn apply_character_movement(
	mut commands: Commands,
	mode: Res<PlaygroundMode>,
	motor: Res<VegetationPlayerMotor>,
	time: Res<Time>,
	mut controllers: Query<
		(
			Entity,
			&MoveWish,
			&ShapeHits,
			&MaxSlopeAngle,
			&WalkableGround,
			&MovementAcceleration,
			&JumpImpulse,
			&mut LinearVelocity,
			Has<Grounded>,
			Option<&Jumping>,
			Has<Player>,
			Option<&RoutedActorControl>,
		),
		With<CharacterController>,
	>,
) {
	if !motor.0 || *mode != PlaygroundMode::Character {
		return;
	}

	let dt = time.delta_secs();

	for (
		entity,
		wish,
		hits,
		max_slope,
		walkable,
		accel,
		jump,
		mut velocity,
		grounded,
		jumping,
		is_player,
		control,
	) in &mut controllers
	{
		let jump_input = control
			.and_then(RoutedActorControl::get)
			.copied()
			.filter(|input| input.payload().a().just_pressed());
		if let Some(normal) = grounded_plane(hits, walkable, grounded) {
			if jumping.is_none() {
				control_ground_velocity(&mut velocity, wish.0, normal, max_slope.0, accel.0, dt);
			} else {
				control_air_velocity(&mut velocity, wish.0, accel.0, dt);
			}
		} else {
			control_air_velocity(&mut velocity, wish.0, accel.0, dt);
		}
		if grounded && is_player {
			if let Some(input) = jump_input {
				let impulse = jump.0;
				input.map(|_| (entity, impulse)).queue(&mut commands, |world, input| {
					let (entity, impulse) = input.into_payload();
					if world.get::<Grounded>(entity).is_none() {
						return;
					}
					let jumped = if let Some(mut velocity) = world.get_mut::<LinearVelocity>(entity)
					{
						velocity.y = impulse;
						true
					} else {
						false
					};
					if jumped {
						world.entity_mut(entity).insert(Jumping { left_ground: false });
					}
				});
			}
		}
	}
}

fn grounded_plane(hits: &ShapeHits, walkable: &WalkableGround, grounded: bool) -> Option<Vec3> {
	grounded.then(|| support_ground_normal(hits).unwrap_or(walkable.normal))
}

fn follow_character_camera(
	mode: Res<PlaygroundMode>,
	follow: Res<CharacterCameraFollowEnabled>,
	players: Query<&Transform, (With<Player>, Without<Camera3d>)>,
	mut cameras: Query<(&mut Transform, &CameraController), With<Camera3d>>,
) {
	if !follow.0 {
		return;
	}
	if *mode != PlaygroundMode::Character {
		return;
	}
	let Ok(player) = players.single() else {
		return;
	};
	let Ok((mut camera_transform, controller)) = cameras.single_mut() else {
		return;
	};

	let yaw = Quat::from_axis_angle(Vec3::Y, controller.yaw);
	let pitch = Quat::from_axis_angle(Vec3::X, controller.pitch);
	let rotation = yaw * pitch;
	let offset = rotation * Vec3::new(0.0, 0.0, CAMERA_DISTANCE) + Vec3::Y * CAMERA_HEIGHT;
	let target = player.translation + Vec3::Y * CAMERA_LOOK_HEIGHT;
	camera_transform.translation = target + offset;
	camera_transform.look_at(target, Vec3::Y);
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use durham::{terrain_collider_covers_xz, CascadeChunk, TerrainConfig};
	use lod::hcsg::HcsgStorage;

	use super::*;

	#[test]
	fn holding_elevation_sits_above_base_noise() {
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
		let hold = holding_elevation(&base, 0.0, 0.0);
		assert!(hold > base.height_at(0.0, 0.0) + 50.0);
	}

	#[test]
	fn surface_or_hold_at_holds_above_unpublished_cells() -> anyhow::Result<()> {
		let mut world = World::new();
		let layout = TerrainCellLayout::default();
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
		world.init_resource::<HcsgStorage>();
		world.insert_resource(layout);
		world.insert_resource(WorldBaseTerrain(base.clone()));

		let mut state = bevy::ecs::system::SystemState::<DurhamSurface>::new(&mut world);
		let surface = state.get(&world)?;
		let xz = Vec2::new(80.0, 5.0);
		let hold = surface_or_hold_at(&surface, xz);
		assert_eq!(hold, holding_elevation(&base, xz.x, xz.y));
		assert!(hold > surface.fallback_height_at(xz) + 50.0);
		Ok(())
	}

	#[test]
	fn void_floor_sits_below_bedrock() {
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
		assert_eq!(void_floor_y(&base), -base.height_scale * 4.0 - VOID_FLOOR_MARGIN);
		assert!(void_floor_y(&base) < -2_000.0);
	}

	#[test]
	fn canyon_depth_does_not_queue_void_recovery() -> anyhow::Result<()> {
		let (mut world, player) = void_world(Vec3::new(0.0, 0.0, 0.0));
		world
			.run_system_once(queue_void_player_respawn)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.resource::<PlayerRespawn>().queued_at.is_none());
		assert!(world.get::<AwaitingTerrainSurface>(player).is_none());
		assert_eq!(
			world.get::<GravityScale>(player).map(|gravity| gravity.0),
			Some(PLAY_GRAVITY_SCALE)
		);
		Ok(())
	}

	#[test]
	fn below_void_floor_queues_recovery_without_reawait() -> anyhow::Result<()> {
		let (mut world, player) = void_world(Vec3::new(0.0, -2_600.0, 0.0));
		world
			.run_system_once(queue_void_player_respawn)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.resource::<PlayerRespawn>().queued_at.is_some());
		assert!(world.get::<AwaitingTerrainSurface>(player).is_none());
		assert_eq!(world.get::<GravityScale>(player).map(|gravity| gravity.0), Some(0.0));
		assert_eq!(
			world.get::<LinearVelocity>(player).map(|velocity| velocity.0),
			Some(Vec3::ZERO)
		);
		Ok(())
	}

	#[test]
	fn snap_ignores_a_placed_body_without_awaiting() -> anyhow::Result<()> {
		let pose = Vec3::new(4.0, 12.0, -3.0);
		let (mut world, player) = void_world(pose);
		world
			.run_system_once(snap_player_to_composed_surface)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(
			world.get::<Transform>(player).map(|transform| transform.translation),
			Some(pose)
		);
		assert!(world.get::<AwaitingTerrainSurface>(player).is_none());
		assert_eq!(
			world.get::<GravityScale>(player).map(|gravity| gravity.0),
			Some(PLAY_GRAVITY_SCALE),
			"a placed body is not held by snap"
		);
		Ok(())
	}

	#[test]
	fn void_recovery_snaps_and_holds_for_first_drop() -> anyhow::Result<()> {
		let (mut world, player) = void_world(Vec3::new(12.0, -2_600.0, -8.0));
		world.resource_mut::<PlayerRespawn>().queued_at = Some(0.0);
		world
			.run_system_once(recover_void_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let base = &world.resource::<WorldBaseTerrain>().0;
		let expected =
			player_spawn_point_at(Vec2::new(12.0, -8.0), holding_elevation(base, 12.0, -8.0));
		assert_eq!(
			world.get::<Transform>(player).map(|transform| transform.translation),
			Some(expected)
		);
		assert!(world.get::<AwaitingTerrainSurface>(player).is_some());
		assert!(world.resource::<PlayerRespawn>().queued_at.is_none());
		Ok(())
	}

	fn shared_support_app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_actor_support(&mut app);
		app.init_resource::<PlayerPhysicsEnabled>();
		app.init_resource::<TerrainCellLayout>();
		app.init_resource::<HcsgStorage>();
		app.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(
			42,
		))));
		app.add_systems(
			Update,
			snap_player_to_composed_surface.in_set(ActorSupportSystems::Prepare),
		);
		app.add_systems(Update, finish_player_surface_arrivals.after(ActorSupportSystems::Resolve));
		app
	}

	fn waiting_player(app: &mut App, position: Vec3) -> Entity {
		app.world_mut()
			.spawn((
				Player,
				AwaitingTerrainSurface,
				Transform::from_translation(position),
				RigidBody::Dynamic,
				Collider::capsule(CAPSULE_RADIUS, CAPSULE_LENGTH),
				GravityScale(PLAY_GRAVITY_SCALE),
				LinearVelocity(Vec3::new(4.0, -7.0, 2.0)),
			))
			.id()
	}

	#[test]
	fn each_awaiting_player_uses_the_shared_retry_lifecycle() -> anyhow::Result<()> {
		let mut app = shared_support_app();
		let a = waiting_player(&mut app, Vec3::new(8.0, 700.0, 8.0));
		let b = waiting_player(&mut app, Vec3::new(48.0, 800.0, 48.0));
		app.update();
		for entity in [a, b] {
			let pending = app
				.world()
				.get::<PendingActorSupport>(entity)
				.ok_or_else(|| anyhow::anyhow!("player missing shared pending support"))?;
			assert!(pending.search().is_none(), "unpublished height must remain pending");
			assert!(app.world().get::<AwaitingTerrainSurface>(entity).is_some());
			assert_eq!(app.world().get::<GravityScale>(entity).map(|g| g.0), Some(0.0));
			assert_eq!(app.world().get::<LinearVelocity>(entity).map(|v| v.0), Some(Vec3::ZERO));
		}
		Ok(())
	}

	#[test]
	fn teleport_replaces_an_existing_pending_destination() -> anyhow::Result<()> {
		let mut app = shared_support_app();
		app.add_plugins((
			TransformPlugin,
			PhysicsPlugins::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::mesh::MeshPlugin,
		));
		app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
			std::time::Duration::from_secs_f64(1.0 / 30.0),
		));
		app.finish();
		let a = Vec3::new(8.0, 700.0, 8.0);
		let b = Vec3::new(4_008.0, 900.0, 4_008.0);
		let player = waiting_player(&mut app, a);
		app.update();
		if let Some(mut pending) = app.world_mut().get_mut::<PendingActorSupport>(player) {
			let hull = pending.hull();
			pending.retarget(b, hull, None);
		}
		app.world_mut()
			.entity_mut(player)
			.insert((Transform::from_translation(b), AwaitingTerrainSurface));
		for _ in 0..4 {
			app.update();
		}
		let pending = app
			.world()
			.get::<PendingActorSupport>(player)
			.ok_or_else(|| anyhow::anyhow!("teleported player missing pending support"))?;
		assert_eq!(pending.held_position(), b);
		assert_eq!(app.world().get::<Transform>(player).map(|t| t.translation), Some(b));
		assert!(pending.search().is_none());
		Ok(())
	}

	#[test]
	fn live_arrival_preserves_custom_gravity_through_hold_and_anchor_release() {
		for gravity in [0.0, 0.65] {
			let mut app = shared_support_app();
			let pose = Vec3::new(8.0, 700.0, 8.0);
			let player = waiting_player(&mut app, pose);
			app.world_mut().entity_mut(player).insert(GravityScale(gravity));
			app.update();
			assert_eq!(app.world().get::<GravityScale>(player).map(|g| g.0), Some(0.0));
			app.world_mut()
				.entity_mut(player)
				.insert(OffTerrainAnchor { translation: pose });
			app.update();
			assert!(app.world().get::<PendingActorSupport>(player).is_none());
			assert!(app.world().get::<AwaitingTerrainSurface>(player).is_none());
			assert_eq!(app.world().get::<GravityScale>(player).map(|g| g.0), Some(gravity));
		}
	}

	#[test]
	fn default_locomotion_keeps_legacy_slope() {
		assert!(
			(CharacterLocomotion::default().max_slope_angle - std::f32::consts::PI * 0.45).abs()
				< 1e-6
		);
	}

	#[test]
	fn playground_motor_stays_on_by_default() {
		assert!(VegetationPlayerMotor::default().0);
	}

	#[test]
	fn drop_in_ignores_distant_trimesh_columns() {
		let spawn = Vec3::new(0.5, 80.0, 0.5);
		let local = CascadeChunk::unit_chunk();
		let distant = CascadeChunk {
			origin: Vec3::new(1_000.0, -2_000.0, 1_000.0),
			size: 160.0,
			..CascadeChunk::unit_chunk()
		};
		assert!(!terrain_collider_covers_xz(spawn, [&distant]));
		assert!(terrain_collider_covers_xz(spawn, [&local]));
	}

	#[test]
	fn regular_slopes_allow_equal_uphill_and_downhill_drive() {
		let slope = 45.0_f32.to_radians();
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		let uphill = ground_drive(Vec3::X, normal, 70.0_f32.to_radians());
		let downhill = ground_drive(Vec3::NEG_X, normal, 70.0_f32.to_radians());
		assert!((uphill.length() - 1.0).abs() < 1e-5);
		assert!((downhill.length() - 1.0).abs() < 1e-5);
		assert!((uphill + downhill).length() < 1e-5);
		assert!(uphill.y > 0.0);
		assert!(downhill.y < 0.0);
	}

	#[test]
	fn cliff_gate_removes_only_uphill_drive() {
		let slope = 80.0_f32.to_radians();
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		let uphill = ground_drive(Vec3::X, normal, 70.0_f32.to_radians());
		let downhill = ground_drive(Vec3::NEG_X, normal, 70.0_f32.to_radians());
		let contour = ground_drive(Vec3::Z, normal, 70.0_f32.to_radians());
		assert!(uphill.length_squared() < 1e-8);
		assert!((downhill.length() - 1.0).abs() < 1e-5);
		assert_eq!(contour, Vec3::Z);
	}

	#[test]
	fn grounded_probe_miss_reuses_last_support_plane() {
		let hits = ShapeHits::default();
		let normal = Vec3::new(-0.4, 0.9, 0.0).normalize();
		let walkable = WalkableGround { normal };
		assert_eq!(grounded_plane(&hits, &walkable, true), Some(normal));
		assert_eq!(grounded_plane(&hits, &walkable, false), None);
	}

	fn void_world(translation: Vec3) -> (World, Entity) {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.insert_resource(PlayerPhysicsEnabled::default());
		world.insert_resource(PlayerRespawn::default());
		world.insert_resource(TerrainCellLayout::default());
		world.init_resource::<HcsgStorage>();
		world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(
			&TerrainConfig::new(42),
		)));
		let player = world
			.spawn((
				Player,
				Transform::from_translation(translation),
				LinearVelocity(Vec3::Y * -20.0),
				GravityScale(PLAY_GRAVITY_SCALE),
			))
			.id();
		(world, player)
	}
}
