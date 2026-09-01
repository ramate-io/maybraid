//! Emissive blaster projectiles spawned from the receiver `barrel` bone.

use avian3d::prelude::*;
use avian3d::schedule::PhysicsSchedulePlugin;
use bevy::prelude::*;

use firearms_components::{BoneMap, FirearmHostSystems, FirearmMembers, FirearmRoot, RigRoot};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoltSpec {
	pub length: f32,
	pub radius: f32,
	pub speed: f32,
	pub max_range: f32,
	pub color: Color,
}

impl Default for BoltSpec {
	fn default() -> Self {
		Self {
			length: 0.55,
			radius: 0.055,
			speed: 42.0,
			max_range: 36.0,
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
	pub color: Color,
}

impl Default for BulletSpec {
	fn default() -> Self {
		Self {
			length: 0.32,
			radius: 0.04,
			speed: 28.0,
			max_range: 42.0,
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

#[derive(Component, Debug, Clone, Copy)]
pub struct Flight {
	pub origin: Vec3,
	pub max_range: f32,
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
		app.init_resource::<WeaponsArmed>().add_systems(
			PostUpdate,
			(fire_weapons, tick_lasers, despawn_spent_flights)
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
	color: Color,
	gravity: f32,
) {
	let transform = capsule_along_y(direction, muzzle, length, radius);
	commands.spawn((
		Name::new("projectile"),
		transform,
		Visibility::default(),
		Mesh3d(meshes.add(Capsule3d::new(radius, length))),
		MeshMaterial3d(materials.add(glow_material(color))),
		RigidBody::Dynamic,
		Collider::capsule(radius, length),
		LockedAxes::ROTATION_LOCKED,
		LinearVelocity(direction * speed),
		GravityScale(gravity),
		Restitution::ZERO,
		Flight { origin: muzzle, max_range },
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
			RigidBody::Kinematic,
			Collider::capsule(spec.radius, 0.05),
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

fn apply_laser_pose(
	transform: &mut Transform,
	collider: Option<&mut Collider>,
	spec: LaserSpec,
	age: f32,
) {
	let (translation, scale) = laser_local(spec, age);
	transform.translation = translation;
	transform.scale = scale;
	if let Some(collider) = collider {
		*collider = Collider::capsule(spec.radius, scale.y.max(0.05));
	}
}

pub fn fire_weapons(
	mut commands: Commands,
	time: Res<Time>,
	armed: Res<WeaponsArmed>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut weapons: Query<(Entity, &FirearmMembers, &mut Weapon), With<FirearmRoot>>,
	maps: Query<&BoneMap, With<RigRoot>>,
	globals: Query<&GlobalTransform>,
	lasers: Query<&LaserBeam>,
) {
	if !armed.0 {
		return;
	}
	let dt = time.delta_secs();
	for (_root, members, mut weapon) in &mut weapons {
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
	mut lasers: Query<(&mut LaserBeam, &mut Transform, &mut Collider)>,
) {
	if !armed.0 {
		return;
	}
	let dt = time.delta_secs();
	for (mut beam, mut transform, mut collider) in &mut lasers {
		beam.age += dt;
		if beam.age >= beam.spec.max_time {
			beam.age = 0.0;
		}
		apply_laser_pose(&mut transform, Some(&mut collider), beam.spec, beam.age);
	}
}

pub fn despawn_spent_flights(
	mut commands: Commands,
	flights: Query<(Entity, &Transform, &Flight)>,
) {
	for (entity, transform, flight) in &flights {
		if transform.translation.distance(flight.origin) > flight.max_range {
			commands.entity(entity).try_despawn();
		}
	}
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
}
