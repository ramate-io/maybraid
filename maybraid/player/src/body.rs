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

/// Last walkable contact plane. Default is world up (flat XZ heading).
///
/// Used only while [`Grounded`]: if the down-cast misses for a snap frame, drive
/// still follows this plane instead of ramming world XZ into the mesh. Off the
/// ground, gravity owns Y.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct WalkableGround {
	pub normal: Vec3,
}

impl Default for WalkableGround {
	fn default() -> Self {
		Self { normal: Vec3::Y }
	}
}

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
		WalkableGround::default(),
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
		if let Some(normal) = walkable_contact_normal(hits, max_slope_angle.map(|angle| angle.0)) {
			commands.entity(entity).insert(WalkableGround { normal });
		}
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
pub fn walkable_contact_normal(hits: &ShapeHits, max_slope_angle: Option<f32>) -> Option<Vec3> {
	let mut best: Option<(f32, Vec3)> = None;
	for hit in hits.iter() {
		let normal = (-hit.normal2).normalize_or_zero();
		if normal.length_squared() < 1e-8 {
			continue;
		}
		let angle = normal.angle_between(Vec3::Y).abs();
		if let Some(max) = max_slope_angle {
			if angle > max {
				continue;
			}
		}
		if best.is_none_or(|(best_angle, _)| angle < best_angle) {
			best = Some((angle, normal));
		}
	}
	best.map(|(_, normal)| normal)
}

fn walkable_ground_normal(hits: &ShapeHits, max_slope: Option<&MaxSlopeAngle>) -> Option<Vec3> {
	walkable_contact_normal(hits, max_slope.map(|angle| angle.0))
}

/// Contact plane used to turn a wish into capsule accel.
///
/// Jumping and true air are XZ only (gravity owns Y). A walkable hit this frame
/// is the plane. Last plane is only for a [`Grounded`] snap when the caster
/// missed — never after walking off a ridge.
pub fn ground_plane_for_wish(
	contact: Option<Vec3>,
	last: Option<Vec3>,
	grounded: bool,
	jumping: bool,
) -> Option<Vec3> {
	if jumping {
		return None;
	}
	if let Some(normal) = contact {
		return Some(normal);
	}
	if grounded {
		return last;
	}
	None
}

/// Unit drive for a movement wish. Compass heading on the contact plane.
///
/// Wish Y is ignored so a raised waypoint cannot become a launch vector. No
/// plane → XZ only (does not fly).
pub fn wish_on_ground(wish: Vec3, ground_normal: Option<Vec3>) -> Vec3 {
	let heading = Vec3::new(wish.x, 0.0, wish.z);
	if heading.length_squared() < 1e-8 {
		return Vec3::ZERO;
	}
	let Some(normal) = ground_normal.map(|normal| normal.normalize_or_zero()) else {
		return heading.normalize();
	};
	if normal.length_squared() < 1e-8 {
		return heading.normalize();
	}
	let along = heading - normal * heading.dot(normal);
	if along.length_squared() > 1e-8 {
		return along.normalize();
	}
	Vec3::ZERO
}

/// Accelerate along the ground plane when a walkable normal is known; else XZ only.
fn accelerate_wish(
	velocity: &mut LinearVelocity,
	wish: Vec3,
	accel: f32,
	dt: f32,
	ground_normal: Option<Vec3>,
) {
	let drive = wish_on_ground(wish, ground_normal);
	if drive.length_squared() < 1e-8 {
		return;
	}
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
			Option<&WalkableGround>,
			&MovementAcceleration,
			&JumpImpulse,
			&mut LinearVelocity,
			Has<Grounded>,
			Has<Jumping>,
		),
		(With<CharacterController>, With<Player>),
	>,
) {
	let dt = time.delta_secs();
	for action in reader.read() {
		for (
			entity,
			look,
			hits,
			max_slope,
			walkable,
			accel,
			jump,
			mut velocity,
			grounded,
			jumping,
		) in &mut controllers
		{
			let yaw = Quat::from_axis_angle(Vec3::Y, look.yaw);
			let forward = yaw * -Vec3::Z;
			let right = yaw * Vec3::X;
			match action {
				MovementAction::Move(direction) => {
					let wish = (right * direction.x + forward * direction.y).normalize_or_zero();
					let contact = walkable_ground_normal(hits, max_slope);
					let ground = ground_plane_for_wish(
						contact,
						walkable.map(|plane| plane.normal),
						grounded,
						jumping,
					);
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
			Option<&WalkableGround>,
			&MovementAcceleration,
			&mut LinearVelocity,
			Has<Grounded>,
			Has<Jumping>,
		),
		(With<CharacterController>, Without<Player>),
	>,
) {
	let dt = time.delta_secs();
	for (wish, hits, max_slope, walkable, accel, mut velocity, grounded, jumping) in &mut controllers
	{
		if wish.0.length_squared() < 1e-6 {
			continue;
		}
		let contact = walkable_ground_normal(hits, max_slope);
		let ground = ground_plane_for_wish(
			contact,
			walkable.map(|plane| plane.normal),
			grounded,
			jumping,
		);
		accelerate_wish(&mut velocity, wish.0, accel.0, dt, ground);
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
	fn uphill_point_wish_climbs_along_the_slope() {
		let mut velocity = LinearVelocity(Vec3::ZERO);
		let slope = 45.0_f32.to_radians();
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		accelerate_wish(&mut velocity, Vec3::new(1.0, 1.0, 0.0), 40.0, 1.0, Some(normal));
		assert!(velocity.y > 1.0, "XZ heading on the plane must add uphill Y, got {}", velocity.y);
		assert!(velocity.x > 0.0);
		assert!(velocity.y.abs() < 40.0);
	}

	#[test]
	fn last_plane_is_only_for_grounded_snap() {
		let slope = Vec3::new(-0.2, 0.98, 0.0);
		assert_eq!(ground_plane_for_wish(None, Some(slope), true, false), Some(slope));
		assert_eq!(ground_plane_for_wish(None, Some(slope), false, false), None);
		assert_eq!(ground_plane_for_wish(None, Some(Vec3::Y), true, true), None);
	}

	#[test]
	fn airborne_wish_stays_xz() {
		let mut velocity = LinearVelocity(Vec3::new(0.0, -5.0, 0.0));
		accelerate_wish(&mut velocity, Vec3::new(1.0, 4.0, 0.0), 40.0, 1.0, None);
		assert!((velocity.y + 5.0).abs() < 1e-4);
		assert!(velocity.x > 0.0);
	}

	#[test]
	fn wish_on_ground_does_not_fly() {
		let drive = wish_on_ground(Vec3::new(0.0, 1.0, 0.0), None);
		assert!(drive.length() < 1e-4, "{drive}");
		let slope = 45.0_f32.to_radians();
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		let along = wish_on_ground(Vec3::X, Some(normal));
		assert!(along.y > 0.0);
		assert!((along.dot(normal)).abs() < 1e-4, "{along} · {normal}");
		let from_3d = wish_on_ground(Vec3::new(1.0, 10.0, 0.0), Some(normal));
		assert!((from_3d - along).length() < 1e-4, "{from_3d} vs {along}");
	}
}
