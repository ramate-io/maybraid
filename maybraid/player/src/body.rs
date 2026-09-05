//! Dynamic capsule: grounded, wish accel, jump.

use avian3d::prelude::*;
use bevy::ecs::query::Has;
use bevy::prelude::*;
use crozon_characters::LocomotionCapsule;
use lod_avian::PhysicsInteractionLayer;
use std::f32::consts::PI;

use crate::identity::{Player, PlayerLook};

pub(crate) const MOVE_ACCEL: f32 = 40.0;
pub(crate) const MOVE_DAMPING: f32 = 0.92;
pub(crate) const JUMP_IMPULSE: f32 = 8.0;
pub(crate) const MAX_SLOPE_ANGLE: f32 = PI * 0.45;
pub(crate) const GROUND_CAST_DISTANCE: f32 = 0.45;
const GROUND_SNAP_SPEED: f32 = 1.5;

/// Walkable grounded slope for FFA / NPC capsules. Insert before [`crate::PlayerPlugin`]
/// to override the default (~81°). World / Durham playgrounds use ~70° so cliffs
/// never count as floor; static terrain friction should exceed `tan(this)`.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct CharacterLocomotion {
	/// Hits steeper than this (radians from up) are not grounded.
	pub max_slope_angle: f32,
}

impl Default for CharacterLocomotion {
	fn default() -> Self {
		Self { max_slope_angle: MAX_SLOPE_ANGLE }
	}
}

#[derive(Component, Default)]
pub struct MoveWish(pub Vec3);

/// One-shot jump request for non-player capsules. Consumed in Body when grounded.
#[derive(Component, Debug, Clone, Copy, Default)]
#[component(storage = "SparseSet")]
pub struct JumpWish;

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

pub fn apply_locomotion_capsule(commands: &mut Commands, body: Entity, hull: LocomotionCapsule) {
	let collider = Collider::capsule(hull.radius, hull.length);
	let mut caster_shape = collider.clone();
	caster_shape.set_scale(Vec3::splat(0.99), 10);
	commands.entity(body).insert((
		hull,
		collider,
		ShapeCaster::new(caster_shape, Vec3::ZERO, Quat::IDENTITY, Dir3::NEG_Y)
			.with_max_distance(GROUND_CAST_DISTANCE)
			.with_query_filter(SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed)),
	));
}

/// Stamp the dynamic character controller onto an existing scene plant.
///
/// The caller owns [`Transform`], so semantic LOD fulfillment can preserve the
/// transform authored by its scene recipe.
pub fn apply_character_controller(commands: &mut Commands, body: Entity, hull: LocomotionCapsule) {
	commands.entity(body).insert((
		CharacterController,
		Visibility::default(),
		RigidBody::Dynamic,
		PhysicsInteractionLayer::animated_layers(),
		LockedAxes::ROTATION_LOCKED,
		MovementAcceleration(MOVE_ACCEL),
		MovementDampingFactor(MOVE_DAMPING),
		JumpImpulse(JUMP_IMPULSE),
		MaxSlopeAngle(MAX_SLOPE_ANGLE),
		MoveWish::default(),
		Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
		Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
		GravityScale(1.25),
	));
	apply_locomotion_capsule(commands, body, hull);
}

/// Copy [`CharacterLocomotion`] onto newly spawned capsules. Runs in PostUpdate
/// so scene materialize in Update is visible the same frame.
pub(crate) fn sync_character_locomotion(
	locomotion: Res<CharacterLocomotion>,
	mut slopes: Query<&mut MaxSlopeAngle, Added<MaxSlopeAngle>>,
) {
	for mut slope in &mut slopes {
		slope.0 = locomotion.max_slope_angle;
	}
}

/// Scale run acceleration and jump impulse from character-sheet factors.
pub fn apply_character_mobility(
	commands: &mut Commands,
	body: Entity,
	running_factor: f32,
	jump_factor: f32,
) {
	commands.entity(body).insert((
		MovementAcceleration(MOVE_ACCEL * running_factor.max(0.01)),
		JumpImpulse(JUMP_IMPULSE * jump_factor.max(0.01)),
	));
}

pub(crate) fn spawn_character_controller(
	commands: &mut Commands,
	translation: Vec3,
	hull: LocomotionCapsule,
) -> Entity {
	let body = commands.spawn(Transform::from_translation(translation)).id();
	apply_character_controller(commands, body, hull);
	body
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
			// Clear a hop that never left the ground (snap / short impulse) so the
			// jump clip does not stick while the capsule is already walking.
			if landed || (jumping.is_some() && velocity.y <= 0.05) {
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

/// Most upright contact within the walkable slope, if any.
fn walkable_ground_normal(hits: &ShapeHits, max_slope: Option<&MaxSlopeAngle>) -> Option<Vec3> {
	let mut best: Option<(f32, Vec3)> = None;
	for hit in hits.iter() {
		let normal = (-hit.normal2).normalize_or_zero();
		if normal.length_squared() < 1e-8 {
			continue;
		}
		let angle = normal.angle_between(Vec3::Y).abs();
		if let Some(max) = max_slope {
			if angle > max.0 {
				continue;
			}
		}
		if best.is_none_or(|(best_angle, _)| angle < best_angle) {
			best = Some((angle, normal));
		}
	}
	best.map(|(_, normal)| normal)
}

/// Accelerate along the ground plane when a walkable normal is known; else XZ only.
fn accelerate_wish(
	velocity: &mut LinearVelocity,
	wish: Vec3,
	accel: f32,
	dt: f32,
	ground_normal: Option<Vec3>,
) {
	let wish = Vec3::new(wish.x, 0.0, wish.z).normalize_or_zero();
	if wish.length_squared() < 1e-8 {
		return;
	}
	let drive = match ground_normal {
		Some(normal) => {
			let along = (wish - normal * wish.dot(normal)).normalize_or_zero();
			if along.length_squared() > 1e-8 {
				along
			} else {
				wish
			}
		}
		None => wish,
	};
	if ground_normal.is_some() {
		**velocity += drive * accel * dt;
	} else {
		velocity.x += drive.x * accel * dt;
		velocity.z += drive.z * accel * dt;
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
			&ShapeHits,
			Option<&MaxSlopeAngle>,
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
		for (entity, look, hits, max_slope, accel, jump, mut velocity, grounded) in &mut controllers
		{
			let yaw = Quat::from_axis_angle(Vec3::Y, look.yaw);
			let forward = yaw * -Vec3::Z;
			let right = yaw * Vec3::X;
			match action {
				MovementAction::Move(direction) => {
					let wish = (right * direction.x + forward * direction.y).normalize_or_zero();
					let ground =
						grounded.then(|| walkable_ground_normal(hits, max_slope)).flatten();
					accelerate_wish(&mut velocity, wish, accel.0, dt, ground);
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

/// Apply [`MoveWish`] for non-player capsules (NPC intelligence, etc.).
///
/// Pad-driven players use [`apply_character_movement`] from [`MovementAction`] instead.
pub(crate) fn apply_wish_movement(
	time: Res<Time>,
	mut controllers: Query<
		(
			&MoveWish,
			&ShapeHits,
			Option<&MaxSlopeAngle>,
			&MovementAcceleration,
			&mut LinearVelocity,
			Has<Grounded>,
		),
		(With<CharacterController>, Without<Player>),
	>,
) {
	let dt = time.delta_secs();
	for (wish, hits, max_slope, accel, mut velocity, grounded) in &mut controllers {
		let dir = Vec3::new(wish.0.x, 0.0, wish.0.z);
		if dir.length_squared() < 1e-6 {
			continue;
		}
		let ground = grounded.then(|| walkable_ground_normal(hits, max_slope)).flatten();
		accelerate_wish(&mut velocity, dir, accel.0, dt, ground);
	}
}

/// Consume [`JumpWish`] on grounded NPC capsules. Players jump via [`MovementAction`].
pub(crate) fn apply_wish_jump(
	mut commands: Commands,
	mut controllers: Query<
		(Entity, &JumpImpulse, &mut LinearVelocity, Has<Grounded>),
		(With<CharacterController>, With<JumpWish>, Without<Player>),
	>,
) {
	for (entity, jump, mut velocity, grounded) in &mut controllers {
		commands.entity(entity).remove::<JumpWish>();
		if grounded {
			velocity.y = jump.0;
			commands.entity(entity).insert(Jumping { left_ground: false });
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_locomotion_keeps_legacy_slope() {
		assert!((CharacterLocomotion::default().max_slope_angle - MAX_SLOPE_ANGLE).abs() < 1e-6);
	}

	#[test]
	fn grounded_wish_climbs_along_the_slope() {
		let mut velocity = LinearVelocity(Vec3::ZERO);
		let slope = 70.0_f32.to_radians();
		// Hill rises in +X, so the normal tilts downhill (−X).
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		accelerate_wish(&mut velocity, Vec3::X, 40.0, 1.0, Some(normal));
		assert!(velocity.y > 1.0, "grounded drive must add uphill Y, got {}", velocity.y);
		assert!(velocity.x > 0.0);
	}

	#[test]
	fn airborne_wish_stays_xz() {
		let mut velocity = LinearVelocity(Vec3::new(0.0, -5.0, 0.0));
		accelerate_wish(&mut velocity, Vec3::X, 40.0, 1.0, None);
		assert!((velocity.y + 5.0).abs() < 1e-4);
		assert!(velocity.x > 0.0);
	}
}
