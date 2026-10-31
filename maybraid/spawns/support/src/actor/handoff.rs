//! Guard a validated placement until the physics world has supported it twice.

use avian3d::{collision::collider::contact_query, prelude::*};
use bevy::{ecs::system::SystemParam, prelude::*};
use characters::{CharacterLocomotion, LocomotionCapsule};
use chunk::cascade::CascadeChunk;

mod coverage;
use coverage::Footprint;
use lod_avian::PhysicsInteractionLayer;
use std::time::Duration;

use crate::actor::SupportDiagnostic;

use crate::{
	clearance_blocked, PendingActorSupport, PlacementSearch, PlacementSupportHit,
	SupportProbeFailure,
};

const CLEARANCE_TOLERANCE: f32 = 0.03;
const MAX_LIFT: f32 = 1.0;
const LIFT_STEP: f32 = 0.125;
const SETTLE_TIMEOUT: f32 = 0.75;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum PoseRejection {
	SupportGeometry,
	TerrainCoverage,
	SupportUnavailable,
	SupportDisabled,
	SupportLayers,
	SupportBand,
	SupportContaining,
	SupportNoContact,
	SupportContactNormal,
	SupportContactBand,
	CapsuleClearance,
	ContactCast,
	ContactNormal,
	ContactBand,
	ContactPlane,
	AuthoredMinimum,
	ContactClearance,
}

#[derive(Clone, Debug)]
pub(super) struct SettlingSupport {
	position: Vec3,
	support: Option<PlacementSupportHit>,
	search: PlacementSearch,
	requires_contact: bool,
	confirmed: u8,
	elapsed: f32,
	before_step: Duration,
	vertical_velocity: f32,
}

impl SettlingSupport {
	pub(super) fn new(
		position: Vec3,
		support: Option<PlacementSupportHit>,
		search: PlacementSearch,
		requires_contact: bool,
	) -> Self {
		Self {
			position,
			support,
			search,
			requires_contact,
			confirmed: 0,
			elapsed: 0.0,
			before_step: Duration::ZERO,
			vertical_velocity: 0.0,
		}
	}

	fn contains(&self, position: Vec3) -> bool {
		if !self.requires_contact {
			return position.is_finite() && position.distance(self.position) <= 0.01;
		}
		position.is_finite()
			&& position.xz().distance(self.position.xz()) <= self.search.hull.radius.max(0.25)
			&& position.y <= self.position.y + 0.1
			&& position.y >= self.position.y - MAX_LIFT - 0.3
	}
}

#[derive(SystemParam)]
pub(super) struct SupportGeometry<'w, 's> {
	spatial: Option<SpatialQuery<'w, 's>>,
	locomotion: Option<Res<'w, CharacterLocomotion>>,
	gravity: Option<Res<'w, Gravity>>,
	colliders: Query<
		'w,
		's,
		(
			&'static Position,
			&'static Rotation,
			&'static Collider,
			&'static CollisionLayers,
			Has<Sensor>,
			Has<ColliderDisabled>,
			Has<RigidBodyDisabled>,
			Option<&'static ColliderOf>,
		),
		Without<PendingActorSupport>,
	>,
	disabled_bodies: Query<'w, 's, (), With<RigidBodyDisabled>>,
	terrain: Query<
		'w,
		's,
		(Entity, Option<&'static CascadeChunk>),
		With<durham::TerrainTrimeshCollider>,
	>,
}

impl SupportGeometry<'_, '_> {
	/// Presentation readiness concerns the generated footprint, not whether every
	/// part of that footprint is a floor. A fully cooked cliff remains a valid edge.
	fn terrain_coverage_ready(
		&self,
		position: Vec3,
		hull: LocomotionCapsule,
		support: Option<PlacementSupportHit>,
		layers: &CollisionLayers,
	) -> bool {
		if !support.is_some_and(|hit| self.terrain.contains(hit.entity)) {
			return true;
		}
		Footprint::capsule(position, hull.radius).covered_by(
			self.terrain
				.iter()
				.filter_map(|(entity, chunk)| {
					let (
						position,
						rotation,
						collider,
						fixed_layers,
						sensor,
						disabled,
						body_disabled,
						body,
					) = self.colliders.get(entity).ok()?;
					if sensor
						|| disabled || body_disabled
						|| body.is_some_and(|body| self.disabled_bodies.contains(body.body))
						|| !fixed_layers.memberships.has_all(PhysicsInteractionLayer::Fixed)
						|| !fixed_layers.interacts_with(*layers)
					{
						return None;
					}
					let chunk = chunk?;
					// Durham vertices are local to chunk.origin. Do not credit stale
					// authored bounds after a collider has moved, rotated, or scaled.
					if position.0 != chunk.origin
						|| (rotation.0 != Quat::IDENTITY && rotation.0 != -Quat::IDENTITY)
						|| collider.scale() != Vec3::ONE
					{
						return None;
					}
					Some(Footprint::chunk(chunk))
				})
				.flatten()
				.flatten(),
		)
	}

	/// Compatibility policy: an authored elevated minimum or original zero/upward
	/// effective gravity validates stable live support twice without forcing a fall.
	/// Absent GravityScale still means ordinary Avian gravity; restore its absence.
	pub(super) fn requires_contact(&self, gravity: Option<f32>, elevated: bool) -> bool {
		let gravity_y = self.gravity.as_ref().map_or(-9.81, |gravity| gravity.0.y);
		!elevated && gravity_y * gravity.unwrap_or(1.0) < 0.0
	}

	/// Unoriented triangle meshes are surfaces, so their point query cannot
	/// establish solid containment. In particular, a degenerate triangle may
	/// report off-surface points inside its AABB as contained. Capsule contacts
	/// still enforce clearance against these surfaces in clear() and lift_safe().
	fn contains_volume(
		collider: &Collider,
		position: Position,
		rotation: Rotation,
		point: Vec3,
	) -> bool {
		if collider
			.shape_scaled()
			.as_trimesh()
			.is_some_and(|mesh| mesh.pseudo_normals_if_oriented().is_none())
		{
			return false;
		}
		collider.contains_point(position, rotation, point)
	}

	fn accepts_normal(&self, normal: Vec3) -> bool {
		self.locomotion
			.as_deref()
			.copied()
			.unwrap_or_default()
			.accepts_support_normal(normal)
	}

	/// This check reads the retained collider directly. Avian updates its spatial
	/// tree inside StepSimulation, so a pre-step BVH lookup can still see old geometry.
	fn support_valid(
		&self,
		hit: PlacementSupportHit,
		search: PlacementSearch,
		layers: &CollisionLayers,
	) -> Result<(), PoseRejection> {
		let Ok((
			position,
			rotation,
			collider,
			support_layers,
			sensor,
			disabled,
			body_disabled,
			body,
		)) = self.colliders.get(hit.entity)
		else {
			return Err(PoseRejection::SupportUnavailable);
		};
		if sensor
			|| disabled
			|| body_disabled
			|| body.is_some_and(|body| self.disabled_bodies.contains(body.body))
		{
			return Err(PoseRejection::SupportDisabled);
		}
		if !support_layers.memberships.has_all(PhysicsInteractionLayer::Fixed)
			|| !support_layers.interacts_with(*layers)
		{
			return Err(PoseRejection::SupportLayers);
		}
		if !search.contains_surface(hit.point.y) {
			return Err(PoseRejection::SupportBand);
		}
		let disk = Collider::cylinder((search.hull.radius * 0.5).max(0.08), 0.04);
		let origin = Vec3::new(search.xz.x, hit.point.y + 0.05, search.xz.y);
		if Self::contains_volume(collider, *position, *rotation, origin) {
			return Err(PoseRejection::SupportContaining);
		}
		let mut contacts = Vec::new();
		contact_query::contact_manifolds(
			&disk,
			origin,
			Quat::IDENTITY,
			collider,
			*position,
			*rotation,
			0.1,
			&mut contacts,
		);
		if contacts.is_empty() {
			return Err(PoseRejection::SupportNoContact);
		}
		if !contacts.iter().any(|manifold| self.accepts_normal(-manifold.normal)) {
			return Err(PoseRejection::SupportContactNormal);
		}
		contacts
			.iter()
			.any(|manifold| {
				self.accepts_normal(-manifold.normal)
					&& manifold.points.iter().any(|point| {
						point.point.y >= search.min_surface_y - CLEARANCE_TOLERANCE
							&& point.point.y <= search.max_surface_y + CLEARANCE_TOLERANCE
					})
			})
			.then_some(())
			.ok_or(PoseRejection::SupportContactBand)
	}

	fn clear(
		&self,
		position: Vec3,
		hull: LocomotionCapsule,
		support: Option<PlacementSupportHit>,
	) -> bool {
		let Some(spatial) = &self.spatial else { return false };
		let filter = PhysicsInteractionLayer::Fixed.query_filter();
		if support.is_none() {
			return !clearance_blocked(spatial, &filter, position, hull, None);
		}
		let capsule = Collider::capsule(hull.radius, hull.length);
		let mut entities = spatial.shape_intersections(&capsule, position, Quat::IDENTITY, &filter);
		entities.extend(spatial.point_intersections(position, &filter));
		// Keep the retained collider even if the pre-step BVH has not refreshed.
		// Neighboring tiles use exactly the same shallow upward-contact rule.
		entities.extend(support.map(|hit| hit.entity));
		entities.sort_unstable();
		entities.dedup();
		for entity in entities {
			let Ok((other_position, other_rotation, collider, ..)) = self.colliders.get(entity)
			else {
				return false;
			};
			if Self::contains_volume(collider, *other_position, *other_rotation, position) {
				return false;
			}
			let mut contacts = Vec::new();
			contact_query::contact_manifolds(
				&capsule,
				position,
				Quat::IDENTITY,
				collider,
				*other_position,
				*other_rotation,
				0.0,
				&mut contacts,
			);
			if contacts.iter().any(|manifold| {
				manifold.points.iter().any(|point| {
					point.penetration > CLEARANCE_TOLERANCE
						|| (point.penetration > 0.001 && !self.accepts_normal(-manifold.normal))
				})
			}) {
				return false;
			}
		}
		true
	}

	/// Only upward-facing overlap can be repaired by lifting. Never lift
	/// through a side wall or an overhang to reach a different roof surface.
	fn lift_safe(
		&self,
		position: Vec3,
		hull: LocomotionCapsule,
		support: Option<PlacementSupportHit>,
	) -> bool {
		let Some(spatial) = &self.spatial else { return false };
		let capsule = Collider::capsule(hull.radius, hull.length);
		let filter = PhysicsInteractionLayer::Fixed.query_filter();
		let mut entities = spatial.shape_intersections(&capsule, position, Quat::IDENTITY, &filter);
		entities.extend(support.map(|hit| hit.entity));
		entities.sort_unstable();
		entities.dedup();
		for entity in entities {
			let Ok((other_position, other_rotation, collider, ..)) = self.colliders.get(entity)
			else {
				return false;
			};
			if Self::contains_volume(collider, *other_position, *other_rotation, position) {
				return false;
			}
			let mut contacts = Vec::new();
			contact_query::contact_manifolds(
				&capsule,
				position,
				Quat::IDENTITY,
				collider,
				*other_position,
				*other_rotation,
				0.0,
				&mut contacts,
			);
			if contacts.iter().any(|manifold| {
				!self.accepts_normal(-manifold.normal)
					&& manifold.points.iter().any(|point| point.penetration > 0.001)
			}) {
				return false;
			}
		}
		true
	}

	/// Close the artificial spawn-clearance gap using the complete capsule and
	/// the retained floor, rather than waiting arbitrarily long under tiny gravity.
	/// Physics must still produce two real contact ticks before release.
	pub(super) fn contact_pose(
		&self,
		position: Vec3,
		support: PlacementSupportHit,
		search: PlacementSearch,
		minimum: Option<f32>,
	) -> Result<Vec3, PoseRejection> {
		let (floor_position, floor_rotation, collider, ..) =
			self.colliders.get(support.entity).map_err(|_| PoseRejection::SupportGeometry)?;
		let capsule = Collider::capsule(search.hull.radius, search.hull.length);
		let hit = contact_query::time_of_impact(
			&capsule,
			position,
			Quat::IDENTITY,
			Vec3::NEG_Y,
			collider,
			*floor_position,
			*floor_rotation,
			Vec3::ZERO,
			MAX_LIFT + 0.3,
		)
		.map_err(|_| PoseRejection::ContactCast)?
		.ok_or(PoseRejection::ContactCast)?;
		if hit.time_of_impact <= 1e-4 {
			// TOI's stop-at-penetration result may have no usable normal. A shallow
			// contact already checked by clear() can instead be validated directly.
			let mut contacts = Vec::new();
			contact_query::contact_manifolds(
				&capsule,
				position,
				Quat::IDENTITY,
				collider,
				*floor_position,
				*floor_rotation,
				0.005,
				&mut contacts,
			);
			return (self.clear(position, search.hull, Some(support))
				&& contacts.iter().any(|manifold| {
					self.accepts_normal(-manifold.normal)
						&& manifold.points.iter().any(|point| {
							point.penetration >= -0.005 && search.contains_surface(point.point.y)
						})
				}))
			.then_some(position)
			.ok_or(PoseRejection::ContactClearance);
		}
		let surface = floor_position.0 + floor_rotation.0 * hit.point2;
		if !self.accepts_normal(floor_rotation.0 * hit.normal2) {
			return Err(PoseRejection::ContactNormal);
		}
		if !search.contains_surface(surface.y) {
			return Err(PoseRejection::ContactBand);
		}
		if (surface - support.point).dot(support.normal).abs() > 0.1 {
			return Err(PoseRejection::ContactPlane);
		}
		let standing = position + Vec3::NEG_Y * hit.time_of_impact;
		if minimum.is_some_and(|minimum| standing.y < minimum) {
			return Err(PoseRejection::AuthoredMinimum);
		}
		self.clear(standing, search.hull, Some(support))
			.then_some(standing)
			.ok_or(PoseRejection::ContactClearance)
	}

	pub(super) fn clear_pose(
		&self,
		position: Vec3,
		support: Option<PlacementSupportHit>,
		search: PlacementSearch,
		layers: &CollisionLayers,
	) -> Result<Vec3, PoseRejection> {
		if let Some(hit) = support {
			self.support_valid(hit, search, layers)?;
		}
		if !self.terrain_coverage_ready(position, search.hull, support, layers) {
			return Err(PoseRejection::TerrainCoverage);
		}
		let upper = (search.max_surface_y + search.hull.spawn_height()).max(position.y);
		for step in 0..=8 {
			let lifted = position + Vec3::Y * (step as f32 * LIFT_STEP);
			if lifted.y > upper + 1e-3 {
				break;
			}
			if self.clear(lifted, search.hull, support) {
				return Ok(lifted);
			}
			if !self.lift_safe(lifted, search.hull, support) {
				break;
			}
			// Airborne arrivals have no floor and must keep their authored altitude.
			if support.is_none() {
				break;
			}
		}
		Err(PoseRejection::CapsuleClearance)
	}

	fn validate(
		&self,
		state: &SettlingSupport,
		position: Vec3,
		layers: &CollisionLayers,
	) -> Result<bool, SupportProbeFailure> {
		if !self.terrain_coverage_ready(position, state.search.hull, state.support, layers) {
			return Ok(false);
		}
		if !state.contains(position) {
			return Err(SupportProbeFailure::Placement);
		}
		if !state
			.support
			.is_some_and(|hit| self.support_valid(hit, state.search, layers).is_ok())
		{
			return Err(SupportProbeFailure::NoEligibleSupport);
		}
		if !self.clear(position, state.search.hull, state.support) {
			return Err(SupportProbeFailure::Clearance);
		}
		Ok(true)
	}
}

impl PendingActorSupport {
	fn wait_for_terrain_coverage(&mut self, entity: Entity) {
		self.settling = None;
		self.failure = None;
		self.diagnose(
			entity,
			SupportDiagnostic::WaitingForPresentation,
			Some(RigidBody::Dynamic),
			true,
		);
	}

	fn cancel_handoff(&mut self, entity: Entity, failure: SupportProbeFailure, advance: bool) {
		self.settling = None;
		self.failure = Some(failure);
		self.diagnose(entity, SupportDiagnostic::Failed(failure), Some(RigidBody::Dynamic), true);
		if advance {
			self.advance_after_failure();
		}
	}
}

pub(super) fn guard_actor_support(
	mut commands: Commands,
	geometry: SupportGeometry,
	time: Option<Res<Time<Physics>>>,
	mut actors: Query<(
		Entity,
		&mut PendingActorSupport,
		&mut Transform,
		Option<&Position>,
		Option<&CollisionLayers>,
		Option<&mut GravityScale>,
		Option<&mut LinearVelocity>,
		Option<&mut AngularVelocity>,
		Option<&RigidBody>,
		Option<&Collider>,
		Has<ColliderDisabled>,
		Has<RigidBodyDisabled>,
		Has<Sensor>,
	)>,
) {
	for (
		entity,
		mut pending,
		mut transform,
		position,
		layers,
		gravity,
		velocity,
		angular,
		body,
		collider,
		collider_disabled,
		body_disabled,
		sensor,
	) in &mut actors
	{
		let Some(mut state) = pending.settling.take() else { continue };
		let layers = layers.copied().unwrap_or(CollisionLayers::DEFAULT);
		let Some(position) = position else { continue };
		if !pending.enabled
			|| body != Some(&RigidBody::Dynamic)
			|| collider.is_none()
			|| collider_disabled
			|| body_disabled
			|| sensor
		{
			pending.cancel_handoff(entity, SupportProbeFailure::Placement, false);
			continue;
		}
		match geometry.validate(&state, position.0, &layers) {
			Ok(true) => {}
			Ok(false) => {
				pending.wait_for_terrain_coverage(entity);
				continue;
			}
			Err(failure) => {
				pending.cancel_handoff(entity, failure, failure == SupportProbeFailure::Clearance);
				continue;
			}
		}
		if state.elapsed >= SETTLE_TIMEOUT {
			pending.cancel_handoff(entity, SupportProbeFailure::Placement, true);
			continue;
		}
		state.before_step = time.as_ref().map_or(Duration::ZERO, |time| time.elapsed());
		if !state.requires_contact {
			commands.entity(entity).try_insert(Position(state.position));
			transform.translation = state.position;
		}
		if let Some(mut velocity) = velocity {
			velocity.0 = if state.requires_contact {
				Vec3::Y * state.vertical_velocity.min(0.0)
			} else {
				Vec3::ZERO
			};
		}
		if let Some(mut angular) = angular {
			angular.0 = Vec3::ZERO;
		}
		let scale =
			if state.requires_contact { pending.resume_gravity.flatten() } else { Some(0.0) };
		match (scale, gravity) {
			(Some(scale), Some(mut gravity)) => gravity.0 = scale,
			(Some(scale), None) => {
				commands.entity(entity).try_insert(GravityScale(scale));
			}
			(None, _) => {
				commands.entity(entity).remove::<GravityScale>();
			}
		}
		pending.settling = Some(state);
	}
}

pub(super) fn confirm_actor_support(
	mut commands: Commands,
	geometry: SupportGeometry,
	time: Option<Res<Time<Physics>>>,
	contacts: Option<Res<ContactGraph>>,
	mut actors: Query<(
		Entity,
		&mut PendingActorSupport,
		&Position,
		Option<&CollisionLayers>,
		Option<&LinearVelocity>,
		Option<&RigidBody>,
		Option<&Collider>,
		Has<ColliderDisabled>,
		Has<RigidBodyDisabled>,
		Has<Sensor>,
	)>,
) {
	let Some(time) = time else { return };
	for (
		entity,
		mut pending,
		position,
		layers,
		velocity,
		body,
		collider,
		collider_disabled,
		body_disabled,
		sensor,
	) in &mut actors
	{
		let Some(mut state) = pending.settling.take() else { continue };
		if time.elapsed() <= state.before_step {
			pending.settling = Some(state);
			continue;
		}
		state.elapsed += (time.elapsed() - state.before_step).as_secs_f32();
		state.before_step = time.elapsed();
		state.vertical_velocity = velocity.map_or(0.0, |velocity| velocity.0.y);
		let layers = layers.copied().unwrap_or(CollisionLayers::DEFAULT);
		if !pending.enabled
			|| body != Some(&RigidBody::Dynamic)
			|| collider.is_none()
			|| collider_disabled
			|| body_disabled
			|| sensor
		{
			pending.cancel_handoff(entity, SupportProbeFailure::Placement, false);
			continue;
		}
		match geometry.validate(&state, position.0, &layers) {
			Ok(true) => {}
			Ok(false) => {
				pending.wait_for_terrain_coverage(entity);
				continue;
			}
			Err(failure) => {
				pending.cancel_handoff(entity, failure, failure == SupportProbeFailure::Clearance);
				continue;
			}
		}
		if state.elapsed >= SETTLE_TIMEOUT {
			pending.cancel_handoff(entity, SupportProbeFailure::Placement, true);
			continue;
		}
		let supported = if state.requires_contact {
			state
				.support
				.and_then(|support| contacts.as_ref()?.get(entity, support.entity))
				.is_some_and(|(_, pair)| {
					pair.is_touching()
						&& pair.generates_constraints()
						&& pair.manifolds.iter().any(|manifold| {
							let normal = if pair.collider1 == entity {
								-manifold.normal
							} else {
								manifold.normal
							};
							geometry.accepts_normal(normal)
								&& manifold.points.iter().any(|point| {
									point.penetration >= -0.005
										&& point.point.y
											>= state.search.min_surface_y - CLEARANCE_TOLERANCE
										&& point.point.y
											<= state.search.max_surface_y + CLEARANCE_TOLERANCE
								})
						})
				})
		} else {
			contacts.is_some()
		};
		state.confirmed = if supported { state.confirmed + 1 } else { 0 };
		if state.confirmed < 2 {
			pending.settling = Some(state);
			continue;
		}
		// Original zero/upward gravity and elevated authored minima use stable
		// live geometry validation; they are deliberately not labeled grounded.
		debug!(target: "spawn.support", ?entity, grounded = state.requires_contact, "actor physics handoff validated");
		let mut actor = commands.entity(entity);
		if !state.requires_contact {
			actor.try_insert(Position(state.position));
		}
		match pending.resume_gravity.flatten() {
			Some(scale) => {
				actor.try_insert(GravityScale(scale));
			}
			None => {
				actor.remove::<GravityScale>();
			}
		}
		actor.remove::<PendingActorSupport>();
	}
}
