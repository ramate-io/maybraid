//! Dynamic capsule: grounded, wish accel, jump.

use avian3d::prelude::*;
use bevy::ecs::query::Has;
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;
use std::f32::consts::PI;

use crate::identity::{Player, PlayerLook};

pub(crate) const MOVE_ACCEL: f32 = 40.0;
pub(crate) const MOVE_DAMPING: f32 = 0.92;
pub(crate) const JUMP_IMPULSE: f32 = 8.0;
pub(crate) const MAX_SLOPE_ANGLE: f32 = PI * 0.45;
pub(crate) const GROUND_CAST_DISTANCE: f32 = 0.45;
const GROUND_SNAP_SPEED: f32 = 1.5;

#[derive(Component, Default)]
pub struct MoveWish(pub Vec3);

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayerControlSystems;

#[derive(Component)]
pub struct CharacterController;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Jumping {
	pub left_ground: bool,
}

#[derive(Component)]
pub(crate) struct MovementAcceleration(pub f32);

#[derive(Component)]
pub(crate) struct MovementDampingFactor(pub f32);

#[derive(Component)]
pub(crate) struct JumpImpulse(pub f32);

#[derive(Component)]
pub(crate) struct MaxSlopeAngle(pub f32);

#[derive(Message)]
pub enum MovementAction {
	Move(Vec2),
	Jump,
}

pub(crate) fn spawn_character_controller(commands: &mut Commands, translation: Vec3) -> Entity {
	let collider = Collider::capsule(crate::CAPSULE_RADIUS, crate::CAPSULE_LENGTH);
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

pub(crate) fn update_grounded(
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

pub(crate) fn apply_character_movement(
	mut commands: Commands,
	time: Res<Time>,
	mut reader: MessageReader<MovementAction>,
	mut controllers: Query<
		(
			Entity,
			&PlayerLook,
			&MovementAcceleration,
			&JumpImpulse,
			&mut LinearVelocity,
			Has<Grounded>,
		),
		(With<CharacterController>, With<Player>),
	>,
) {
	let dt = time.delta_secs();
	for action in reader.read() {
		for (entity, look, accel, jump, mut velocity, grounded) in &mut controllers {
			let yaw = Quat::from_axis_angle(Vec3::Y, look.yaw);
			let forward = yaw * -Vec3::Z;
			let right = yaw * Vec3::X;
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

pub(crate) fn apply_movement_damping(
	mut query: Query<(&MovementDampingFactor, &mut LinearVelocity), With<CharacterController>>,
) {
	for (damping, mut velocity) in &mut query {
		velocity.x *= damping.0;
		velocity.z *= damping.0;
	}
}
