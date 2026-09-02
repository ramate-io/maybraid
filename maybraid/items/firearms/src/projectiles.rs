//! Emissive blaster projectiles spawned from the receiver `barrel` bone.
//!
//! Bolts and bullets are query-only (`Sensor` + projectile layer). They do not
//! bounce: a sweep along each step charges [`Flight::through`] while overlapping
//! [`PhysicsInteractionLayer::Fixed`]. Lasers are visuals only.

use avian3d::prelude::*;
use avian3d::schedule::PhysicsSchedulePlugin;
use bevy::ecs::query::Has;
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;

use firearms_components::{BoneMap, FirearmHostSystems, FirearmMembers, FirearmRoot, RigRoot};

use crate::impact::{setup_impact_effects, spawn_impact, tick_impact_bursts, ImpactEffects};

/// Authored rest length of the `barrel` bone (head → tail) in bone-local units.
pub const BARREL_REST_LENGTH: f32 = 1.0;

/// When false, weapons do not spawn shots and lasers freeze.
#[derive(Resource, Clone, Copy, PartialEq, Eq)]
pub struct WeaponsArmed(pub bool);

impl Default for WeaponsArmed {
	fn default() -> Self {
		Self(true)
	}
}

/// This [`Weapon`] only fires while [`TriggerFire`] is set (right trigger / click).
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FireOnTrigger;

/// Held analog fire (RT). Playgrounds set this from [`CharacterIntent::UseItem`].
#[derive(Resource, Clone, Copy, Default)]
pub struct TriggerFire(pub bool);

/// Multiplier on path length spent inside this collider. Missing is `1.0`.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PenetrationCost(pub f32);

impl Default for PenetrationCost {
	fn default() -> Self {
		Self(1.0)
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoltSpec {
	pub length: f32,
	pub radius: f32,
	pub speed: f32,
	pub max_range: f32,
	pub max_age: f32,
	pub penetration: f32,
	pub color: Color,
}

impl Default for BoltSpec {
	fn default() -> Self {
		Self {
			length: 0.55,
			radius: 0.055,
			speed: 180.0,
			max_range: 36.0,
			max_age: 2.0,
			penetration: 0.85,
			color: Color::srgb(0.35, 0.95, 1.0),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BulletSpec {
	pub length: f32,
	pub radius: f32,
	pub speed: f32,
	pub max_range: f32,
	pub max_age: f32,
	pub penetration: f32,
	pub color: Color,
}

impl Default for BulletSpec {
	fn default() -> Self {
		Self {
			length: 0.32,
			radius: 0.04,
			speed: 28.0,
			max_range: 42.0,
			max_age: 3.0,
			penetration: 0.25,
			color: Color::srgb(1.0, 0.72, 0.22),
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LaserSpec {
	pub radius: f32,
	pub max_length: f32,
	pub max_time: f32,
	pub color: Color,
}

impl Default for LaserSpec {
	fn default() -> Self {
		Self { radius: 0.035, max_length: 22.0, max_time: 0.7, color: Color::srgb(1.0, 0.18, 0.22) }
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProjectileLoad {
	Bolt(BoltSpec),
	Bullet(BulletSpec),
	Laser(LaserSpec),
}

impl ProjectileLoad {
	pub fn label(self) -> &'static str {
		match self {
			Self::Bolt(_) => "bolt",
			Self::Bullet(_) => "bullet",
			Self::Laser(_) => "laser",
		}
	}
}

/// Auto-fire on a [`FirearmRoot`]. Interval is unused for lasers (they grow in place).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Weapon {
	pub load: ProjectileLoad,
	pub interval: f32,
	pub cooldown: f32,
	pub laser: Option<Entity>,
}

impl Weapon {
	pub fn new(load: ProjectileLoad, interval: f32) -> Self {
		Self { load, interval, cooldown: 0.0, laser: None }
	}

	pub fn bolt() -> Self {
		Self::new(ProjectileLoad::Bolt(BoltSpec::default()), 0.32)
	}

	pub fn bullet() -> Self {
		Self::new(ProjectileLoad::Bullet(BulletSpec::default()), 0.4)
	}

	pub fn laser() -> Self {
		Self::new(ProjectileLoad::Laser(LaserSpec::default()), 0.0)
	}
}

/// Path / time / through-solid budgets for a bolt or bullet.
#[derive(Component, Debug, Clone, Copy)]
pub struct Flight {
	pub origin: Vec3,
	pub last: Vec3,
	pub path: f32,
	pub through: f32,
	pub age: f32,
	pub max_range: f32,
	pub max_through: f32,
	pub max_age: f32,
}

impl Flight {
	pub fn spawn(origin: Vec3, max_range: f32, max_through: f32, max_age: f32) -> Self {
		Self {
			origin,
			last: origin,
			path: 0.0,
			through: 0.0,
			age: 0.0,
			max_range,
			max_through,
			max_age,
		}
	}

	pub fn exhausted(self) -> bool {
		self.path > self.max_range || self.through > self.max_through || self.age > self.max_age
	}
}

#[derive(Component, Debug, Clone, Copy)]
pub struct LaserBeam {
	pub spec: LaserSpec,
	pub age: f32,
}

/// Avian + fire / despawn / laser grow. Hosts still add [`crate::FirearmHostsPlugin`].
pub struct FirearmWeaponsPlugin;

impl Plugin for FirearmWeaponsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<PhysicsSchedulePlugin>() {
			app.add_plugins(PhysicsPlugins::default());
		}
		if !app.is_plugin_added::<bevy_hanabi::HanabiPlugin>() {
			app.add_plugins(bevy_hanabi::HanabiPlugin);
		}
		app.init_resource::<WeaponsArmed>()
			.init_resource::<TriggerFire>()
			.add_systems(Startup, setup_impact_effects)
			.add_systems(
				PostUpdate,
				(fire_weapons, tick_lasers, tick_flights, tick_impact_bursts)
					.after(TransformSystems::Propagate)
					.after(FirearmHostSystems::Pose),
			);
	}
}

/// World muzzle (barrel tail) and unit fire direction (bone +Y).
pub fn muzzle_world(global: &GlobalTransform) -> (Vec3, Vec3) {
	let muzzle = global.transform_point(Vec3::Y * BARREL_REST_LENGTH);
	let origin = global.translation();
	let dir = (muzzle - origin).normalize_or(Vec3::Y);
	(muzzle, dir)
}

/// Solid length along a step of `ds`.
///
/// `forward_hit` / `backward_hit` are distances to the first face from each end.
pub fn through_length(
	ds: f32,
	start_inside: bool,
	end_inside: bool,
	forward_hit: Option<f32>,
	backward_hit: Option<f32>,
) -> f32 {
	if ds <= 1e-8 {
		return 0.0;
	}
	let clamp = |x: f32| x.clamp(0.0, ds);
	match (start_inside, end_inside) {
		(true, true) => ds,
		(false, false) => match (forward_hit, backward_hit) {
			(Some(enter), Some(leave)) => clamp(ds - enter - leave),
			_ => 0.0,
		},
		(false, true) => clamp(ds - forward_hit.unwrap_or(0.0)),
		(true, false) => match (forward_hit, backward_hit) {
			(Some(exit), _) => clamp(exit),
			(_, Some(air)) => clamp(ds - air),
			(None, None) => ds,
		},
	}
}

fn barrel_global<'a>(
	members: &FirearmMembers,
	maps: &Query<&BoneMap, With<RigRoot>>,
	globals: &'a Query<&GlobalTransform>,
) -> Option<(Entity, &'a GlobalTransform)> {
	for member in members.iter() {
		let Ok(map) = maps.get(member) else {
			continue;
		};
		let bone = *map.by_name.get("barrel")?;
		let global = globals.get(bone).ok()?;
		return Some((bone, global));
	}
	None
}

fn glow_material(color: Color) -> StandardMaterial {
	let glow = color.to_linear();
	StandardMaterial {
		base_color: color,
		emissive: LinearRgba::rgb(glow.red * 14.0, glow.green * 14.0, glow.blue * 14.0),
		unlit: true,
		..default()
	}
}

fn capsule_along_y(direction: Vec3, muzzle: Vec3, length: f32, radius: f32) -> Transform {
	let rotation = Quat::from_rotation_arc(Vec3::Y, direction);
	let center = muzzle + direction * (radius + length * 0.5);
	Transform { translation: center, rotation, scale: Vec3::ONE }
}

fn penetration_filter(exclude: Entity) -> SpatialQueryFilter {
	SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed).with_excluded_entities([exclude])
}

fn overlapping(
	spatial: &SpatialQuery,
	collider: &Collider,
	at: Vec3,
	rotation: Quat,
	filter: &SpatialQueryFilter,
) -> bool {
	!spatial.shape_intersections(collider, at, rotation, filter).is_empty()
}

fn first_hit_cost(
	spatial: &SpatialQuery,
	collider: &Collider,
	at: Vec3,
	rotation: Quat,
	filter: &SpatialQueryFilter,
	costs: &Query<&PenetrationCost>,
) -> f32 {
	spatial
		.shape_intersections(collider, at, rotation, filter)
		.into_iter()
		.find_map(|entity| costs.get(entity).ok().map(|cost| cost.0))
		.unwrap_or(1.0)
}

fn sweep_hit(
	spatial: &SpatialQuery,
	collider: &Collider,
	origin: Vec3,
	rotation: Quat,
	direction: Dir3,
	ds: f32,
	filter: &SpatialQueryFilter,
) -> Option<f32> {
	let config = ShapeCastConfig::from_max_distance(ds);
	let hit = spatial.cast_shape(collider, origin, rotation, direction, &config, filter)?;
	(hit.distance < ds - 1e-4).then_some(hit.distance)
}

fn through_on_step(
	spatial: &SpatialQuery,
	collider: &Collider,
	start: Vec3,
	end: Vec3,
	rotation: Quat,
	filter: &SpatialQueryFilter,
	costs: &Query<&PenetrationCost>,
) -> f32 {
	let delta = end - start;
	let ds = delta.length();
	if ds <= 1e-5 {
		return 0.0;
	}
	let Ok(dir) = Dir3::new(delta) else {
		return 0.0;
	};
	let Ok(back) = Dir3::new(-delta) else {
		return 0.0;
	};
	let start_inside = overlapping(spatial, collider, start, rotation, filter);
	let end_inside = overlapping(spatial, collider, end, rotation, filter);
	let forward = sweep_hit(spatial, collider, start, rotation, dir, ds, filter);
	let backward = sweep_hit(spatial, collider, end, rotation, back, ds, filter);
	let sample_at = if end_inside { end } else { start };
	let cost = first_hit_cost(spatial, collider, sample_at, rotation, filter, costs);
	through_length(ds, start_inside, end_inside, forward, backward) * cost
}

fn spawn_flight(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	muzzle: Vec3,
	direction: Vec3,
	length: f32,
	radius: f32,
	speed: f32,
	max_range: f32,
	max_through: f32,
	max_age: f32,
	color: Color,
	gravity: f32,
) {
	let transform = capsule_along_y(direction, muzzle, length, radius);
	let origin = transform.translation;
	let collider = Collider::capsule(radius, length);
	// Sensors do not contribute collider mass; set it explicitly so Avian
	// does not warn about a massless dynamic body.
	commands.spawn((
		Name::new("projectile"),
		transform,
		Visibility::default(),
		Mesh3d(meshes.add(Capsule3d::new(radius, length))),
		MeshMaterial3d(materials.add(glow_material(color))),
		RigidBody::Dynamic,
		MassPropertiesBundle::from_shape(&collider, 1.0),
		collider,
		Sensor,
		PhysicsInteractionLayer::projectile_layers(),
		LockedAxes::ROTATION_LOCKED,
		LinearVelocity(direction * speed),
		GravityScale(gravity),
		Restitution::ZERO,
		Flight::spawn(origin, max_range, max_through, max_age),
	));
}

fn spawn_laser(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	barrel: Entity,
	spec: LaserSpec,
) -> Entity {
	let (translation, scale) = laser_local(spec, 0.0);
	commands
		.spawn((
			Name::new("laser"),
			Transform { translation, rotation: Quat::IDENTITY, scale },
			Visibility::default(),
			Mesh3d(meshes.add(Mesh::from(Cylinder::new(1.0, 1.0)))),
			MeshMaterial3d(materials.add(glow_material(spec.color))),
			ChildOf(barrel),
			LaserBeam { spec, age: 0.0 },
		))
		.id()
}

fn laser_local(spec: LaserSpec, age: f32) -> (Vec3, Vec3) {
	let t = (age / spec.max_time).clamp(0.0, 1.0);
	let len = (spec.max_length * t).max(0.02);
	let translation = Vec3::Y * (BARREL_REST_LENGTH + len * 0.5);
	let scale = Vec3::new(spec.radius, len, spec.radius);
	(translation, scale)
}

fn apply_laser_pose(transform: &mut Transform, spec: LaserSpec, age: f32) {
	let (translation, scale) = laser_local(spec, age);
	transform.translation = translation;
	transform.scale = scale;
}

pub fn fire_weapons(
	mut commands: Commands,
	time: Res<Time>,
	armed: Res<WeaponsArmed>,
	trigger: Res<TriggerFire>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut weapons: Query<
		(Entity, &FirearmMembers, &mut Weapon, Has<FireOnTrigger>),
		With<FirearmRoot>,
	>,
	maps: Query<&BoneMap, With<RigRoot>>,
	globals: Query<&GlobalTransform>,
	lasers: Query<&LaserBeam>,
) {
	if !armed.0 {
		return;
	}
	let dt = time.delta_secs();
	for (_root, members, mut weapon, manual) in &mut weapons {
		if manual && !trigger.0 {
			if matches!(weapon.load, ProjectileLoad::Bolt(_) | ProjectileLoad::Bullet(_)) {
				weapon.cooldown -= dt;
			}
			continue;
		}
		let Some((barrel, global)) = barrel_global(members, &maps, &globals) else {
			continue;
		};
		match weapon.load {
			ProjectileLoad::Laser(spec) => {
				let live = weapon.laser.filter(|entity| lasers.get(*entity).is_ok());
				if live.is_none() {
					weapon.laser =
						Some(spawn_laser(&mut commands, &mut meshes, &mut materials, barrel, spec));
				}
			}
			ProjectileLoad::Bolt(spec) => {
				weapon.cooldown -= dt;
				if weapon.cooldown > 0.0 {
					continue;
				}
				weapon.cooldown = weapon.interval;
				let (muzzle, dir) = muzzle_world(global);
				spawn_flight(
					&mut commands,
					&mut meshes,
					&mut materials,
					muzzle,
					dir,
					spec.length,
					spec.radius,
					spec.speed,
					spec.max_range,
					spec.penetration,
					spec.max_age,
					spec.color,
					0.0,
				);
			}
			ProjectileLoad::Bullet(spec) => {
				weapon.cooldown -= dt;
				if weapon.cooldown > 0.0 {
					continue;
				}
				weapon.cooldown = weapon.interval;
				let (muzzle, dir) = muzzle_world(global);
				spawn_flight(
					&mut commands,
					&mut meshes,
					&mut materials,
					muzzle,
					dir,
					spec.length,
					spec.radius,
					spec.speed,
					spec.max_range,
					spec.penetration,
					spec.max_age,
					spec.color,
					1.0,
				);
			}
		}
	}
}

pub fn tick_lasers(
	time: Res<Time>,
	armed: Res<WeaponsArmed>,
	mut lasers: Query<(&mut LaserBeam, &mut Transform)>,
) {
	if !armed.0 {
		return;
	}
	let dt = time.delta_secs();
	for (mut beam, mut transform) in &mut lasers {
		beam.age += dt;
		if beam.age >= beam.spec.max_time {
			beam.age = 0.0;
		}
		apply_laser_pose(&mut transform, beam.spec, beam.age);
	}
}

pub fn tick_flights(
	time: Res<Time>,
	spatial: SpatialQuery,
	costs: Query<&PenetrationCost>,
	effects: Option<Res<ImpactEffects>>,
	mut commands: Commands,
	mut flights: Query<(Entity, &mut Flight, &Transform, &Collider)>,
) {
	let dt = time.delta_secs();
	for (entity, mut flight, transform, collider) in &mut flights {
		let pos = transform.translation;
		let rotation = transform.rotation;
		let filter = penetration_filter(entity);
		let ds = pos.distance(flight.last);
		flight.age += dt;
		flight.path += ds;
		if let Some((point, normal)) =
			first_contact(&spatial, collider, flight.last, pos, rotation, &filter)
		{
			if let Some(effects) = effects.as_ref() {
				spawn_impact(&mut commands, effects, point, normal);
			}
		}
		flight.through +=
			through_on_step(&spatial, collider, flight.last, pos, rotation, &filter, &costs);
		flight.last = pos;
		if flight.exhausted() {
			commands.entity(entity).try_despawn();
		}
	}
}

/// First Fixed face this step, if the bolt was still in air at `start`.
fn first_contact(
	spatial: &SpatialQuery,
	collider: &Collider,
	start: Vec3,
	end: Vec3,
	rotation: Quat,
	filter: &SpatialQueryFilter,
) -> Option<(Vec3, Vec3)> {
	if overlapping(spatial, collider, start, rotation, filter) {
		return None;
	}
	let delta = end - start;
	let ds = delta.length();
	if ds <= 1e-5 {
		return None;
	}
	let dir = Dir3::new(delta).ok()?;
	let config = ShapeCastConfig::from_max_distance(ds);
	let hit = spatial.cast_shape(collider, start, rotation, dir, &config, filter)?;
	if hit.distance >= ds - 1e-4 {
		return None;
	}
	Some((hit.point1, hit.normal1.normalize_or(Vec3::Y)))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn muzzle_follows_bone_local_y() {
		let global = GlobalTransform::from(
			Transform::from_translation(Vec3::new(1.0, 2.0, 3.0))
				.with_rotation(Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2)),
		);
		let (muzzle, dir) = muzzle_world(&global);
		assert!((dir - Vec3::X).length() < 1.0e-4, "dir {dir}");
		assert!((muzzle - (Vec3::new(1.0, 2.0, 3.0) + Vec3::X)).length() < 1.0e-4);
	}

	#[test]
	fn laser_grows_from_barrel_tail() {
		let spec =
			LaserSpec { max_length: 10.0, max_time: 1.0, radius: 0.1, ..LaserSpec::default() };
		let (t0, s0) = laser_local(spec, 0.0);
		let (t1, s1) = laser_local(spec, 1.0);
		assert!(s0.y < s1.y);
		assert!((s1.y - 10.0).abs() < 1.0e-4);
		assert!(t1.y > t0.y);
	}

	#[test]
	fn bolt_is_not_gravity_bullet_is() {
		assert_eq!(Weapon::bolt().load.label(), "bolt");
		assert_eq!(Weapon::bullet().load.label(), "bullet");
		assert_eq!(Weapon::laser().load.label(), "laser");
	}

	#[test]
	fn through_length_air_and_solid() {
		assert_eq!(through_length(1.0, false, false, None, None), 0.0);
		assert_eq!(through_length(1.0, true, true, None, None), 1.0);
		assert_eq!(through_length(0.0, true, true, None, None), 0.0);
	}

	#[test]
	fn through_length_enter_and_thin_wall() {
		assert!((through_length(1.0, false, true, Some(0.3), None) - 0.7).abs() < 1e-5);
		assert!((through_length(1.0, false, false, Some(0.4), Some(0.4)) - 0.2).abs() < 1e-5);
		assert!((through_length(1.0, true, false, Some(0.4), Some(0.6)) - 0.4).abs() < 1e-5);
	}

	#[test]
	fn flight_exhausts_on_any_budget() {
		let mut flight = Flight::spawn(Vec3::ZERO, 10.0, 1.0, 2.0);
		assert!(!flight.exhausted());
		flight.path = 10.1;
		assert!(flight.exhausted());
		flight.path = 0.0;
		flight.through = 1.1;
		assert!(flight.exhausted());
		flight.through = 0.0;
		flight.age = 2.1;
		assert!(flight.exhausted());
	}
}
