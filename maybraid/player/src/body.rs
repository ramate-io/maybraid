//! Dynamic capsule: grounded, wish accel, jump.

use avian3d::prelude::*;
use bevy::ecs::query::Has;
use bevy::prelude::*;
use crozon_characters::LocomotionCapsule;
use lod_avian::PhysicsInteractionLayer;
use std::f32::consts::PI;

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

/// One-shot jump request. Consumed in Body when grounded.
#[derive(Component, Debug, Clone, Copy, Default)]
#[component(storage = "SparseSet")]
pub struct JumpWish;

/// Takeoff (grounded) → air → land recovery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JumpPhase {
	Takeoff,
	Air,
	Land,
}

/// In-flight jump. Impulse waits until takeoff ends; land holds until recovery.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
#[component(storage = "SparseSet")]
pub struct Jumping {
	pub phase: JumpPhase,
	pub left_ground: bool,
	pub leaping: bool,
	pub phase_elapsed: f32,
	pub launch_vy: f32,
}

/// Horizontal speed at which a jump is a running leap rather than a standing hop.
pub const LEAP_SPEED: f32 = 5.0;
const JUMP_TAKEOFF_DURATION: f32 = 0.14;
const JUMP_LAND_DURATION: f32 = 0.24;
const FAILED_HOP_SECONDS: f32 = 0.05;

impl Jumping {
	pub fn start(xz_speed: f32) -> Self {
		Self {
			phase: JumpPhase::Takeoff,
			left_ground: false,
			leaping: xz_speed > LEAP_SPEED,
			phase_elapsed: 0.0,
			launch_vy: 0.0,
		}
	}

	pub fn airborne(&self) -> bool {
		self.phase == JumpPhase::Air
	}

	/// Normalized 0..1 takeoff / air / land for the leap sampler.
	pub fn leap_progress(&self, vertical_velocity: f32) -> f32 {
		match self.phase {
			JumpPhase::Takeoff => {
				(self.phase_elapsed / JUMP_TAKEOFF_DURATION).clamp(0.0, 1.0) * LEAP_TAKEOFF_END
			}
			JumpPhase::Air => {
				let launch = self.launch_vy.max(1e-3);
				let frac = (1.0 - vertical_velocity / launch).clamp(0.0, 2.0) * 0.5;
				LEAP_TAKEOFF_END + frac.clamp(0.0, 1.0) * (LEAP_AIR_END - LEAP_TAKEOFF_END)
			}
			JumpPhase::Land => {
				LEAP_AIR_END
					+ (self.phase_elapsed / JUMP_LAND_DURATION).clamp(0.0, 1.0)
						* (1.0 - LEAP_AIR_END)
			}
		}
	}
}

/// Advance a jump. Returns true when the shot is done and [`Jumping`] should drop.
///
/// Takeoff stays on the ground; impulse fires when takeoff ends or the capsule
/// leaves the mesh. Land starts on first grounded frame after leaving.
pub fn tick_jump(
	jump: &mut Jumping,
	grounded: bool,
	velocity: &mut Vec3,
	impulse: f32,
	dt: f32,
) -> bool {
	match jump.phase {
		JumpPhase::Takeoff => {
			jump.phase_elapsed += dt;
			if jump.phase_elapsed >= JUMP_TAKEOFF_DURATION || !grounded {
				velocity.y = impulse;
				jump.launch_vy = impulse;
				jump.phase = JumpPhase::Air;
				jump.phase_elapsed = 0.0;
				if !grounded {
					jump.left_ground = true;
				}
			}
			false
		}
		JumpPhase::Air => {
			jump.phase_elapsed += dt;
			if grounded && jump.left_ground {
				jump.phase = JumpPhase::Land;
				jump.phase_elapsed = 0.0;
				false
			} else {
				grounded
					&& !jump.left_ground
					&& velocity.y <= 0.05
					&& jump.phase_elapsed > FAILED_HOP_SECONDS
			}
		}
		JumpPhase::Land => {
			jump.phase_elapsed += dt;
			jump.phase_elapsed >= JUMP_LAND_DURATION
		}
	}
}

/// Leap sampler windows (must match `malo_animations::animations::leap`).
const LEAP_TAKEOFF_END: f32 = 0.18;
const LEAP_AIR_END: f32 = 0.72;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayerControlSystems;

#[derive(Component)]
pub struct CharacterController;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Grounded;

#[derive(Component)]
pub(crate) struct MovementAcceleration(pub f32);

#[derive(Component)]
pub(crate) struct MovementDampingFactor(pub f32);

#[derive(Component)]
pub(crate) struct JumpImpulse(pub f32);

#[derive(Component)]
pub(crate) struct MaxSlopeAngle(pub f32);

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
			&& jumping.as_ref().is_none_or(|jump| !jump.airborne())
			&& velocity.y > -GROUND_SNAP_SPEED
			&& velocity.y < GROUND_SNAP_SPEED
		{
			is_grounded = true;
		}
		if is_grounded {
			commands.entity(entity).insert(Grounded);
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
/// Airborne jump (and true air) are XZ only (gravity owns Y). Takeoff and land
/// stay on the walkable plane. A walkable hit this frame is the plane. Last
/// plane is only for a [`Grounded`] snap when the caster missed — never after
/// walking off a ridge.
pub fn ground_plane_for_wish(
	contact: Option<Vec3>,
	last: Option<Vec3>,
	grounded: bool,
	airborne: bool,
) -> Option<Vec3> {
	if airborne {
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
	gravity: Vec3,
) {
	let drive = wish_on_ground(wish, ground_normal);
	if drive.length_squared() < 1e-8 {
		return;
	}
	if ground_normal.is_some() {
		let horizontal = Vec2::new(drive.x, drive.z).length().max(0.25);
		let slope_accel = (accel / horizontal - gravity.dot(drive)).max(0.0);
		**velocity += drive * slope_accel * dt;
	} else {
		velocity.x += drive.x * accel * dt;
		velocity.z += drive.z * accel * dt;
	}
}

/// Apply [`MoveWish`] for every capsule. Pad intent and NPC drive both write it.
pub(crate) fn apply_wish_movement(
	time: Res<Time>,
	gravity: Option<Res<Gravity>>,
	mut controllers: Query<
		(
			&MoveWish,
			&ShapeHits,
			Option<&MaxSlopeAngle>,
			Option<&WalkableGround>,
			&MovementAcceleration,
			&GravityScale,
			&mut LinearVelocity,
			Has<Grounded>,
			Option<&Jumping>,
		),
		With<CharacterController>,
	>,
) {
	let dt = time.delta_secs();
	let gravity = gravity.map(|gravity| gravity.0).unwrap_or(Vec3::ZERO);
	for (wish, hits, max_slope, walkable, accel, gravity_scale, mut velocity, grounded, jumping) in
		&mut controllers
	{
		if wish.0.length_squared() < 1e-6 {
			continue;
		}
		let contact = walkable_ground_normal(hits, max_slope);
		let airborne = jumping.is_some_and(Jumping::airborne);
		let ground =
			ground_plane_for_wish(contact, walkable.map(|plane| plane.normal), grounded, airborne);
		accelerate_wish(&mut velocity, wish.0, accel.0, dt, ground, gravity * gravity_scale.0);
	}
}

/// Start a jump. Impulse waits for [`advance_jump_phases`].
pub(crate) fn apply_wish_jump(
	mut commands: Commands,
	mut controllers: Query<
		(Entity, &LinearVelocity, Has<Grounded>),
		(With<CharacterController>, With<JumpWish>, Without<Jumping>),
	>,
) {
	for (entity, velocity, grounded) in &mut controllers {
		commands.entity(entity).remove::<JumpWish>();
		if grounded {
			let xz = Vec3::new(velocity.x, 0.0, velocity.z).length();
			commands.entity(entity).insert(Jumping::start(xz));
		}
	}
}

pub(crate) fn advance_jump_phases(
	mut commands: Commands,
	time: Res<Time>,
	mut controllers: Query<
		(Entity, &JumpImpulse, &mut LinearVelocity, &mut Jumping, Has<Grounded>),
		With<CharacterController>,
	>,
) {
	let dt = time.delta_secs();
	for (entity, impulse, mut velocity, mut jumping, grounded) in &mut controllers {
		if tick_jump(&mut jumping, grounded, &mut velocity, impulse.0, dt) {
			commands.entity(entity).remove::<Jumping>();
		}
	}
}

pub(crate) fn apply_movement_damping(
	mut query: Query<
		(
			&MovementDampingFactor,
			&ShapeHits,
			Option<&MaxSlopeAngle>,
			Option<&WalkableGround>,
			Has<Grounded>,
			Option<&Jumping>,
			&mut LinearVelocity,
		),
		With<CharacterController>,
	>,
) {
	for (damping, hits, max_slope, walkable, grounded, jumping, mut velocity) in &mut query {
		let contact = walkable_ground_normal(hits, max_slope);
		let airborne = jumping.is_some_and(Jumping::airborne);
		let ground =
			ground_plane_for_wish(contact, walkable.map(|plane| plane.normal), grounded, airborne);
		damp_movement(&mut velocity, damping.0, ground);
	}
}

/// Damp locomotion without tipping slope-tangent velocity into or away from
/// the ground. Airborne motion keeps vertical velocity under gravity.
fn damp_movement(velocity: &mut LinearVelocity, damping: f32, ground_normal: Option<Vec3>) {
	if let Some(normal) = ground_normal.map(Vec3::normalize_or_zero) {
		if normal.length_squared() > 1e-8 {
			let tangent = **velocity - normal * velocity.dot(normal);
			**velocity = tangent * damping;
			return;
		}
	}
	velocity.x *= damping;
	velocity.z *= damping;
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
		accelerate_wish(&mut velocity, Vec3::X, 40.0, 1.0, Some(normal), Vec3::ZERO);
		assert!(velocity.y > 1.0, "grounded drive must add uphill Y, got {}", velocity.y);
		assert!(velocity.x > 0.0);
	}

	#[test]
	fn slope_drive_preserves_horizontal_pace_against_gravity() {
		let gravity = Vec3::NEG_Y * 9.81 * 1.25;
		let slope = 60.0_f32.to_radians();
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		let tangent_gravity = gravity - normal * gravity.dot(normal);
		let mut uphill = LinearVelocity(Vec3::ZERO);
		let mut downhill = LinearVelocity(Vec3::ZERO);

		accelerate_wish(&mut uphill, Vec3::X, 40.0, 1.0, Some(normal), gravity);
		accelerate_wish(&mut downhill, Vec3::NEG_X, 40.0, 1.0, Some(normal), gravity);
		uphill.0 += tangent_gravity;
		downhill.0 += tangent_gravity;

		assert!((uphill.x - 40.0).abs() < 1e-4, "{uphill:?}");
		assert!((downhill.x + 40.0).abs() < 1e-4, "{downhill:?}");
	}

	#[test]
	fn uphill_point_wish_climbs_along_the_slope() {
		let mut velocity = LinearVelocity(Vec3::ZERO);
		let slope = 45.0_f32.to_radians();
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		accelerate_wish(
			&mut velocity,
			Vec3::new(1.0, 1.0, 0.0),
			40.0,
			1.0,
			Some(normal),
			Vec3::ZERO,
		);
		assert!(velocity.y > 1.0, "XZ heading on the plane must add uphill Y, got {}", velocity.y);
		assert!((velocity.x - 40.0).abs() < 1e-4, "{velocity:?}");
		assert!((velocity.y - 40.0).abs() < 1e-4, "{velocity:?}");
	}

	#[test]
	fn last_plane_is_only_for_grounded_snap() {
		let slope = Vec3::new(-0.2, 0.98, 0.0);
		assert_eq!(ground_plane_for_wish(None, Some(slope), true, false), Some(slope));
		assert_eq!(ground_plane_for_wish(None, Some(slope), false, false), None);
		assert_eq!(ground_plane_for_wish(None, Some(Vec3::Y), true, true), None);
	}

	#[test]
	fn takeoff_delays_impulse_until_the_window_ends() {
		let mut jump = Jumping::start(0.0);
		let mut velocity = Vec3::ZERO;
		assert!(!tick_jump(&mut jump, true, &mut velocity, 8.0, 0.05));
		assert_eq!(jump.phase, JumpPhase::Takeoff);
		assert!(velocity.y.abs() < 1e-4);
		assert!(!tick_jump(&mut jump, true, &mut velocity, 8.0, 0.12));
		assert_eq!(jump.phase, JumpPhase::Air);
		assert!((velocity.y - 8.0).abs() < 1e-4);
	}

	#[test]
	fn leaving_the_ground_during_takeoff_launches() {
		let mut jump = Jumping::start(6.0);
		let mut velocity = Vec3::ZERO;
		assert!(jump.leaping);
		assert!(!tick_jump(&mut jump, false, &mut velocity, 8.0, 0.016));
		assert_eq!(jump.phase, JumpPhase::Air);
		assert!(jump.left_ground);
		assert!((velocity.y - 8.0).abs() < 1e-4);
	}

	#[test]
	fn land_starts_on_contact_and_finishes_after_recovery() {
		let mut jump = Jumping::start(0.0);
		jump.phase = JumpPhase::Air;
		jump.left_ground = true;
		jump.launch_vy = 8.0;
		let mut velocity = Vec3::new(0.0, -2.0, 0.0);
		assert!(!tick_jump(&mut jump, true, &mut velocity, 8.0, 0.016));
		assert_eq!(jump.phase, JumpPhase::Land);
		assert!(!tick_jump(&mut jump, true, &mut velocity, 8.0, 0.1));
		assert!(tick_jump(&mut jump, true, &mut velocity, 8.0, 0.2));
	}

	#[test]
	fn leap_progress_covers_takeoff_air_and_land() {
		let takeoff = Jumping::start(0.0);
		assert!(takeoff.leap_progress(0.0) < 0.18);
		let mut air = Jumping::start(0.0);
		air.phase = JumpPhase::Air;
		air.launch_vy = 8.0;
		let apex = air.leap_progress(0.0);
		let descent = air.leap_progress(-8.0);
		assert!(apex > 0.18 && apex < 0.72, "{apex}");
		assert!(descent > apex && descent <= 0.72, "{descent}");
		let mut land = air;
		land.phase = JumpPhase::Land;
		land.phase_elapsed = JUMP_LAND_DURATION;
		assert!((land.leap_progress(0.0) - 1.0).abs() < 1e-4);
	}

	#[test]
	fn airborne_wish_stays_xz() {
		let mut velocity = LinearVelocity(Vec3::new(0.0, -5.0, 0.0));
		accelerate_wish(
			&mut velocity,
			Vec3::new(1.0, 4.0, 0.0),
			40.0,
			1.0,
			None,
			Vec3::NEG_Y * 9.81,
		);
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

	#[test]
	fn slope_damping_is_symmetric_uphill_and_downhill() {
		let slope = 45.0_f32.to_radians();
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		let uphill = wish_on_ground(Vec3::X, Some(normal)) * 10.0;
		let mut uphill = LinearVelocity(uphill);
		let mut downhill = LinearVelocity(-uphill.0);

		damp_movement(&mut uphill, 0.92, Some(normal));
		damp_movement(&mut downhill, 0.92, Some(normal));

		assert!((uphill.length() - downhill.length()).abs() < 1e-5);
		assert!(uphill.dot(normal).abs() < 1e-5, "{uphill:?}");
		assert!(downhill.dot(normal).abs() < 1e-5, "{downhill:?}");
		assert!(uphill.y > 0.0);
		assert!(downhill.y < 0.0);
	}
}
