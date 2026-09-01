//! Capsule player + third-person follow. Intents come from the character controller.

use avian3d::prelude::*;
use bevy::ecs::query::Has;
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;
use std::f32::consts::PI;

use crate::camera::CameraController;

pub(crate) const CAPSULE_RADIUS: f32 = 0.4;
pub(crate) const CAPSULE_LENGTH: f32 = 1.0;
const MOVE_ACCEL: f32 = 40.0;
const MOVE_DAMPING: f32 = 0.92;
const JUMP_IMPULSE: f32 = 8.0;
const MAX_SLOPE_ANGLE: f32 = PI * 0.45;
pub(crate) const CAMERA_DISTANCE: f32 = 3.6;
pub(crate) const CAMERA_HEIGHT: f32 = 1.1;
pub(crate) const CAMERA_LOOK_HEIGHT: f32 = 0.65;
/// Shift the look target right so the character composes left of the reticle.
const CAMERA_SHOULDER_OFFSET: f32 = 0.7;
const GROUND_CAST_DISTANCE: f32 = 0.45;
const GROUND_SNAP_SPEED: f32 = 1.5;

#[derive(Component, Default)]
pub(crate) struct MoveWish(pub Vec3);

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PlayerControlSystems;

#[derive(Component)]
pub struct Player;

#[derive(Component)]
pub(crate) struct CameraFollow;

#[derive(Component)]
pub struct PlayerCapsule;

#[derive(Component)]
pub(crate) struct CharacterController;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub(crate) struct Grounded;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub(crate) struct Jumping {
	left_ground: bool,
}

#[derive(Component)]
struct MovementAcceleration(f32);

#[derive(Component)]
struct MovementDampingFactor(f32);

#[derive(Component)]
struct JumpImpulse(f32);

#[derive(Component)]
struct MaxSlopeAngle(f32);

#[derive(Message)]
pub(crate) enum MovementAction {
	Move(Vec2),
	Jump,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<MovementAction>().add_systems(
			Update,
			(
				update_grounded,
				apply_character_movement,
				apply_movement_damping,
				follow_character_camera,
			)
				.chain()
				.in_set(PlayerControlSystems),
		);
	}
}

pub(crate) fn spawn_player(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let spawn = Vec3::new(0.0, CAPSULE_RADIUS + CAPSULE_LENGTH * 0.5 + 0.15, 0.0);
	let player = spawn_character_controller(&mut commands, spawn);
	commands.entity(player).insert((Name::new("Player"), Player, CameraFollow));
	commands.spawn((
		Name::new("PlayerCapsule"),
		PlayerCapsule,
		ChildOf(player),
		Visibility::Hidden,
		Mesh3d(meshes.add(Capsule3d::new(CAPSULE_RADIUS, CAPSULE_LENGTH))),
		MeshMaterial3d(materials.add(Color::srgb(0.85, 0.55, 0.35))),
	));
}

fn spawn_character_controller(commands: &mut Commands, translation: Vec3) -> Entity {
	let collider = Collider::capsule(CAPSULE_RADIUS, CAPSULE_LENGTH);
	let mut caster_shape = collider.clone();
	caster_shape.set_scale(Vec3::splat(0.99), 10);
	commands
		.spawn((
			CharacterController,
			Transform::from_translation(translation),
			Visibility::default(),
			RigidBody::Dynamic,
			collider,
			PhysicsInteractionLayer::animated_layers(),
			ShapeCaster::new(caster_shape, Vec3::ZERO, Quat::IDENTITY, Dir3::NEG_Y)
				.with_max_distance(GROUND_CAST_DISTANCE)
				.with_query_filter(SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed)),
			LockedAxes::ROTATION_LOCKED,
		))
		.insert((
			MovementAcceleration(MOVE_ACCEL),
			MovementDampingFactor(MOVE_DAMPING),
			JumpImpulse(JUMP_IMPULSE),
			MaxSlopeAngle(MAX_SLOPE_ANGLE),
			MoveWish::default(),
			Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
			Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
			GravityScale(1.25),
		))
		.id()
}

fn update_grounded(
	mut commands: Commands,
	mut query: Query<
		(
			Entity,
			&ShapeHits,
			&LinearVelocity,
			Option<&MaxSlopeAngle>,
			Has<Grounded>,
			Option<&mut Jumping>,
		),
		With<CharacterController>,
	>,
) {
	for (entity, hits, velocity, max_slope_angle, was_grounded, jumping) in &mut query {
		let mut is_grounded = hits.iter().any(|hit| {
			if let Some(angle) = max_slope_angle {
				(-hit.normal2).angle_between(Vec3::Y).abs() <= angle.0
			} else {
				true
			}
		});
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

fn apply_character_movement(
	mut commands: Commands,
	time: Res<Time>,
	cameras: Query<&CameraController, With<Camera3d>>,
	mut reader: MessageReader<MovementAction>,
	mut controllers: Query<
		(Entity, &MovementAcceleration, &JumpImpulse, &mut LinearVelocity, Has<Grounded>),
		With<CharacterController>,
	>,
) {
	let Ok(camera) = cameras.single() else {
		for _ in reader.read() {}
		return;
	};

	let yaw = Quat::from_axis_angle(Vec3::Y, camera.yaw);
	let forward = yaw * -Vec3::Z;
	let right = yaw * Vec3::X;
	let dt = time.delta_secs();

	for action in reader.read() {
		for (entity, accel, jump, mut velocity, grounded) in &mut controllers {
			match action {
				MovementAction::Move(direction) => {
					let wish = (right * direction.x + forward * direction.y).normalize_or_zero();
					velocity.x += wish.x * accel.0 * dt;
					velocity.z += wish.z * accel.0 * dt;
				}
				MovementAction::Jump => {
					if grounded {
						velocity.y = jump.0;
						commands.entity(entity).insert(Jumping { left_ground: false });
					}
				}
			}
		}
	}
}

fn apply_movement_damping(
	mut query: Query<(&MovementDampingFactor, &mut LinearVelocity), With<CharacterController>>,
) {
	for (damping, mut velocity) in &mut query {
		velocity.x *= damping.0;
		velocity.z *= damping.0;
	}
}

fn follow_character_camera(
	players: Query<&Transform, (With<CameraFollow>, Without<Camera3d>)>,
	mut cameras: Query<(&mut Transform, &CameraController), With<Camera3d>>,
) {
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
	let target =
		player.translation + Vec3::Y * CAMERA_LOOK_HEIGHT + yaw * Vec3::X * CAMERA_SHOULDER_OFFSET;
	camera_transform.translation = target + offset;
	camera_transform.look_at(target, Vec3::Y);
}
