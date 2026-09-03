//! Firearms spawn [`::projectiles`] from the receiver `barrel` bone.
//!
//! Lasers are visuals parented to the barrel (not a [`::projectiles::Flight`]).
//! They grow to the first bore hit, then retract to the muzzle when the trigger
//! is released.

use ::projectiles::{
	spawn_flight, tick_flights, BoltSpec, BulletSpec, ProjectileContact, ProjectileSource,
	ProjectilesPlugin,
};
use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::ecs::query::Has;
use bevy::prelude::*;
use damage::{DamageSystems, Hit, HitPayload};
use firearms_components::{BoneMap, FirearmHostSystems, FirearmMembers, FirearmRoot, RigRoot};
use lod_avian::PhysicsInteractionLayer;

use crate::cadence::{trigger_allows_fire, FireControl, WeaponFired, WeaponRecoil};
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

/// This [`Weapon`] only fires while [`WeaponTrigger`] is set.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct FireOnTrigger;

/// Analog/digital fire for this gun. Item-user crates write this; do not use a world resource.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct WeaponTrigger(pub bool);

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
pub struct LaserBeam {
	pub spec: LaserSpec,
	pub age: f32,
	pub retracting: bool,
}

/// Avian flights + fire / despawn / laser grow. Hosts still add [`crate::FirearmHostsPlugin`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FirearmWeaponSystems {
	Fire,
}

type ArmedWeaponQuery<'w, 's> = Query<
	'w,
	's,
	(
		Entity,
		&'static FirearmMembers,
		&'static mut Weapon,
		Has<FireOnTrigger>,
		Option<&'static WeaponTrigger>,
		Option<&'static ProjectileSource>,
		Option<&'static mut FireControl>,
		Option<&'static HitPayload>,
		Option<&'static WeaponRecoil>,
	),
	With<FirearmRoot>,
>;

pub struct FirearmWeaponsPlugin;

impl Plugin for FirearmWeaponsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ProjectilesPlugin>() {
			app.add_plugins(ProjectilesPlugin);
		}
		if !app.is_plugin_added::<damage::DamagePlugin>() {
			app.add_plugins(damage::DamagePlugin);
		}
		if !app.is_plugin_added::<bevy_hanabi::HanabiPlugin>() {
			app.add_plugins(bevy_hanabi::HanabiPlugin);
		}
		app.init_resource::<WeaponsArmed>()
			.add_message::<WeaponFired>()
			.add_systems(Startup, setup_impact_effects)
			.add_systems(
				PostUpdate,
				(
					fire_weapons.in_set(FirearmWeaponSystems::Fire),
					tick_lasers.after(FirearmWeaponSystems::Fire),
					tick_laser_hits
						.in_set(DamageSystems::Collect)
						.after(FirearmWeaponSystems::Fire),
					spawn_impacts_from_contacts,
					tick_impact_bursts,
				)
					.after(TransformSystems::Propagate)
					.after(FirearmHostSystems::Pose)
					.after(tick_flights),
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

fn spawn_laser(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	barrel: Entity,
	spec: LaserSpec,
	source: Option<ProjectileSource>,
) -> Entity {
	let (translation, scale) = laser_local(spec, 0.0, spec.max_length);
	let mut entity = commands.spawn((
		Name::new("laser"),
		Transform { translation, rotation: Quat::IDENTITY, scale },
		Visibility::default(),
		Mesh3d(meshes.add(Mesh::from(Cylinder::new(1.0, 1.0)))),
		MeshMaterial3d(materials.add(glow_material(spec.color))),
		ChildOf(barrel),
		LaserBeam { spec, age: 0.0, retracting: false },
	));
	if let Some(source) = source {
		entity.insert(source);
	}
	entity.id()
}

fn laser_local(spec: LaserSpec, age: f32, range: f32) -> (Vec3, Vec3) {
	let t = if spec.max_time > 1e-8 { (age / spec.max_time).clamp(0.0, 1.0) } else { 1.0 };
	let len = (spec.max_length * t).min(range).max(0.02);
	let translation = Vec3::Y * (BARREL_REST_LENGTH + len * 0.5);
	let scale = Vec3::new(spec.radius, len, spec.radius);
	(translation, scale)
}

fn apply_laser_pose(transform: &mut Transform, spec: LaserSpec, age: f32, range: f32) {
	let (translation, scale) = laser_local(spec, age, range);
	transform.translation = translation;
	transform.scale = scale;
}

fn laser_bore_hit(
	spatial: &SpatialQuery,
	muzzle: Vec3,
	dir: Vec3,
	max_length: f32,
	source: Option<Entity>,
) -> Option<(Entity, f32)> {
	let direction = Dir3::new(dir).ok()?;
	if max_length <= 1e-4 {
		return None;
	}
	let mut filter = SpatialQueryFilter::from_mask([
		PhysicsInteractionLayer::Fixed,
		PhysicsInteractionLayer::Animated,
	]);
	if let Some(source) = source {
		filter = filter.with_excluded_entities([source]);
	}
	let hit = spatial.cast_ray(muzzle, direction, max_length, true, &filter)?;
	if source == Some(hit.entity) {
		return None;
	}
	Some((hit.entity, hit.distance))
}

#[allow(clippy::too_many_arguments)]
pub fn fire_weapons(
	mut commands: Commands,
	time: Res<Time>,
	armed: Res<WeaponsArmed>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut weapons: ArmedWeaponQuery,
	maps: Query<&BoneMap, With<RigRoot>>,
	globals: Query<&GlobalTransform>,
	mut lasers: Query<&mut LaserBeam>,
	mut fired: MessageWriter<WeaponFired>,
) {
	if !armed.0 {
		return;
	}
	let dt = time.delta_secs();
	for (_root, members, mut weapon, manual, trigger, source, mut control, payload, recoil) in
		&mut weapons
	{
		let held = trigger.is_some_and(|trigger| trigger.0);
		let allowed = trigger_allows_fire(control.as_deref_mut(), manual, held);
		let Some((barrel, global)) = barrel_global(members, &maps, &globals) else {
			continue;
		};
		match weapon.load {
			ProjectileLoad::Laser(spec) => {
				let live = weapon.laser.filter(|entity| lasers.get(*entity).is_ok());
				weapon.laser = live;
				if manual && !held {
					if let Some(entity) = live {
						if let Ok(mut beam) = lasers.get_mut(entity) {
							beam.retracting = true;
						}
					}
					continue;
				}
				if let Some(entity) = live {
					if let Ok(mut beam) = lasers.get_mut(entity) {
						beam.retracting = false;
					}
				} else {
					weapon.laser = Some(spawn_laser(
						&mut commands,
						&mut meshes,
						&mut materials,
						barrel,
						spec,
						source.copied(),
					));
				}
			}
			ProjectileLoad::Bolt(spec) => {
				if !allowed {
					weapon.cooldown -= dt;
					continue;
				}
				if !try_fire_ballistic(
					&mut commands,
					&mut meshes,
					&mut materials,
					&mut weapon,
					global,
					spec,
					0.0,
					dt,
					source,
					payload,
					recoil,
					control.as_deref_mut(),
					&mut fired,
				) {
					continue;
				}
			}
			ProjectileLoad::Bullet(spec) => {
				if !allowed {
					weapon.cooldown -= dt;
					continue;
				}
				if !try_fire_ballistic(
					&mut commands,
					&mut meshes,
					&mut materials,
					&mut weapon,
					global,
					spec,
					1.0,
					dt,
					source,
					payload,
					recoil,
					control.as_deref_mut(),
					&mut fired,
				) {
					continue;
				}
			}
		}
	}
}

#[allow(clippy::too_many_arguments)]
fn try_fire_ballistic(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	weapon: &mut Weapon,
	global: &GlobalTransform,
	spec: impl IntoBallistic,
	gravity: f32,
	dt: f32,
	source: Option<&ProjectileSource>,
	payload: Option<&HitPayload>,
	recoil: Option<&WeaponRecoil>,
	control: Option<&mut FireControl>,
	fired: &mut MessageWriter<WeaponFired>,
) -> bool {
	weapon.cooldown -= dt;
	if weapon.cooldown > 0.0 {
		return false;
	}
	weapon.cooldown = weapon.interval;
	let (length, radius, speed, max_range, penetration, max_age, color) = spec.ballistic();
	let (muzzle, dir) = muzzle_world(global);
	let projectile = spawn_flight(
		commands,
		meshes,
		materials,
		muzzle,
		dir,
		length,
		radius,
		speed,
		max_range,
		penetration,
		max_age,
		color,
		gravity,
	);
	if let Some(payload) = payload {
		commands.entity(projectile).insert(*payload);
	}
	if let Some(source) = source {
		commands.entity(projectile).insert(*source);
	}
	if let Some(control) = control {
		control.note_shot();
	}
	let kick = recoil.map(|recoil| recoil.0).unwrap_or(0.0);
	if let Some(source) = source {
		fired.write(WeaponFired { shooter: source.0, recoil: kick });
	}
	true
}

trait IntoBallistic {
	fn ballistic(self) -> (f32, f32, f32, f32, f32, f32, Color);
}

impl IntoBallistic for BoltSpec {
	fn ballistic(self) -> (f32, f32, f32, f32, f32, f32, Color) {
		(
			self.length,
			self.radius,
			self.speed,
			self.max_range,
			self.penetration,
			self.max_age,
			self.color,
		)
	}
}

impl IntoBallistic for BulletSpec {
	fn ballistic(self) -> (f32, f32, f32, f32, f32, f32, Color) {
		(
			self.length,
			self.radius,
			self.speed,
			self.max_range,
			self.penetration,
			self.max_age,
			self.color,
		)
	}
}

type LaserWeaponQuery<'w, 's> = Query<
	'w,
	's,
	(
		&'static FirearmMembers,
		&'static mut Weapon,
		Has<FireOnTrigger>,
		Option<&'static WeaponTrigger>,
		Option<&'static ProjectileSource>,
		Option<&'static HitPayload>,
	),
	With<FirearmRoot>,
>;

/// Floor so a catalog laser with interval 0 does not apply damage every frame.
const LASER_HIT_INTERVAL: f32 = 0.15;

#[allow(clippy::too_many_arguments)]
fn tick_laser_hits(
	time: Res<Time>,
	armed: Res<WeaponsArmed>,
	spatial: SpatialQuery,
	mut weapons: LaserWeaponQuery,
	maps: Query<&BoneMap, With<RigRoot>>,
	globals: Query<&GlobalTransform>,
	mut hits: MessageWriter<Hit>,
	mut fired: MessageWriter<WeaponFired>,
) {
	if !armed.0 {
		return;
	}
	let dt = time.delta_secs();
	for (members, mut weapon, manual, trigger, source, payload) in &mut weapons {
		let ProjectileLoad::Laser(spec) = weapon.load else {
			continue;
		};
		let Some(payload) = payload else {
			continue;
		};
		let held = trigger.is_some_and(|trigger| trigger.0);
		if manual && !held {
			continue;
		}
		if weapon.laser.is_none() {
			continue;
		}
		weapon.cooldown -= dt;
		if weapon.cooldown > 0.0 {
			continue;
		}
		weapon.cooldown = weapon.interval.max(LASER_HIT_INTERVAL);
		if let Some(source) = source {
			fired.write(WeaponFired { shooter: source.0, recoil: 0.0 });
		}
		let Some((_, global)) = barrel_global(members, &maps, &globals) else {
			continue;
		};
		let (muzzle, dir) = muzzle_world(global);
		let Some((target, distance)) =
			laser_bore_hit(&spatial, muzzle, dir, spec.max_length, source.map(|source| source.0))
		else {
			continue;
		};
		hits.write(Hit {
			target,
			source: source.map(|source| source.0),
			amount: payload.amount,
			point: muzzle + dir * distance,
		});
	}
}

pub fn tick_lasers(
	time: Res<Time>,
	armed: Res<WeaponsArmed>,
	spatial: SpatialQuery,
	mut commands: Commands,
	globals: Query<&GlobalTransform>,
	mut lasers: Query<(
		Entity,
		&mut LaserBeam,
		&mut Transform,
		&ChildOf,
		Option<&ProjectileSource>,
	)>,
) {
	if !armed.0 {
		return;
	}
	let dt = time.delta_secs();
	for (entity, mut beam, mut transform, child_of, source) in &mut lasers {
		if beam.retracting {
			beam.age -= dt;
			if beam.age <= 0.0 {
				commands.entity(entity).try_despawn();
				continue;
			}
		} else {
			beam.age = (beam.age + dt).min(beam.spec.max_time);
		}
		let range = globals
			.get(child_of.parent())
			.ok()
			.and_then(|global| {
				let (muzzle, dir) = muzzle_world(global);
				laser_bore_hit(
					&spatial,
					muzzle,
					dir,
					beam.spec.max_length,
					source.map(|source| source.0),
				)
				.map(|(_, distance)| distance)
			})
			.unwrap_or(beam.spec.max_length);
		apply_laser_pose(&mut transform, beam.spec, beam.age, range);
	}
}

fn spawn_impacts_from_contacts(
	mut contacts: MessageReader<ProjectileContact>,
	effects: Option<Res<ImpactEffects>>,
	mut commands: Commands,
) {
	let Some(effects) = effects else {
		for _ in contacts.read() {}
		return;
	};
	for contact in contacts.read() {
		spawn_impact(&mut commands, &effects, contact.point, contact.normal);
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
		assert!((muzzle - (Vec3::new(1.0, 2.0, 3.0) + Vec3::X)).length() < 1e-4);
	}

	#[test]
	fn laser_grows_from_barrel_tail() {
		let spec =
			LaserSpec { max_length: 10.0, max_time: 1.0, radius: 0.1, ..LaserSpec::default() };
		let (t0, s0) = laser_local(spec, 0.0, spec.max_length);
		let (t1, s1) = laser_local(spec, 1.0, spec.max_length);
		assert!(s0.y < s1.y);
		assert!((s1.y - 10.0).abs() < 1.0e-4);
		assert!(t1.y > t0.y);
	}

	#[test]
	fn laser_local_stops_at_range() {
		let spec =
			LaserSpec { max_length: 10.0, max_time: 1.0, radius: 0.1, ..LaserSpec::default() };
		let (_t, scale) = laser_local(spec, 1.0, 4.0);
		assert!((scale.y - 4.0).abs() < 1e-4);
	}

	#[test]
	fn laser_local_retracts_with_age() {
		let spec =
			LaserSpec { max_length: 10.0, max_time: 1.0, radius: 0.1, ..LaserSpec::default() };
		let (_t_full, s_full) = laser_local(spec, 1.0, spec.max_length);
		let (_t_half, s_half) = laser_local(spec, 0.5, spec.max_length);
		assert!(s_half.y < s_full.y);
		assert!((s_half.y - 5.0).abs() < 1e-4);
	}

	#[test]
	fn bolt_is_not_gravity_bullet_is() {
		assert_eq!(Weapon::bolt().load.label(), "bolt");
		assert_eq!(Weapon::bullet().load.label(), "bullet");
		assert_eq!(Weapon::laser().load.label(), "laser");
	}
}
