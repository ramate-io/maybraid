//! Firearms spawn [`::projectiles`] from the receiver `barrel` bone.
//!
//! Lasers are visuals parented to the barrel (not a [`::projectiles::Flight`]).
//! They grow to the first bore hit and despawn when the trigger is released.
//! Every shot leaves a white flame cone on that bone, with a small ember burst at the cone's tip.
//! A laser keeps a smaller cone and a thin ember jet while the beam is on.

use ::projectiles::{
	spawn_flight, tick_flights, BoltSpec, BulletSpec, ProjectileContact, ProjectileSource,
	ProjectileVisualCache, ProjectilesPlugin,
};

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::ecs::query::Has;
use bevy::light::NotShadowCaster;
use bevy::mesh::ConeAnchor;
use bevy::prelude::*;
use bevy_hanabi::prelude::{
	Attribute, ColorBlendMask, ColorBlendMode, ColorOverLifetimeModifier, EffectAsset,
	EffectMaterial, ExprWriter, ImageSampleMapping, LinearDragModifier, OrientMode, OrientModifier,
	ParticleEffect, ParticleTextureModifier, SetAttributeModifier, SetPositionSphereModifier,
	SetVelocitySphereModifier, ShapeDimension, SimulationSpace, SizeOverLifetimeModifier,
	SpawnerSettings,
};
use bevy_hanabi::Gradient;
use damage::{DamageSystems, Hit, HitPayload};
use firearms_components::{BoneMap, FirearmHostSystems, FirearmMembers, FirearmRoot, RigRoot};
use lod_avian::PhysicsInteractionLayer;

use crate::cadence::{trigger_allows_fire, FireControl, WeaponFired, WeaponRecoil};
use crate::impact::{
	puff_mask, setup_impact_effects, spawn_impact, tick_impact_bursts, ImpactEffects,
};
use crate::muzzle_flame::{
	init_muzzle_flame_caches, muzzle_flame_ref, resolve_muzzle_flame, MuzzleFlameMaterial,
	MuzzleFlameMaterialPlugin, MuzzleFlameMaterialRefCache,
};

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
		Self { radius: 0.07, max_length: 22.0, max_time: 0.7, color: Color::srgb(1.0, 0.18, 0.22) }
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
		init_muzzle_flame_caches(app);
		if !app.is_plugin_added::<MuzzleFlameMaterialPlugin>() {
			app.add_plugins(MuzzleFlameMaterialPlugin);
		}
		app.init_resource::<WeaponsArmed>()
			.add_message::<WeaponFired>()
			.add_systems(Startup, (setup_impact_effects, setup_muzzle_flash))
			.add_systems(
				PostUpdate,
				(
					fire_weapons.in_set(FirearmWeaponSystems::Fire),
					// Hanabi inserts `EffectSpawner` in this set. Despawn after that insert
					// is applied, or the insert lands on an entity we already deleted.
					tick_muzzle_flashes
						.after(FirearmWeaponSystems::Fire)
						.after(bevy_hanabi::EffectSystems::TickSpawners),
					tick_lasers.after(FirearmWeaponSystems::Fire),
					tick_laser_hits
						.in_set(DamageSystems::Collect)
						.after(FirearmWeaponSystems::Fire),
					spawn_impacts_from_contacts,
					tick_impact_bursts.after(bevy_hanabi::EffectSystems::TickSpawners),
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

/// How long the cone takes to shrink away. The ember entity outlives the particles.
const CONE_LIFE: f32 = 0.08;
const MUZZLE_FLASH_LIFE: f32 = 0.28;
const MUZZLE_FLASH_RADIUS: f32 = 0.22;
const MUZZLE_FLASH_LENGTH: f32 = 0.55;
/// Embers spawn here in the emitter's local space. The emitter sits on the cone tip.
const EMBER_SPAWN_CENTER: Vec3 = Vec3::ZERO;
/// Behind the tip on barrel -Y, so the cheap burst travels on down the bore (+Y).
const FLAME_VELOCITY_CENTER: Vec3 = Vec3::new(0.0, -0.2, 0.0);
/// One-shot plume. Long life and light drag so the burst clears the cone.
const EMBER_SPEED: (f32, f32) = (7.0, 14.0);
const EMBER_LIFE: (f32, f32) = (0.10, 0.20);
const EMBER_DRAG: f32 = 2.0;

/// Cone plus embers. A laser holds both until the beam is gone.
#[derive(Component)]
struct MuzzleFlash {
	age: f32,
	life: f32,
	sustain_laser: Option<Entity>,
}

/// The fading cone. Kept off the flash root so ember sprites are not scaled with it.
#[derive(Component)]
struct MuzzleFlashCone;

/// Shared cone mesh, the white-flame material, and the small ember effects.
#[derive(Resource)]
pub(crate) struct MuzzleFlashEffects {
	burst: Handle<EffectAsset>,
	jet: Handle<EffectAsset>,
	puff: Handle<Image>,
	cone: Handle<Mesh>,
	flame: Handle<MuzzleFlameMaterial>,
}

fn setup_muzzle_flash(
	mut commands: Commands,
	mut effects: ResMut<Assets<EffectAsset>>,
	mut images: ResMut<Assets<Image>>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut flame_materials: ResMut<Assets<MuzzleFlameMaterial>>,
	mut flame_cache: ResMut<MuzzleFlameMaterialRefCache>,
) {
	let puff = images.add(puff_mask());
	let flame = resolve_muzzle_flame(&mut flame_materials, &mut flame_cache, &muzzle_flame_ref());
	commands.insert_resource(MuzzleFlashEffects {
		burst: effects.add(flame_effect(FlameKind::Burst)),
		jet: effects.add(flame_effect(FlameKind::Jet)),
		puff,
		cone: meshes.add(muzzle_flash_mesh()),
		flame,
	});
}

fn muzzle_flash_mesh() -> Mesh {
	Cone::new(1.0, 1.0).mesh().resolution(12).anchor(ConeAnchor::Base).build()
}

/// Cone scale in flash-local space. The base stays at the origin; the tip points down the bore.
fn cone_scale(age: f32, sustain: bool) -> Vec3 {
	let t = if CONE_LIFE > 1e-6 { (age / CONE_LIFE).clamp(0.0, 1.0) } else { 1.0 };
	let fade = if sustain { 1.0 - 0.45 * t } else { 1.0 - t };
	Vec3::new(MUZZLE_FLASH_RADIUS * fade, MUZZLE_FLASH_LENGTH * fade, MUZZLE_FLASH_RADIUS * fade)
}

#[derive(Clone, Copy)]
enum FlameKind {
	Burst,
	Jet,
}

fn flame_effect(kind: FlameKind) -> EffectAsset {
	let (name, capacity, spawner, spawn_radius, speed, life, size, drag) = match kind {
		FlameKind::Burst => (
			"muzzle-burst",
			16_u32,
			SpawnerSettings::once(12.0.into()),
			0.06,
			EMBER_SPEED,
			EMBER_LIFE,
			(0.045, 0.10, 0.16),
			EMBER_DRAG,
		),
		FlameKind::Jet => (
			"muzzle-jet",
			12,
			SpawnerSettings::rate(16.0.into()),
			0.05,
			(5.0, 10.0),
			(0.08, 0.16),
			(0.035, 0.07, 0.11),
			2.2,
		),
	};
	let writer = ExprWriter::new();
	let init_pos = SetPositionSphereModifier {
		center: writer.lit(EMBER_SPAWN_CENTER).expr(),
		radius: writer.lit(spawn_radius).expr(),
		dimension: ShapeDimension::Volume,
	};
	let speed = writer.lit(speed.0).uniform(writer.lit(speed.1));
	let init_vel = SetVelocitySphereModifier {
		center: writer.lit(FLAME_VELOCITY_CENTER).expr(),
		speed: speed.expr(),
	};
	let init_age = SetAttributeModifier::new(Attribute::AGE, writer.lit(0.).expr());
	let init_lifetime = SetAttributeModifier::new(
		Attribute::LIFETIME,
		writer.lit(life.0).uniform(writer.lit(life.1)).expr(),
	);
	let update_drag = LinearDragModifier::new(writer.lit(drag).expr());
	let texture_slot = writer.lit(0u32).expr();

	let mut color = Gradient::new();
	color.add_key(0.0, Vec4::new(4.0, 3.2, 1.6, 1.0));
	color.add_key(0.45, Vec4::new(2.0, 0.55, 0.08, 0.7));
	color.add_key(1.0, Vec4::new(0.4, 0.05, 0.01, 0.0));
	let mut size_gradient = Gradient::new();
	size_gradient.add_key(0.0, Vec3::splat(size.0));
	size_gradient.add_key(0.35, Vec3::splat(size.1));
	size_gradient.add_key(1.0, Vec3::splat(size.2));

	let mut module = writer.finish();
	module.add_texture_slot("puff");

	EffectAsset::new(capacity, spawner, module)
		.with_name(name)
		.with_simulation_space(SimulationSpace::Local)
		.with_alpha_mode(bevy_hanabi::AlphaMode::Add)
		.init(init_pos)
		.init(init_vel)
		.init(init_age)
		.init(init_lifetime)
		.update(update_drag)
		.render(ParticleTextureModifier {
			texture_slot,
			sample_mapping: ImageSampleMapping::ModulateOpacityFromR,
		})
		.render(ColorOverLifetimeModifier {
			gradient: color,
			blend: ColorBlendMode::Overwrite,
			mask: ColorBlendMask::RGBA,
		})
		.render(SizeOverLifetimeModifier { gradient: size_gradient, screen_space_size: false })
		.render(OrientModifier::new(OrientMode::FaceCameraPosition))
}

fn muzzle_flash_translation() -> Vec3 {
	Vec3::Y * BARREL_REST_LENGTH
}

/// Emitter at the cone tip. The unit cone's tip is local +Y, scaled by the flash length.
fn ember_translation() -> Vec3 {
	Vec3::Y * MUZZLE_FLASH_LENGTH
}

fn spawn_muzzle_flash(
	commands: &mut Commands,
	effects: &mut MuzzleFlashEffects,
	barrel: Entity,
	sustain_laser: Option<Entity>,
) {
	let asset = if sustain_laser.is_some() { effects.jet.clone() } else { effects.burst.clone() };
	let puff = effects.puff.clone();
	let mesh = effects.cone.clone();
	let material = effects.flame.clone();
	let sustain = sustain_laser.is_some();
	let flash = commands
		.spawn((
			Name::new("muzzle-flash"),
			ChildOf(barrel),
			Transform::from_translation(muzzle_flash_translation()),
			Visibility::Visible,
			MuzzleFlash { age: 0.0, life: MUZZLE_FLASH_LIFE, sustain_laser },
		))
		.id();
	commands.spawn((
		Name::new("muzzle-flash-cone"),
		ChildOf(flash),
		Transform::from_scale(cone_scale(0.0, sustain)),
		Visibility::Inherited,
		Mesh3d(mesh),
		// Resolved at spawn. A `MaterialRefRoot` would be fulfilled a frame later and
		// can insert onto this cone after the flash has already been despawned.
		MeshMaterial3d(material),
		NotShadowCaster,
		MuzzleFlashCone,
	));
	commands.spawn((
		Name::new("muzzle-flash-embers"),
		ChildOf(flash),
		Transform::from_translation(ember_translation()),
		Visibility::Inherited,
		ParticleEffect::new(asset),
		EffectMaterial { images: vec![puff] },
	));
}

fn tick_muzzle_flashes(
	time: Res<Time>,
	mut commands: Commands,
	lasers: Query<(), With<LaserBeam>>,
	mut flashes: Query<(Entity, &mut MuzzleFlash, &Children)>,
	mut cones: Query<&mut Transform, With<MuzzleFlashCone>>,
) {
	let dt = time.delta_secs();
	for (entity, mut flash, children) in &mut flashes {
		flash.age += dt;
		let held = flash.sustain_laser.is_some_and(|laser| lasers.get(laser).is_ok());
		if flash.sustain_laser.is_some() && !held {
			commands.entity(entity).try_despawn();
			continue;
		}
		if flash.sustain_laser.is_none() && flash.age >= flash.life {
			commands.entity(entity).try_despawn();
			continue;
		}
		let scale = cone_scale(flash.age, held);
		for child in children.iter() {
			if let Ok(mut transform) = cones.get_mut(child) {
				transform.scale = scale;
			}
		}
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
		LaserBeam { spec, age: 0.0 },
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
pub(crate) fn fire_weapons(
	mut commands: Commands,
	time: Res<Time>,
	armed: Res<WeaponsArmed>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut projectile_visuals: ResMut<ProjectileVisualCache>,
	mut weapons: ArmedWeaponQuery,
	maps: Query<&BoneMap, With<RigRoot>>,
	globals: Query<&GlobalTransform>,
	lasers: Query<&LaserBeam>,
	mut flashes: ResMut<MuzzleFlashEffects>,
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
				if manual && !held {
					if let Some(entity) = live {
						commands.entity(entity).try_despawn();
					}
					weapon.laser = None;
					continue;
				}
				if live.is_none() {
					let laser = spawn_laser(
						&mut commands,
						&mut meshes,
						&mut materials,
						barrel,
						spec,
						source.copied(),
					);
					spawn_muzzle_flash(&mut commands, &mut flashes, barrel, Some(laser));
					weapon.laser = Some(laser);
				} else {
					weapon.laser = live;
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
					&mut projectile_visuals,
					&mut flashes,
					&mut weapon,
					barrel,
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
					&mut projectile_visuals,
					&mut flashes,
					&mut weapon,
					barrel,
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
	projectile_visuals: &mut ProjectileVisualCache,
	flashes: &mut MuzzleFlashEffects,
	weapon: &mut Weapon,
	barrel: Entity,
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
	spawn_muzzle_flash(commands, flashes, barrel, None);
	let projectile = spawn_flight(
		commands,
		meshes,
		materials,
		projectile_visuals,
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
	globals: Query<&GlobalTransform>,
	mut lasers: Query<(&mut LaserBeam, &mut Transform, &ChildOf, Option<&ProjectileSource>)>,
) {
	if !armed.0 {
		return;
	}
	let dt = time.delta_secs();
	for (mut beam, mut transform, child_of, source) in &mut lasers {
		beam.age = (beam.age + dt).min(beam.spec.max_time);
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
	fn muzzle_flash_sits_on_the_barrel_tail() {
		let origin = muzzle_flash_translation();
		assert!((origin.y - BARREL_REST_LENGTH).abs() < 1e-4, "{}", origin.y);
		assert_eq!(origin.x, 0.0);
		assert_eq!(origin.z, 0.0);
		let tip = ember_translation();
		assert!((tip.y - cone_scale(0.0, false).y).abs() < 1e-4, "embers leave the cone tip");
		assert_eq!(EMBER_SPAWN_CENTER, Vec3::ZERO, "the burst is centered on that tip");
	}

	#[test]
	fn cone_reaches_down_the_bore_then_fades() {
		let full = cone_scale(0.0, false);
		assert!(full.y > full.x, "cone should reach down the bore");
		let gone = cone_scale(CONE_LIFE, false);
		assert!(gone.x < 1e-4, "one-shot cone ends at zero");
		let held = cone_scale(CONE_LIFE * 4.0, true);
		assert!(held.x > MUZZLE_FLASH_RADIUS * 0.5, "laser keeps a muzzle cone");
	}

	#[test]
	fn flame_burst_travels_down_the_bore() {
		assert!(FLAME_VELOCITY_CENTER.y < 0.0, "bias sits behind the tip");
		assert_eq!(FLAME_VELOCITY_CENTER.x, 0.0);
		assert_eq!(FLAME_VELOCITY_CENTER.z, 0.0);
		let speed = (EMBER_SPEED.0 + EMBER_SPEED.1) * 0.5;
		let life = (EMBER_LIFE.0 + EMBER_LIFE.1) * 0.5;
		let reach = speed / EMBER_DRAG * (1.0 - (-EMBER_DRAG * life).exp());
		assert!(reach > MUZZLE_FLASH_LENGTH, "embers travel past the cone, reach {reach}");
		assert!(MUZZLE_FLASH_LIFE > EMBER_LIFE.1, "the flash outlives the plume");
	}

	#[test]
	fn bolt_is_not_gravity_bullet_is() {
		assert_eq!(Weapon::bolt().load.label(), "bolt");
		assert_eq!(Weapon::bullet().load.label(), "bullet");
		assert_eq!(Weapon::laser().load.label(), "laser");
	}
}
