//! Short-lived fist / forearm hit sensor for an active jab.
//!
//! The volume is a pose-only marker spawned only while [`AttackPhase::Active`].
//! The first Fixed or Animated solid reached by a chamber-to-endpoint capsule
//! sweep emits [`FisticuffsImpact`]. A Health owner also receives a
//! [`damage::Hit`]. The entity does **not** carry an Avian [`Collider`] /
//! [`Sensor`]: those observers queue inserts after the Active window can
//! already have despawned the marker. [`AttackState::combo_step`] is unused
//! for damage or vocals; there is no combo-step damage scale.

use std::collections::{HashMap, HashSet};

use avian3d::prelude::*;
use bevy::prelude::*;
use characters::{CharacterHeading, CharacterRoot};
use damage::{Downed, Health, Hit};
use lod_avian::PhysicsInteractionLayer;

use crate::aim::JabAim;
use crate::attack::AttackPhase;
use crate::{FisticuffsImpact, FisticuffsUser, Side};

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
	/// One physical landing consumes the rest of this jab, including walls.
	pub impact_consumed: bool,
}

#[derive(Clone, Copy)]
struct FistContact {
	entity: Entity,
	distance: f32,
	point: Vec3,
	normal: Vec3,
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
	// Match the posed character's yaw convention: facing −Z, character +X
	// (left) points to world −X. This keeps a positive camera yaw on the
	// same side as the jab pose.
	let left = Vec3::Y.cross(forward).normalize_or(Vec3::X);
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

fn is_descendant_of(entity: Entity, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
	let mut current = entity;
	loop {
		let Ok(child_of) = parents.get(current) else {
			return false;
		};
		current = child_of.parent();
		if current == ancestor {
			return true;
		}
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
	// Pose + ownership only. Overlap uses [`fist_collider`] as a query shape,
	// not a live Avian collider, so physics cannot queue inserts onto a
	// sensor that teardown already despawned.
	commands.spawn((
		Name::new(SENSOR_NAME),
		FisticuffsHitSensor { attacker, side, impact_consumed: false },
		FisticuffsHitSensorOf(attacker),
		ChildOf(attacker),
		transform,
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

/// Sweep to the first Fixed or Animated solid and emit [`FisticuffsImpact`].
/// Write [`Hit`] only when the collider resolves to a Health owner. Self-hits
/// are dropped here and again in [`damage::apply_hits`] (`allow_self: false`).
pub fn resolve_fist_hits(
	spatial: Option<SpatialQuery>,
	users: Query<(
		Entity,
		&FisticuffsUser,
		Option<&GlobalTransform>,
		Option<&Transform>,
		Option<&maybraid_character_controller::ActorControlStatus>,
	)>,
	visuals: Query<(&CharacterHeading, &ChildOf), With<CharacterRoot>>,
	health: Query<&Health>,
	parents: Query<&ChildOf>,
	downed: Query<(), With<Downed>>,
	mut sensors: Query<(Entity, &mut FisticuffsHitSensor)>,
	mut hits: MessageWriter<Hit>,
	mut impacts: MessageWriter<FisticuffsImpact>,
) {
	let Some(spatial) = spatial else {
		return;
	};
	for (sensor, mut volume) in &mut sensors {
		if volume.impact_consumed {
			continue;
		}
		let Ok((attacker, user, global, local, control)) = users.get(volume.attacker) else {
			continue;
		};
		if user.state.phase() != Some(AttackPhase::Active)
			|| downed.contains(attacker)
			|| health.get(attacker).is_ok_and(|pool| pool.is_dead())
			|| control.is_some_and(|control| control.is_frozen())
		{
			continue;
		}
		let heading = heading_of(attacker, &visuals);
		let aim = user.state.aim().unwrap_or_default();
		let origin = attacker_origin(global, local);
		let point = character_to_world(origin, heading, fist_local(volume.side, aim));
		let anchor = character_to_world(
			origin,
			heading,
			Vec3::new(
				match volume.side {
					Side::Left => PLACEHOLDER_FIST_SIDE_LEFT,
					Side::Right => -PLACEHOLDER_FIST_SIDE_RIGHT,
				},
				PLACEHOLDER_FIST_HEIGHT,
				0.18,
			),
		);
		let rotation = fist_world_rotation(heading, aim.direction());
		let mut filter = SpatialQueryFilter::from_mask([
			PhysicsInteractionLayer::Fixed,
			PhysicsInteractionLayer::Animated,
		]);
		filter.excluded_entities.insert(sensor);
		filter.excluded_entities.insert(volume.attacker);
		// Rig descendants are skipped in the all-hit callback, walking only the
		// candidates returned by physics rather than scanning the entity tree.
		let delta = point - anchor;
		let distance = delta.length();
		let direction_vec = if distance > 1e-5 {
			delta / distance
		} else {
			(character_to_world(Vec3::ZERO, heading, aim.direction())).normalize_or(-Vec3::Z)
		};
		let Ok(direction) = Dir3::new(direction_vec) else {
			continue;
		};
		let mut config = ShapeCastConfig::from_max_distance(distance);
		// Avian reports initial penetration at distance zero and computes the
		// real target-surface point/normal. Keep this explicit for overlaps.
		config.compute_contact_on_penetration = true;
		config.ignore_origin_penetration = false;
		let mut candidates = Vec::new();
		spatial.shape_hits_callback(
			&fist_collider(),
			anchor,
			rotation,
			direction,
			&config,
			&filter,
			|hit| {
				if is_descendant_of(hit.entity, volume.attacker, &parents) {
					return true;
				}
				candidates.push(FistContact {
					entity: hit.entity,
					distance: hit.distance,
					point: hit.point1,
					normal: hit.normal1,
				});
				true
			},
		);
		candidates.sort_by(|a, b| a.distance.total_cmp(&b.distance));
		let Some(collision) = candidates.first() else {
			continue;
		};
		let collider = collision.entity;
		let parent = parents.get(collider).ok().map(|child| child.parent());
		let owner = Health::entity_or_parent(
			collider,
			parent,
			health.contains(collider),
			parent.is_some_and(|parent| health.contains(parent)),
		);
		let target = (owner != volume.attacker && health.contains(owner)).then_some(owner);
		let mut normal = collision.normal.normalize_or(-direction_vec);
		if normal.dot(direction_vec) > 0.0 {
			normal = -normal;
		}
		volume.impact_consumed = true;
		impacts.write(FisticuffsImpact {
			attacker: volume.attacker,
			side: volume.side,
			collider,
			target,
			point: collision.point,
			normal,
		});
		if let Some(target) = target {
			hits.write(Hit {
				target,
				source: Some(volume.attacker),
				amount: PLACEHOLDER_JAB_DAMAGE,
				point: collision.point,
				allow_self: false,
			});
		}
	}
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
		assert!((world - Vec3::new(1.84, 0.62, 2.72)).length() < 1e-4);
		let left = character_to_world(origin, -Vec3::Z, Vec3::X);
		assert!((left - (origin - Vec3::X)).length() < 1e-4);
	}

	#[test]
	fn camera_yaw_aim_round_trips_through_the_world_basis() {
		use player::{look_forward, PlayerLook};

		let headings = [
			-Vec3::Z,
			Vec3::X,
			Vec3::new(1.0, 0.0, -1.0).normalize(),
			Vec3::new(-0.3, 0.0, 1.0).normalize(),
		];
		for heading in headings {
			for pitch in [-12.0_f32, 0.0, 12.0] {
				for relative_yaw in [-24.0_f32, 24.0] {
					let look = PlayerLook {
						yaw: crate::body_camera_yaw(heading) + relative_yaw.to_radians(),
						pitch: pitch.to_radians(),
						first_person: true,
						focus: 0.0,
					};
					let aim = JabAim::from_look(&look, heading);
					let world =
						character_to_world(Vec3::ZERO, heading, aim.direction()).normalize();
					let expected = look_forward(&look);
					assert!(
						(world - expected).length() < 1e-4,
						"heading {heading:?}, pitch {pitch}, yaw {relative_yaw}: {world:?} != {expected:?}"
					);
				}
			}
		}
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
		app.add_message::<Hit>()
			.add_message::<DamageApplied>()
			.add_message::<Died>()
			.add_message::<FisticuffsImpact>();
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
			.spawn((Health::from_max(100.0), Transform::IDENTITY, GlobalTransform::IDENTITY))
			.id();
		let hull = app
			.world_mut()
			.spawn((
				ChildOf(dummy),
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
		let impacts: Vec<FisticuffsImpact> =
			app.world_mut().resource_mut::<Messages<FisticuffsImpact>>().drain().collect();
		if impacts.len() != 1 || impacts[0].target != Some(dummy) || impacts[0].collider != hull {
			return Err(anyhow!("child hull should resolve to its Health owner, got {impacts:?}"));
		}
		if !impacts[0].point.is_finite() || impacts[0].normal.dot(-Vec3::Z) > 1e-4 {
			return Err(anyhow!(
				"impact should have finite surface data facing the incoming fist: {impacts:?}"
			));
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
	fn initial_overlap_reports_surface_contact_instead_of_the_sweep_origin() -> Result<()> {
		let mut app = physics_hit_app();
		let mut user = FisticuffsUser::default();
		drive_to_active(&mut user);
		app.world_mut().spawn((user, Transform::IDENTITY, GlobalTransform::IDENTITY));
		let center = Vec3::new(0.0, 0.62, -0.2);
		let body = app
			.world_mut()
			.spawn((
				Health::from_max(100.0),
				Transform::from_translation(center),
				GlobalTransform::from_translation(center),
				RigidBody::Kinematic,
				Collider::sphere(0.2),
				PhysicsInteractionLayer::animated_layers(),
			))
			.id();
		app.update();
		app.update();

		let impacts: Vec<FisticuffsImpact> =
			app.world_mut().resource_mut::<Messages<FisticuffsImpact>>().drain().collect();
		if impacts.len() != 1 || impacts[0].collider != body || impacts[0].target != Some(body) {
			return Err(anyhow!("initial overlap should produce one body impact, got {impacts:?}"));
		}
		let offset = impacts[0].point - center;
		if !impacts[0].point.is_finite()
			|| !impacts[0].normal.is_finite()
			|| (offset.length() - 0.2).abs() > 0.04
		{
			return Err(anyhow!(
				"overlap impact should retain Avian surface geometry: {impacts:?}"
			));
		}
		Ok(())
	}

	fn fixed_environment_impact(collider: Collider) -> Result<FisticuffsImpact> {
		let mut app = physics_hit_app();
		let mut user = FisticuffsUser::default();
		drive_to_active(&mut user);
		app.world_mut().spawn((user, Transform::IDENTITY, GlobalTransform::IDENTITY));
		app.world_mut().spawn((
			Transform::IDENTITY,
			GlobalTransform::IDENTITY,
			RigidBody::Static,
			collider,
			PhysicsInteractionLayer::fixed_layers(),
		));
		app.update();
		app.update();
		let impacts: Vec<FisticuffsImpact> =
			app.world_mut().resource_mut::<Messages<FisticuffsImpact>>().drain().collect();
		if impacts.len() != 1 || impacts[0].target.is_some() {
			return Err(anyhow!(
				"fixed environmental solid should emit one non-damage impact: {impacts:?}"
			));
		}
		if !impacts[0].point.is_finite() || !impacts[0].normal.is_finite() {
			return Err(anyhow!("environment impact surface geometry must be finite: {impacts:?}"));
		}
		Ok(impacts[0])
	}

	#[test]
	fn terrain_trimesh_and_tree_building_compounds_emit_environment_impacts() -> Result<()> {
		let terrain = Collider::trimesh(
			vec![
				Vec3::new(-1.0, 0.0, -0.7),
				Vec3::new(1.0, 0.0, -0.7),
				Vec3::new(1.0, 1.5, -0.7),
				Vec3::new(-1.0, 1.5, -0.7),
			],
			vec![[0, 1, 2], [0, 2, 3]],
		);
		fixed_environment_impact(terrain)?;

		let tree = Collider::compound(vec![(
			Vec3::new(0.0, 0.7, -0.7),
			Quat::IDENTITY,
			Collider::capsule(0.16, 1.1),
		)]);
		fixed_environment_impact(tree)?;

		let building = Collider::compound(vec![
			(Vec3::new(0.0, 0.6, -0.7), Quat::IDENTITY, Collider::cuboid(1.0, 1.2, 0.2)),
			(Vec3::new(0.0, 1.3, -0.7), Quat::IDENTITY, Collider::cuboid(1.2, 0.2, 0.2)),
		]);
		fixed_environment_impact(building)?;
		Ok(())
	}

	#[test]
	fn fixed_wall_stops_damage_to_body_behind_it() -> Result<()> {
		let mut app = physics_hit_app();
		let mut user = FisticuffsUser::default();
		drive_to_active(&mut user);
		let player = app
			.world_mut()
			.spawn((user, Transform::IDENTITY, GlobalTransform::IDENTITY))
			.id();
		let wall = app
			.world_mut()
			.spawn((
				Transform::from_translation(Vec3::new(0.0, 0.62, -0.72)),
				GlobalTransform::from_translation(Vec3::new(0.0, 0.62, -0.72)),
				RigidBody::Static,
				Collider::cuboid(1.0, 1.0, 0.2),
				PhysicsInteractionLayer::fixed_layers(),
			))
			.id();
		let dummy = app
			.world_mut()
			.spawn((
				Health::from_max(100.0),
				Transform::from_translation(Vec3::new(0.0, 0.62, -1.35)),
				GlobalTransform::from_translation(Vec3::new(0.0, 0.62, -1.35)),
				RigidBody::Kinematic,
				Collider::capsule(0.4, 1.0),
				PhysicsInteractionLayer::animated_layers(),
			))
			.id();
		app.update();
		app.update();

		let remaining = app.world().get::<Health>(dummy).map(|health| health.current);
		if remaining != Some(100.0) {
			return Err(anyhow!("wall must protect the body behind it, health={remaining:?}"));
		}
		let impacts: Vec<FisticuffsImpact> =
			app.world_mut().resource_mut::<Messages<FisticuffsImpact>>().drain().collect();
		if impacts.len() != 1 || impacts[0].collider != wall || impacts[0].target.is_some() {
			return Err(anyhow!("wall should be the only contact, got {impacts:?}"));
		}
		if app.world().get::<FisticuffsUser>(player).is_none() {
			return Err(anyhow!("attacker should remain equipped"));
		}
		Ok(())
	}

	#[test]
	fn nearer_body_wins_when_wall_is_behind_it() -> Result<()> {
		let mut app = physics_hit_app();
		let mut user = FisticuffsUser::default();
		drive_to_active(&mut user);
		app.world_mut().spawn((user, Transform::IDENTITY, GlobalTransform::IDENTITY));
		let wall = app
			.world_mut()
			.spawn((
				Transform::from_translation(Vec3::new(0.0, 0.62, -0.92)),
				GlobalTransform::from_translation(Vec3::new(0.0, 0.62, -0.92)),
				RigidBody::Static,
				Collider::cuboid(1.0, 1.0, 0.2),
				PhysicsInteractionLayer::fixed_layers(),
			))
			.id();
		let dummy = app
			.world_mut()
			.spawn((
				Health::from_max(100.0),
				Transform::from_translation(Vec3::new(0.0, 0.62, -0.56)),
				GlobalTransform::from_translation(Vec3::new(0.0, 0.62, -0.56)),
				RigidBody::Kinematic,
				Collider::capsule(0.25, 0.5),
				PhysicsInteractionLayer::animated_layers(),
			))
			.id();
		app.update();
		app.update();

		let remaining = app.world().get::<Health>(dummy).map(|health| health.current);
		if remaining != Some(100.0 - PLACEHOLDER_JAB_DAMAGE) {
			return Err(anyhow!("nearer body should receive the jab, health={remaining:?}"));
		}
		let impacts: Vec<FisticuffsImpact> =
			app.world_mut().resource_mut::<Messages<FisticuffsImpact>>().drain().collect();
		if impacts.len() != 1 || impacts[0].collider != dummy || impacts[0].target != Some(dummy) {
			return Err(anyhow!(
				"body before the wall should be the first contact, got {impacts:?}"
			));
		}
		if impacts[0].collider == wall {
			return Err(anyhow!("wall behind body must not win"));
		}
		Ok(())
	}

	#[test]
	fn own_descendant_hull_is_skipped_for_the_first_external_body() -> Result<()> {
		let mut app = physics_hit_app();
		let mut user = FisticuffsUser::default();
		drive_to_active(&mut user);
		let player = app
			.world_mut()
			.spawn((user, Transform::IDENTITY, GlobalTransform::IDENTITY))
			.id();
		let own_hull = app
			.world_mut()
			.spawn((
				ChildOf(player),
				Transform::from_translation(Vec3::new(0.0, 0.62, -0.48)),
				GlobalTransform::from_translation(Vec3::new(0.0, 0.62, -0.48)),
				RigidBody::Static,
				Collider::cuboid(0.8, 0.8, 0.1),
				PhysicsInteractionLayer::fixed_layers(),
			))
			.id();
		let dummy = app
			.world_mut()
			.spawn((
				Health::from_max(100.0),
				Transform::from_translation(Vec3::new(0.0, 0.62, -1.32)),
				GlobalTransform::from_translation(Vec3::new(0.0, 0.62, -1.32)),
				RigidBody::Kinematic,
				Collider::capsule(0.35, 0.8),
				PhysicsInteractionLayer::animated_layers(),
			))
			.id();
		app.update();
		app.update();

		let remaining = app.world().get::<Health>(dummy).map(|health| health.current);
		if remaining != Some(100.0 - PLACEHOLDER_JAB_DAMAGE) {
			return Err(anyhow!("external body should receive the jab, health={remaining:?}"));
		}
		let impacts: Vec<FisticuffsImpact> =
			app.world_mut().resource_mut::<Messages<FisticuffsImpact>>().drain().collect();
		if impacts.len() != 1 || impacts[0].collider != dummy || impacts[0].collider == own_hull {
			return Err(anyhow!("own descendant must not block the sweep, got {impacts:?}"));
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
