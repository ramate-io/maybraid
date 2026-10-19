//! Short-lived fist / forearm hit sensor for an active jab.
//!
//! The volume is an Avian [`Sensor`] (query-only, [`projectile_layers`]) spawned
//! only while [`AttackPhase::Active`]. Hits are collected with `SpatialQuery`
//! the same way grenade blasts and skill-map bursts resolve, then written as
//! [`damage::Hit`]. [`AttackState::combo_step`] rotates for effort SFX only;
//! there is no combo-step damage scale.

use std::collections::{HashMap, HashSet};

use avian3d::prelude::*;
use bevy::prelude::*;
use characters::{CharacterHeading, CharacterRoot};
use damage::{Downed, Health, Hit};
use lod_avian::PhysicsInteractionLayer;

use crate::aim::JabAim;
use crate::attack::AttackPhase;
use crate::{FisticuffsUser, Side};

/// Placeholder jab damage. **Not final balance** — retune after playtest.
pub const PLACEHOLDER_JAB_DAMAGE: f32 = 12.0;

/// Placeholder fist / forearm capsule radius, meters. **Not final feel.**
pub const PLACEHOLDER_FIST_RADIUS: f32 = 0.12;

/// Placeholder forearm cylinder length, meters. **Not final feel.**
///
/// Avian `Collider::capsule(radius, length)` is the cylinder; total height is
/// `length + 2 * radius`.
pub const PLACEHOLDER_FOREARM_LENGTH: f32 = 0.28;

/// Placeholder reach from the body origin along the latched look, meters.
///
/// Matches the posed Active fist (~1.4 m) more closely than the look-point
/// `DEFAULT_JAB_TARGET.z`. **Not final feel.**
pub const PLACEHOLDER_FIST_REACH: f32 = 1.28;

/// Placeholder punching-hand height at level, meters. **Not final feel.**
const PLACEHOLDER_FIST_HEIGHT: f32 = 0.62;

/// Left-hand outboard offset in character +X. **Not final feel.**
const PLACEHOLDER_FIST_SIDE_LEFT: f32 = 0.16;

/// Right-hand outboard offset in character −X. **Not final feel.**
const PLACEHOLDER_FIST_SIDE_RIGHT: f32 = 0.10;

const SENSOR_NAME: &str = "fisticuffs-fist-sensor";
const DEBUG_COLOR: Color = Color::srgb(1.0, 0.42, 0.16);

/// Debug overlay for the live fist volume. World playground chrome turns this
/// on; the shipped game leaves it off. Insert `FisticuffsSensorDebug(true)` in
/// a capture host to draw the capsule.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FisticuffsSensorDebug(pub bool);

impl Default for FisticuffsSensorDebug {
	fn default() -> Self {
		Self(false)
	}
}

impl FisticuffsSensorDebug {
	pub fn from_debug_chrome(debug_chrome: bool) -> Self {
		Self(debug_chrome)
	}
}

/// Authoritative fist / forearm volume. Lives only while the owning jab is
/// [`AttackPhase::Active`].
#[derive(Component, Debug)]
pub struct FisticuffsHitSensor {
	pub attacker: Entity,
	pub side: Side,
	/// Health entities already injured by this strike (once-per-target).
	pub contacted: Vec<Entity>,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct FisticuffsHitSensorOf(pub Entity);

/// Character-space fist center for the punching hand.
pub fn fist_local(side: Side, aim: JabAim) -> Vec3 {
	let side_x = match side {
		Side::Left => PLACEHOLDER_FIST_SIDE_LEFT,
		Side::Right => -PLACEHOLDER_FIST_SIDE_RIGHT,
	};
	Vec3::new(side_x, PLACEHOLDER_FIST_HEIGHT, 0.0) + aim.direction() * PLACEHOLDER_FIST_REACH
}

/// World point of a character-space offset. `heading` is fight-forward (+Z).
pub fn character_to_world(origin: Vec3, heading: Vec3, local: Vec3) -> Vec3 {
	let forward = xz_forward(heading);
	// Facing −Z: character +X (left) is world +X = forward × Y.
	let left = forward.cross(Vec3::Y).normalize_or(Vec3::X);
	origin + left * local.x + Vec3::Y * local.y + forward * local.z
}

/// World rotation that stands an Avian Y-capsule along `local_dir`.
pub fn fist_world_rotation(heading: Vec3, local_dir: Vec3) -> Quat {
	let dir = character_to_world(Vec3::ZERO, heading, local_dir);
	let dir = if dir.length_squared() < 1e-8 { xz_forward(heading) } else { dir.normalize() };
	Quat::from_rotation_arc(Vec3::Y, dir)
}

pub fn fist_collider() -> Collider {
	Collider::capsule(PLACEHOLDER_FIST_RADIUS, PLACEHOLDER_FOREARM_LENGTH)
}

fn xz_forward(heading: Vec3) -> Vec3 {
	let planar = Vec3::new(heading.x, 0.0, heading.z);
	if planar.length_squared() < 1e-8 {
		-Vec3::Z
	} else {
		planar.normalize()
	}
}

fn heading_of(
	attacker: Entity,
	visuals: &Query<(&CharacterHeading, &ChildOf), With<CharacterRoot>>,
) -> Vec3 {
	visuals
		.iter()
		.find(|(_, child_of)| child_of.parent() == attacker)
		.map(|(heading, _)| heading.0)
		.unwrap_or(-Vec3::Z)
}

fn attacker_origin(global: Option<&GlobalTransform>, local: Option<&Transform>) -> Vec3 {
	global
		.map(GlobalTransform::translation)
		.or_else(|| local.map(|transform| transform.translation))
		.unwrap_or(Vec3::ZERO)
}

fn sensor_local_transform(heading: Vec3, side: Side, aim: JabAim) -> Transform {
	let local = fist_local(side, aim);
	Transform {
		translation: character_to_world(Vec3::ZERO, heading, local),
		rotation: fist_world_rotation(heading, aim.direction()),
		scale: Vec3::ONE,
	}
}

fn should_keep_sensor(
	attacker: Entity,
	user: &FisticuffsUser,
	downed: &Query<(), With<Downed>>,
	health: &Query<&Health>,
) -> bool {
	if user.state.phase() != Some(AttackPhase::Active) {
		return false;
	}
	if downed.contains(attacker) {
		return false;
	}
	if health.get(attacker).is_ok_and(|pool| pool.is_dead()) {
		return false;
	}
	true
}

fn try_despawn(commands: &mut Commands, entity: Entity) {
	if let Ok(mut entity) = commands.get_entity(entity) {
		entity.try_despawn();
	}
}

fn despawn_group(
	by_attacker: &mut HashMap<Entity, Vec<Entity>>,
	attacker: Entity,
	commands: &mut Commands,
) {
	if let Some(group) = by_attacker.remove(&attacker) {
		for sensor in group {
			try_despawn(commands, sensor);
		}
	}
}

fn spawn_sensor(commands: &mut Commands, attacker: Entity, side: Side, transform: Transform) {
	commands.spawn((
		Name::new(SENSOR_NAME),
		FisticuffsHitSensor { attacker, side, contacted: Vec::new() },
		FisticuffsHitSensorOf(attacker),
		ChildOf(attacker),
		transform,
		fist_collider(),
		Sensor,
		PhysicsInteractionLayer::projectile_layers(),
	));
}

/// Spawn, pose, or tear down the fist volume from the live attack phase.
///
/// A new strike gets a new sensor because the previous Active window already
/// tore the last one down. Do not key respawn on
/// [`crate::AttackState::just_entered_active`] — that flag stays set until the
/// next `tick` and would wipe once-per-target dedupe.
pub fn sync_fist_sensors(
	mut commands: Commands,
	users: Query<(Entity, &FisticuffsUser)>,
	visuals: Query<(&CharacterHeading, &ChildOf), With<CharacterRoot>>,
	sensors: Query<(Entity, &FisticuffsHitSensorOf)>,
	downed: Query<(), With<Downed>>,
	health: Query<&Health>,
	mut removed: RemovedComponents<FisticuffsUser>,
) {
	let mut by_attacker: HashMap<Entity, Vec<Entity>> = HashMap::new();
	for (sensor, of) in &sensors {
		by_attacker.entry(of.0).or_default().push(sensor);
	}

	for attacker in removed.read() {
		despawn_group(&mut by_attacker, attacker, &mut commands);
	}

	let mut wanted = HashSet::new();
	for (attacker, user) in &users {
		if !should_keep_sensor(attacker, user, &downed, &health) {
			continue;
		}
		let Some(side) = user.state.side() else {
			continue;
		};
		wanted.insert(attacker);
		let heading = heading_of(attacker, &visuals);
		let aim = user.state.aim().unwrap_or_default();
		let pose = sensor_local_transform(heading, side, aim);
		match by_attacker.remove(&attacker).and_then(|mut group| {
			let keep = group.pop();
			for extra in group {
				try_despawn(&mut commands, extra);
			}
			keep
		}) {
			Some(sensor) => {
				if let Ok(mut entity) = commands.get_entity(sensor) {
					entity.try_insert((pose, FisticuffsHitSensorOf(attacker)));
				}
			}
			None => {
				spawn_sensor(&mut commands, attacker, side, pose);
			}
		}
	}

	let existing: Vec<Entity> = by_attacker.keys().copied().collect();
	for attacker in existing {
		if !wanted.contains(&attacker) {
			despawn_group(&mut by_attacker, attacker, &mut commands);
		}
	}
	for extras in by_attacker.into_values() {
		for sensor in extras {
			try_despawn(&mut commands, sensor);
		}
	}
}

/// Resolve overlap against [`Health`] and write [`Hit`]. Self-hits are dropped
/// here and again in [`damage::apply_hits`] (`allow_self: false`). There is no
/// separate team filter on the damage seam — same rule as firearms / grenades.
pub fn resolve_fist_hits(
	spatial: SpatialQuery,
	users: Query<(Entity, &FisticuffsUser, Option<&GlobalTransform>, Option<&Transform>)>,
	visuals: Query<(&CharacterHeading, &ChildOf), With<CharacterRoot>>,
	transforms: Query<(Entity, &Transform, Option<&GlobalTransform>)>,
	health: Query<(), With<Health>>,
	parents: Query<&ChildOf>,
	mut sensors: Query<(Entity, &mut FisticuffsHitSensor)>,
	mut hits: MessageWriter<Hit>,
) {
	for (sensor, mut volume) in &mut sensors {
		let Ok((attacker, user, global, local)) = users.get(volume.attacker) else {
			continue;
		};
		let heading = heading_of(attacker, &visuals);
		let aim = user.state.aim().unwrap_or_default();
		let origin = attacker_origin(global, local);
		let point = character_to_world(origin, heading, fist_local(volume.side, aim));
		let rotation = fist_world_rotation(heading, aim.direction());
		let filter = SpatialQueryFilter::from_mask([
			PhysicsInteractionLayer::Fixed,
			PhysicsInteractionLayer::Animated,
		])
		.with_excluded_entities([sensor, volume.attacker]);
		let candidates = spatial.shape_intersections(&fist_collider(), point, rotation, &filter);
		for hit in collect_fist_hits(
			volume.attacker,
			PLACEHOLDER_JAB_DAMAGE,
			point,
			&candidates,
			&mut volume.contacted,
			&transforms,
			&health,
			&parents,
		) {
			hits.write(hit);
		}
	}
}

/// Remount, dedupe, and skip the attacker. Unit-tested without Avian overlap.
pub fn collect_fist_hits(
	attacker: Entity,
	amount: f32,
	fallback_point: Vec3,
	candidates: &[Entity],
	contacted: &mut Vec<Entity>,
	transforms: &Query<(Entity, &Transform, Option<&GlobalTransform>)>,
	health: &Query<(), With<Health>>,
	parents: &Query<&ChildOf>,
) -> Vec<Hit> {
	let mut out = Vec::new();
	for &collider in candidates {
		if collider == attacker {
			continue;
		}
		let parent = parents.get(collider).ok().map(|child| child.parent());
		let target = Health::entity_or_parent(
			collider,
			parent,
			health.contains(collider),
			parent.is_some_and(|parent| health.contains(parent)),
		);
		if target == attacker || !health.contains(target) || contacted.contains(&target) {
			continue;
		}
		let point = hit_point(target, collider, transforms).unwrap_or(fallback_point);
		contacted.push(target);
		out.push(Hit { target, source: Some(attacker), amount, point, allow_self: false });
	}
	out
}

fn hit_point(
	target: Entity,
	collider: Entity,
	transforms: &Query<(Entity, &Transform, Option<&GlobalTransform>)>,
) -> Option<Vec3> {
	for entity in [target, collider] {
		if let Ok((_, transform, global)) = transforms.get(entity) {
			return Some(global.map(GlobalTransform::translation).unwrap_or(transform.translation));
		}
	}
	None
}

pub fn draw_fist_sensors(
	debug: Res<FisticuffsSensorDebug>,
	mut gizmos: Gizmos,
	sensors: Query<&GlobalTransform, With<FisticuffsHitSensor>>,
) {
	if !debug.0 {
		return;
	}
	for transform in &sensors {
		let iso = Isometry3d::from_translation(transform.translation())
			* Isometry3d::from_rotation(transform.rotation());
		gizmos.primitive_3d(
			&Capsule3d::new(PLACEHOLDER_FIST_RADIUS, PLACEHOLDER_FOREARM_LENGTH),
			iso,
			DEBUG_COLOR,
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{AttackSpec, AttackTiming, JAB_STARTUP_END};
	use anyhow::{anyhow, Result};
	use bevy::ecs::system::RunSystemOnce;
	use character_animations::animations::DEFAULT_JAB_TARGET;
	use damage::{apply_hits, DamageApplied, Died};
	use player::Player;

	fn jab_dt(progress: f32) -> f32 {
		progress * AttackTiming::JAB.duration_secs
	}

	fn drive_to_active(user: &mut FisticuffsUser) {
		user.state.request(AttackSpec::jab(Side::Left));
		user.state.latch_aim(JabAim::level());
		user.state.tick(jab_dt(JAB_STARTUP_END) + 1e-4);
	}

	fn sensor_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TransformPlugin));
		app.add_systems(Update, sync_fist_sensors);
		app
	}

	fn sensor_count(world: &mut World) -> usize {
		world.query::<&FisticuffsHitSensor>().iter(world).count()
	}

	#[test]
	fn placeholder_damage_is_labelled_and_below_a_firearm_bolt() {
		assert!(PLACEHOLDER_JAB_DAMAGE > 0.0);
		assert!(PLACEHOLDER_JAB_DAMAGE < damage::DEFAULT_HIT);
		assert!(PLACEHOLDER_FIST_RADIUS > 0.05 && PLACEHOLDER_FIST_RADIUS < 0.25);
		assert!(PLACEHOLDER_FOREARM_LENGTH > 0.1 && PLACEHOLDER_FOREARM_LENGTH < 0.6);
	}

	#[test]
	fn fist_local_sits_on_the_punching_side_and_past_the_look_point() {
		let left = fist_local(Side::Left, JabAim::level());
		let right = fist_local(Side::Right, JabAim::level());
		assert!(left.x > 0.1, "left fist {left:?}");
		assert!(right.x < -0.05, "right fist {right:?}");
		assert!(left.z > DEFAULT_JAB_TARGET.z, "fist should reach past the look point");
		assert!(left.z > 1.1 && left.z < 1.7, "placeholder reach {left:?}");
	}

	#[test]
	fn character_to_world_maps_plus_z_onto_heading() {
		let origin = Vec3::new(2.0, 0.0, 4.0);
		let world = character_to_world(origin, -Vec3::Z, Vec3::new(0.16, 0.62, 1.28));
		assert!((world - Vec3::new(2.16, 0.62, 2.72)).length() < 1e-4);
		let left = character_to_world(origin, -Vec3::Z, Vec3::X);
		assert!((left - (origin + Vec3::X)).length() < 1e-4);
	}

	#[test]
	fn sensor_exists_only_during_active_frames() -> Result<()> {
		let mut app = sensor_app();
		let mut user = FisticuffsUser::default();
		user.state.request(AttackSpec::jab(Side::Left));
		user.state.latch_aim(JabAim::level());
		let player = app
			.world_mut()
			.spawn((user, Transform::default(), GlobalTransform::default()))
			.id();
		app.update();
		if sensor_count(app.world_mut()) != 0 {
			return Err(anyhow!("startup must not spawn a fist sensor"));
		}

		{
			let mut user =
				app.world_mut().get_mut::<FisticuffsUser>(player).ok_or(anyhow!("user"))?;
			user.state.tick(jab_dt(JAB_STARTUP_END) + 1e-4);
		}
		app.update();
		if sensor_count(app.world_mut()) != 1 {
			return Err(anyhow!("active window should spawn one sensor"));
		}
		if app.world().get::<FisticuffsUser>(player).ok_or(anyhow!("user"))?.state.phase()
			!= Some(AttackPhase::Active)
		{
			return Err(anyhow!("expected Active after the startup tick"));
		}

		{
			let mut user =
				app.world_mut().get_mut::<FisticuffsUser>(player).ok_or(anyhow!("user"))?;
			user.state.tick(jab_dt(0.25));
		}
		app.update();
		if sensor_count(app.world_mut()) != 0 {
			return Err(anyhow!("recovery must tear the sensor down"));
		}
		Ok(())
	}

	#[test]
	fn cancel_tears_the_sensor_down() -> Result<()> {
		let mut app = sensor_app();
		let mut user = FisticuffsUser::default();
		drive_to_active(&mut user);
		let player = app
			.world_mut()
			.spawn((user, Transform::default(), GlobalTransform::default()))
			.id();
		app.update();
		if sensor_count(app.world_mut()) != 1 {
			return Err(anyhow!("active jab should have a sensor"));
		}
		{
			let mut user =
				app.world_mut().get_mut::<FisticuffsUser>(player).ok_or(anyhow!("user"))?;
			user.state.cancel();
		}
		app.update();
		if sensor_count(app.world_mut()) != 0 {
			return Err(anyhow!("cancel must despawn the fist sensor"));
		}
		Ok(())
	}

	#[test]
	fn swap_removing_the_driver_tears_the_sensor_down() -> Result<()> {
		let mut app = sensor_app();
		let mut user = FisticuffsUser::default();
		drive_to_active(&mut user);
		let player = app
			.world_mut()
			.spawn((user, Transform::default(), GlobalTransform::default()))
			.id();
		app.update();
		if sensor_count(app.world_mut()) != 1 {
			return Err(anyhow!("active jab should have a sensor"));
		}
		app.world_mut().entity_mut(player).remove::<FisticuffsUser>();
		app.update();
		if sensor_count(app.world_mut()) != 0 {
			return Err(anyhow!("losing FisticuffsUser must despawn the sensor"));
		}
		Ok(())
	}

	#[test]
	fn death_tears_the_sensor_down() -> Result<()> {
		let mut app = sensor_app();
		let mut user = FisticuffsUser::default();
		drive_to_active(&mut user);
		app.world_mut().spawn((
			user,
			Health { current: 0.0, max: 100.0 },
			Transform::default(),
			GlobalTransform::default(),
		));
		app.update();
		if sensor_count(app.world_mut()) != 0 {
			return Err(anyhow!("a dead attacker must not keep a fist sensor"));
		}
		Ok(())
	}

	#[test]
	fn no_sensor_without_the_driver() -> Result<()> {
		let mut app = sensor_app();
		app.world_mut()
			.spawn((Transform::default(), GlobalTransform::default(), Player));
		app.update();
		if sensor_count(app.world_mut()) != 0 {
			return Err(anyhow!("an unequipped player must not grow a fist sensor"));
		}
		Ok(())
	}

	#[test]
	fn collect_hits_once_per_target_and_skips_the_attacker() -> Result<()> {
		let mut world = World::new();
		let attacker = world.spawn(Health::from_max(100.0)).id();
		let dummy = world.spawn(Health::from_max(100.0)).id();
		let hull = world.spawn(ChildOf(dummy)).id();
		world
			.run_system_once(
				move |transforms: Query<(Entity, &Transform, Option<&GlobalTransform>)>,
				      health: Query<(), With<Health>>,
				      parents: Query<&ChildOf>| {
					let mut contacted = Vec::new();
					let first = collect_fist_hits(
						attacker,
						PLACEHOLDER_JAB_DAMAGE,
						Vec3::ZERO,
						&[attacker, hull, dummy],
						&mut contacted,
						&transforms,
						&health,
						&parents,
					);
					let second = collect_fist_hits(
						attacker,
						PLACEHOLDER_JAB_DAMAGE,
						Vec3::ZERO,
						&[hull, dummy],
						&mut contacted,
						&transforms,
						&health,
						&parents,
					);
					if first.len() != 1 || first[0].target != dummy {
						return Err(anyhow!(
							"first pass should injure the dummy once, got {first:?}"
						));
					}
					if first[0].source != Some(attacker) || first[0].allow_self {
						return Err(anyhow!("hit must name the attacker and forbid self"));
					}
					if (first[0].amount - PLACEHOLDER_JAB_DAMAGE).abs() > 1e-5 {
						return Err(anyhow!("placeholder amount, got {}", first[0].amount));
					}
					if !second.is_empty() {
						return Err(anyhow!("second pass must dedupe, got {second:?}"));
					}
					Ok(())
				},
			)
			.map_err(|error| anyhow!("{error:?}"))?
	}

	fn physics_hit_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			PhysicsPlugins::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::mesh::MeshPlugin,
		));
		app.finish();
		app.add_message::<Hit>().add_message::<DamageApplied>().add_message::<Died>();
		app.add_systems(Update, (sync_fist_sensors, resolve_fist_hits, apply_hits).chain());
		app.update();
		app
	}

	#[test]
	fn jab_overlap_writes_damage_applied() -> Result<()> {
		let mut app = physics_hit_app();
		let mut user = FisticuffsUser::default();
		drive_to_active(&mut user);
		let player = app
			.world_mut()
			.spawn((
				Player,
				user,
				Transform::from_translation(Vec3::ZERO),
				GlobalTransform::from_translation(Vec3::ZERO),
			))
			.id();
		let dummy = app
			.world_mut()
			.spawn((
				Health::from_max(100.0),
				Transform::from_translation(Vec3::new(0.16, 0.62, -1.28)),
				GlobalTransform::from_translation(Vec3::new(0.16, 0.62, -1.28)),
				RigidBody::Kinematic,
				Collider::capsule(0.4, 1.0),
				PhysicsInteractionLayer::animated_layers(),
			))
			.id();
		app.update();
		app.update();

		let remaining = app.world().get::<Health>(dummy).map(|health| health.current);
		if remaining != Some(100.0 - PLACEHOLDER_JAB_DAMAGE) {
			return Err(anyhow!("dummy should take placeholder jab damage, got {remaining:?}"));
		}
		let applied: Vec<DamageApplied> =
			app.world_mut().resource_mut::<Messages<DamageApplied>>().drain().collect();
		if applied.len() != 1 || applied[0].target != dummy || applied[0].source != Some(player) {
			return Err(anyhow!("expected one player-sourced DamageApplied, got {applied:?}"));
		}
		if (applied[0].applied - PLACEHOLDER_JAB_DAMAGE).abs() > 1e-4 {
			return Err(anyhow!(
				"applied amount {}, want {PLACEHOLDER_JAB_DAMAGE}",
				applied[0].applied
			));
		}
		Ok(())
	}

	#[test]
	fn unequipped_player_does_not_injure_a_dummy() -> Result<()> {
		let mut app = physics_hit_app();
		app.world_mut().spawn((
			Player,
			Transform::from_translation(Vec3::ZERO),
			GlobalTransform::from_translation(Vec3::ZERO),
		));
		let dummy = app
			.world_mut()
			.spawn((
				Health::from_max(100.0),
				Transform::from_translation(Vec3::new(0.0, 0.6, -1.2)),
				GlobalTransform::from_translation(Vec3::new(0.0, 0.6, -1.2)),
				RigidBody::Kinematic,
				Collider::capsule(0.4, 1.0),
				PhysicsInteractionLayer::animated_layers(),
			))
			.id();
		app.update();
		app.update();
		let remaining = app.world().get::<Health>(dummy).map(|health| health.current);
		if remaining != Some(100.0) {
			return Err(anyhow!("unequipped punches must not land, got {remaining:?}"));
		}
		Ok(())
	}
}
