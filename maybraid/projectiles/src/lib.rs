//! Query-only bolts and bullets: shapecast through Fixed / Animated and emit
//! each distinct contact along the flight.

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
#[derive(Component, Debug, Clone)]
pub struct Flight {
	pub origin: Vec3,
	pub last: Vec3,
	pub path: f32,
	pub through: f32,
	pub age: f32,
	pub max_range: f32,
	pub max_through: f32,
	pub max_age: f32,
	contacted: Vec<Entity>,
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
			contacted: Vec::new(),
		}
	}

	pub fn exhausted(&self) -> bool {
		self.path >= self.max_range || self.through > self.max_through || self.age >= self.max_age
	}

	fn note_contact(&mut self, target: Entity) -> bool {
		if self.contacted.contains(&target) {
			return false;
		}
		self.contacted.push(target);
		true
	}
}

/// Entity responsible for spawning a projectile.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProjectileSource(pub Entity);

/// One distinct collider crossed by a flight. Impacts listen; flights do not spawn VFX.
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
		(true, false) => match (backward_hit, forward_hit) {
			(Some(air), _) => clamp(ds - air),
			(None, Some(exit)) => clamp(exit),
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

fn contacts_on_step(
	spatial: &SpatialQuery,
	collider: &Collider,
	start: Vec3,
	end: Vec3,
	rotation: Quat,
	filter: &SpatialQueryFilter,
) -> Vec<ShapeHitData> {
	let delta = end - start;
	let ds = delta.length();
	if ds <= 1e-5 {
		return Vec::new();
	}
	let Ok(dir) = Dir3::new(delta) else {
		return Vec::new();
	};
	let config = ShapeCastConfig::from_max_distance(ds);
	let mut hits = Vec::new();
	spatial.shape_hits_callback(collider, start, rotation, dir, &config, filter, |hit| {
		hits.push(hit);
		true
	});
	// Avian's all-hit query does not guarantee traversal order.
	hits.sort_by(|a, b| a.distance.total_cmp(&b.distance));
	hits
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct PenetrationSpan {
	enter: f32,
	exit: f32,
	cost: f32,
}

fn penetration_spans(
	spatial: &SpatialQuery,
	collider: &Collider,
	start: Vec3,
	end: Vec3,
	rotation: Quat,
	filter: &SpatialQueryFilter,
	costs: &Query<&PenetrationCost>,
	forward: &[ShapeHitData],
) -> Vec<PenetrationSpan> {
	let ds = start.distance(end);
	if ds <= 1e-5 {
		return Vec::new();
	}
	let starts = spatial.shape_intersections(collider, start, rotation, filter);
	let ends = spatial.shape_intersections(collider, end, rotation, filter);
	let backward = contacts_on_step(spatial, collider, end, start, rotation, filter);
	let mut entities = Vec::new();
	for entity in starts
		.iter()
		.chain(&ends)
		.chain(forward.iter().map(|hit| &hit.entity))
		.chain(backward.iter().map(|hit| &hit.entity))
	{
		if !entities.contains(entity) {
			entities.push(*entity);
		}
	}
	entities
		.into_iter()
		.filter_map(|entity| {
			let enter = if starts.contains(&entity) {
				0.0
			} else {
				forward.iter().find(|hit| hit.entity == entity)?.distance.clamp(0.0, ds)
			};
			let exit = if ends.contains(&entity) {
				ds
			} else {
				let air = backward.iter().find(|hit| hit.entity == entity)?.distance;
				(ds - air).clamp(0.0, ds)
			};
			(exit > enter + 1e-5).then_some(PenetrationSpan {
				enter,
				exit,
				cost: costs.get(entity).map_or(1.0, |cost| cost.0),
			})
		})
		.collect()
}

fn penetration_at(spans: &[PenetrationSpan], distance: f32) -> f32 {
	spans
		.iter()
		.map(|span| (distance.min(span.exit) - span.enter).max(0.0) * span.cost)
		.sum()
}

fn allowed_step_distance(flight: &Flight, ds: f32, dt: f32) -> f32 {
	let range = (flight.max_range - flight.path).max(0.0);
	let age_fraction =
		if dt <= 1e-8 { 1.0 } else { ((flight.max_age - flight.age) / dt).clamp(0.0, 1.0) };
	ds.min(range).min(ds * age_fraction)
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
		let delta = pos - flight.last;
		let ds = delta.length();
		let allowed = allowed_step_distance(&flight, ds, dt);
		let end = if ds <= 1e-5 { flight.last } else { flight.last + delta / ds * allowed };
		flight.age = (flight.age + dt).min(flight.max_age);
		flight.path += allowed;
		let mut budget_exhausted = false;
		visit_sweep_segments(flight.last, end, MAX_SWEEP_METERS, |start, end| {
			if budget_exhausted {
				return;
			}
			let segment = end - start;
			let segment_length = segment.length();
			let direction = segment / segment_length.max(1e-8);
			let hits = contacts_on_step(&spatial, collider, start, end, rotation, &filter);
			let spans =
				penetration_spans(&spatial, collider, start, end, rotation, &filter, &costs, &hits);
			for hit in hits {
				let to_hit = hit.distance.clamp(0.0, segment_length);
				let through_before = penetration_at(&spans, to_hit);
				if flight.through + through_before > flight.max_through {
					break;
				}
				if flight.note_contact(hit.entity) {
					contacts.write(ProjectileContact {
						projectile: entity,
						source,
						target: hit.entity,
						point: hit.point1,
						normal: hit.normal1.normalize_or(-direction),
					});
				}
			}
			flight.through += penetration_at(&spans, segment_length);
			budget_exhausted = flight.through > flight.max_through;
		});
		flight.last = pos;
		if budget_exhausted || allowed < ds - 1e-5 || flight.exhausted() {
			commands.entity(entity).try_despawn();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

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
	fn flight_reports_each_entity_once() {
		let mut flight = Flight::spawn(Vec3::ZERO, 10.0, 1.0, 2.0);
		let a = Entity::from_bits(7);
		let b = Entity::from_bits(8);
		assert!(flight.note_contact(a));
		assert!(!flight.note_contact(a));
		assert!(flight.note_contact(b));
	}

	#[test]
	fn step_is_clamped_to_range_and_age() {
		let mut flight = Flight::spawn(Vec3::ZERO, 10.0, 1.0, 2.0);
		flight.path = 8.0;
		assert!((allowed_step_distance(&flight, 5.0, 0.1) - 2.0).abs() < 1e-5);
		flight.path = 0.0;
		flight.age = 1.95;
		assert!((allowed_step_distance(&flight, 5.0, 0.1) - 2.5).abs() < 1e-5);
	}

	#[test]
	fn penetration_sums_only_solid_spans() {
		let spans = [
			PenetrationSpan { enter: 0.2, exit: 0.5, cost: 2.0 },
			PenetrationSpan { enter: 0.8, exit: 1.0, cost: 1.0 },
		];
		assert!((penetration_at(&spans, 0.1) - 0.0).abs() < 1e-5);
		assert!((penetration_at(&spans, 0.6) - 0.6).abs() < 1e-5);
		assert!((penetration_at(&spans, 0.9) - 0.7).abs() < 1e-5);
		assert!((penetration_at(&spans, 1.2) - 0.8).abs() < 1e-5);
	}

	#[test]
	fn sweep_returns_multiple_ordered_contacts() {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			PhysicsPlugins::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::mesh::MeshPlugin,
		));
		app.finish();
		let near = app
			.world_mut()
			.spawn((
				RigidBody::Static,
				Collider::cuboid(0.2, 1.0, 1.0),
				Transform::from_xyz(2.0, 0.0, 0.0),
				PhysicsInteractionLayer::fixed_layers(),
			))
			.id();
		let far = app
			.world_mut()
			.spawn((
				RigidBody::Static,
				Collider::cuboid(0.2, 1.0, 1.0),
				Transform::from_xyz(4.0, 0.0, 0.0),
				PhysicsInteractionLayer::fixed_layers(),
			))
			.id();
		app.update();

		let hits = app
			.world_mut()
			.run_system_once(move |spatial: SpatialQuery| {
				contacts_on_step(
					&spatial,
					&Collider::sphere(0.05),
					Vec3::ZERO,
					Vec3::X * 6.0,
					Quat::IDENTITY,
					&PhysicsInteractionLayer::Fixed.query_filter(),
				)
			})
			.expect("spatial query system should run");
		assert_eq!(hits.iter().map(|hit| hit.entity).collect::<Vec<_>>(), [near, far]);
		assert!(hits[0].distance < hits[1].distance);

		let overlap_hits = app
			.world_mut()
			.run_system_once(move |spatial: SpatialQuery| {
				contacts_on_step(
					&spatial,
					&Collider::sphere(0.05),
					Vec3::X * 2.0,
					Vec3::X * 6.0,
					Quat::IDENTITY,
					&PhysicsInteractionLayer::Fixed.query_filter(),
				)
			})
			.expect("spatial query system should run");
		assert_eq!(overlap_hits.iter().map(|hit| hit.entity).collect::<Vec<_>>(), [near, far]);
		assert!(overlap_hits[0].distance <= 1e-5);
	}
}
