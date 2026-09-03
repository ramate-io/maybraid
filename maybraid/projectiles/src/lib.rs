//! Query-only bolts and bullets: shapecast through Fixed, emit first contact.

use avian3d::prelude::*;
use avian3d::schedule::PhysicsSchedulePlugin;
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;

/// Capsule bolt. Gravity off.
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

/// Capsule bullet. Gravity on.
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

/// Multiplier on path length spent inside this collider. Missing is `1.0`.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct PenetrationCost(pub f32);

impl Default for PenetrationCost {
	fn default() -> Self {
		Self(1.0)
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

/// Entity responsible for spawning a projectile.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectileSource(pub Entity);

/// First air→solid hit this step. Impacts listen; flights do not spawn VFX.
#[derive(Message, Clone, Copy, Debug)]
pub struct ProjectileContact {
	pub projectile: Entity,
	pub source: Option<Entity>,
	pub target: Entity,
	pub point: Vec3,
	pub normal: Vec3,
}

pub struct ProjectilesPlugin;

impl Plugin for ProjectilesPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<PhysicsSchedulePlugin>() {
			app.add_plugins(PhysicsPlugins::default());
		}
		app.add_message::<ProjectileContact>()
			.add_systems(PostUpdate, tick_flights.after(TransformSystems::Propagate));
	}
}

/// Solid length along a step of `ds`.
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

/// Longest shapecast segment. Faster bolts split `last → pos` so thin colliders
/// are not skipped in one huge `cast_shape`.
pub const MAX_SWEEP_METERS: f32 = 2.0;
const MAX_SWEEP_STEPS: u32 = 24;

fn visit_sweep_segments(start: Vec3, end: Vec3, max_len: f32, mut visit: impl FnMut(Vec3, Vec3)) {
	let delta = end - start;
	let total = delta.length();
	if total <= 1e-5 {
		return;
	}
	let dir = delta / total;
	let steps = ((total / max_len).ceil() as u32).clamp(1, MAX_SWEEP_STEPS);
	let step = total / steps as f32;
	for index in 0..steps {
		let a = start + dir * (step * index as f32);
		let b = start + dir * (step * (index + 1) as f32);
		visit(a, b);
	}
}

fn penetration_filter(projectile: Entity, source: Option<Entity>) -> SpatialQueryFilter {
	let excluded = [Some(projectile), source].into_iter().flatten();
	SpatialQueryFilter::from_mask([
		PhysicsInteractionLayer::Fixed,
		PhysicsInteractionLayer::Animated,
	])
	.with_excluded_entities(excluded)
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

fn overlap_hit(
	spatial: &SpatialQuery,
	collider: &Collider,
	at: Vec3,
	rotation: Quat,
	along: Vec3,
	filter: &SpatialQueryFilter,
) -> Option<(Entity, Vec3, Vec3)> {
	let entity = *spatial.shape_intersections(collider, at, rotation, filter).first()?;
	Some(contact_from_overlap(entity, at, along))
}

/// Point + facing for a shapecast that already sits inside `target`.
fn contact_from_overlap(target: Entity, point: Vec3, along: Vec3) -> (Entity, Vec3, Vec3) {
	let normal = Dir3::new(-along).map(|dir| *dir).unwrap_or(Vec3::Y);
	(target, point, normal)
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

fn first_contact(
	spatial: &SpatialQuery,
	collider: &Collider,
	start: Vec3,
	end: Vec3,
	rotation: Quat,
	filter: &SpatialQueryFilter,
	allow_embedded: bool,
) -> Option<(Entity, Vec3, Vec3)> {
	let delta = end - start;
	if overlapping(spatial, collider, start, rotation, filter) {
		if !allow_embedded {
			return None;
		}
		return overlap_hit(spatial, collider, start, rotation, delta, filter);
	}
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
	Some((hit.entity, hit.point1, hit.normal1.normalize_or(Vec3::Y)))
}

/// Spawn a sensor capsule along `direction` from `muzzle`.
///
/// [`Flight::last`] starts at the muzzle so the first sweep can enter a body
/// the capsule center already occupies.
#[allow(clippy::too_many_arguments)]
pub fn spawn_flight(
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
) -> Entity {
	let transform = capsule_along_y(direction, muzzle, length, radius);
	let collider = Collider::capsule(radius, length);
	commands
		.spawn((
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
			Flight::spawn(muzzle, max_range, max_through, max_age),
		))
		.id()
}

pub fn tick_flights(
	time: Res<Time>,
	spatial: SpatialQuery,
	costs: Query<&PenetrationCost>,
	mut contacts: MessageWriter<ProjectileContact>,
	mut commands: Commands,
	mut flights: Query<(Entity, &mut Flight, &Transform, &Collider, Option<&ProjectileSource>)>,
) {
	let dt = time.delta_secs();
	for (entity, mut flight, transform, collider, source) in &mut flights {
		let pos = transform.translation;
		let rotation = transform.rotation;
		let source = source.map(|source| source.0);
		let filter = penetration_filter(entity, source);
		let ds = pos.distance(flight.last);
		let allow_embedded = flight.path <= 1e-8;
		flight.age += dt;
		flight.path += ds;
		let mut contacted = false;
		if ds <= 1e-5 {
			if allow_embedded {
				if let Some((target, point, normal)) =
					overlap_hit(&spatial, collider, pos, rotation, Vec3::Y, &filter)
				{
					contacts.write(ProjectileContact {
						projectile: entity,
						source,
						target,
						point,
						normal,
					});
				}
			}
		} else {
			visit_sweep_segments(flight.last, pos, MAX_SWEEP_METERS, |start, end| {
				if !contacted {
					if let Some((target, point, normal)) = first_contact(
						&spatial,
						collider,
						start,
						end,
						rotation,
						&filter,
						allow_embedded,
					) {
						contacts.write(ProjectileContact {
							projectile: entity,
							source,
							target,
							point,
							normal,
						});
						contacted = true;
					}
				}
				flight.through +=
					through_on_step(&spatial, collider, start, end, rotation, &filter, &costs);
			});
		}
		flight.last = pos;
		if flight.exhausted() {
			commands.entity(entity).try_despawn();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

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

	#[test]
	fn long_step_splits_into_two_meter_segments() {
		let start = Vec3::ZERO;
		let end = Vec3::X * 8.0;
		let mut segments = Vec::new();
		visit_sweep_segments(start, end, MAX_SWEEP_METERS, |a, b| segments.push((a, b)));
		assert_eq!(segments.len(), 4);
		assert!((segments[0].0 - start).length() < 1e-5);
		assert!((segments[3].1 - end).length() < 1e-4);
		for (a, b) in &segments {
			assert!((a.distance(*b) - 2.0).abs() < 1e-4, "{a} {b}");
		}
	}

	#[test]
	fn short_step_stays_one_segment() {
		let mut n = 0;
		visit_sweep_segments(Vec3::ZERO, Vec3::Z * 0.5, MAX_SWEEP_METERS, |_, _| n += 1);
		assert_eq!(n, 1);
	}

	#[test]
	fn overlap_contact_faces_back_along_flight() {
		let (target, point, normal) =
			contact_from_overlap(Entity::from_bits(7), Vec3::new(1.0, 2.0, 3.0), Vec3::X);
		assert_eq!(target, Entity::from_bits(7));
		assert_eq!(point, Vec3::new(1.0, 2.0, 3.0));
		assert!((normal + Vec3::X).length() < 1e-5);
	}
}
