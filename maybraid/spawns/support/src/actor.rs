//! Persistent support readiness for bodies constructed before their floor.

use avian3d::prelude::{
	AngularVelocity, Collider, GravityScale, LinearVelocity, PhysicsSystems, Position, RigidBody,
	SpatialQuery, SpatialQueryFilter,
};
use bevy::{ecs::query::Has, prelude::*, transform::TransformSystems};
use characters::{CharacterLocomotion, LocomotionCapsule};
use lod_avian::PhysicsInteractionLayer;

use crate::{
	clearance_blocked, probe_search_with_locomotion, PlacementSearch, SupportProbeFailure,
	SupportProbeOutcome,
};

/// Log once when the reason a live actor remains held changes. This state is
/// cloned with adapter refreshes, so a persistent request never logs per frame.
#[derive(Clone, Copy, Debug, PartialEq)]
enum SupportDiagnostic {
	MissingSpatialQuery,
	BodyNotReady { body: Option<RigidBody>, collider: bool },
	Disabled,
	MissingSearch,
	WaitingForPresentation,
	Failed(SupportProbeFailure),
	FinalPoseClearance,
	Ready { position: Vec3, support: Option<Entity> },
}

/// Existing destinations stay exact. Initial terrain population may explicitly
/// choose a bounded nearby search using the same support/clearance probe.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ActorSupportSearchPolicy {
	#[default]
	Exact,
	NearbyTerrain {
		max_radius: f32,
	},
}

/// Holds one existing actor until its intended support column is usable.
/// Search updates never replace the original pose or gravity snapshot.
#[derive(Component, Clone, Debug)]
pub struct PendingActorSupport {
	held_position: Vec3,
	hull: LocomotionCapsule,
	minimum_center_y: Option<f32>,
	search: Option<PlacementSearch>,
	search_policy: ActorSupportSearchPolicy,
	candidate: u8,
	exhausted: bool,
	enabled: bool,
	// Outer None: the dynamic body has not yet supplied its original gravity.
	// Inner None: the actor originally had no GravityScale component.
	resume_gravity: Option<Option<f32>>,
	last_diagnostic: Option<SupportDiagnostic>,
	pub failure: Option<SupportProbeFailure>,
}

impl PendingActorSupport {
	pub fn new(
		held_position: Vec3,
		hull: LocomotionCapsule,
		minimum_center_y: Option<f32>,
	) -> Self {
		Self {
			held_position,
			hull,
			minimum_center_y,
			search: None,
			search_policy: ActorSupportSearchPolicy::Exact,
			candidate: 0,
			exhausted: false,
			enabled: true,
			resume_gravity: None,
			last_diagnostic: None,
			failure: None,
		}
	}

	/// Try the original column, then eight directions on four concentric
	/// rings. The caller owns the maximum permitted relocation distance.
	pub fn with_nearby_terrain(mut self, max_radius: f32) -> Self {
		self.search_policy = ActorSupportSearchPolicy::NearbyTerrain { max_radius };
		self.candidate = 0;
		self.exhausted = !max_radius.is_finite() || max_radius <= 0.0;
		self
	}

	/// Current candidate to sample for published terrain, distinct from the
	/// original position held while generation/collision is pending.
	pub fn search_xz(&self) -> Vec2 {
		let ActorSupportSearchPolicy::NearbyTerrain { max_radius } = self.search_policy else {
			return self.held_position.xz();
		};
		if self.candidate == 0 || self.exhausted {
			return self.held_position.xz();
		}
		let candidate = self.candidate - 1;
		let radius = max_radius * (f32::from(candidate / 8 + 1) * 0.25);
		let angle = f32::from(candidate % 8) * std::f32::consts::TAU / 8.0;
		self.held_position.xz() + Vec2::new(angle.cos(), angle.sin()) * radius
	}

	pub const fn search_policy(&self) -> ActorSupportSearchPolicy {
		self.search_policy
	}

	pub const fn exhausted(&self) -> bool {
		self.exhausted
	}

	fn advance_after_failure(&mut self) {
		if !matches!(self.search_policy, ActorSupportSearchPolicy::NearbyTerrain { .. }) {
			return;
		}
		if self.candidate < 32 {
			self.candidate += 1;
		} else {
			self.exhausted = true;
		}
		self.search = None;
	}

	/// Explicit domain override, including restoring an absent component.
	pub fn with_resume_gravity(mut self, gravity: Option<f32>) -> Self {
		self.resume_gravity = Some(gravity);
		self
	}

	/// Replace the intended arrival before the next physics hold. Preserve the
	/// saved gravity even though the currently held body has forced zero gravity.
	pub fn retarget(
		&mut self,
		held_position: Vec3,
		hull: LocomotionCapsule,
		minimum_center_y: Option<f32>,
	) {
		self.held_position = held_position;
		self.hull = hull;
		self.minimum_center_y = minimum_center_y;
		self.last_diagnostic = None;
		self.candidate = 0;
		self.exhausted = matches!(self.search_policy, ActorSupportSearchPolicy::NearbyTerrain { max_radius } if !max_radius.is_finite() || max_radius <= 0.0);
		self.search = None;
		self.failure = None;
	}

	fn diagnose(
		&mut self,
		entity: Entity,
		state: SupportDiagnostic,
		body: Option<RigidBody>,
		collider: bool,
	) -> bool {
		if self.last_diagnostic == Some(state) {
			return false;
		}
		self.last_diagnostic = Some(state);
		debug!(
			target: "spawn.support",
			?entity, ?state, ?body, collider,
			held = ?self.held_position,
			search = ?self.search,
			hull = ?self.hull,
			minimum_center_y = ?self.minimum_center_y,
			resume_gravity = ?self.resume_gravity,
			search_policy = ?self.search_policy, candidate = self.candidate, exhausted = self.exhausted,
			"actor support transition"
		);
		true
	}

	fn diagnose_geometry(
		&self,
		entity: Entity,
		spatial: &SpatialQuery,
		filter: &SpatialQueryFilter,
		collider_names: &Query<(Option<&Name>, Has<durham::TerrainTrimeshCollider>)>,
		locomotion: &CharacterLocomotion,
	) {
		if !bevy::log::tracing::enabled!(target: "spawn.support", bevy::log::Level::DEBUG) {
			return;
		}
		let Some(search) = self.search.filter(|search| search.bounds_valid()) else { return };
		let geometry =
			crate::probe::ProbeDiagnostics::collect(spatial, filter, search, self.minimum_center_y);
		let entities: Vec<_> = geometry
			.entities()
			.into_iter()
			.map(|collider| {
				let (name, terrain) = collider_names.get(collider).ok().unwrap_or((None, false));
				(collider, name.map(Name::as_str), terrain)
			})
			.collect();
		debug!(
			target: "spawn.support", ?entity,
			origin = ?geometry.origin,
			start_overlaps = ?geometry.start_overlaps,
			start_containing = ?geometry.start_containing,
			first_hit = ?geometry.first_hit,
			max_slope_degrees = locomotion.max_slope_angle.to_degrees(),
			first_hit_normal_accepted = ?geometry.first_hit.map(|hit| search.support.accepts_normal_with_locomotion(hit.normal, locomotion)),
			first_hit_inside_search = ?geometry.first_hit.map(|hit| search.contains_surface(hit.point.y)),
			final_position = ?geometry.final_position,
			final_clearance = ?geometry.final_clearance,
			final_containing = ?geometry.final_containing,
			colliders = ?entities,
			"actor support failure geometry (collider, name, terrain_trimesh)"
		);
	}

	pub fn observe_search(&mut self, search: Option<PlacementSearch>) {
		self.search = search;
	}

	pub fn set_enabled(&mut self, enabled: bool) {
		self.enabled = enabled;
	}

	pub const fn held_position(&self) -> Vec3 {
		self.held_position
	}

	pub const fn hull(&self) -> LocomotionCapsule {
		self.hull
	}

	/// Saved gravity: outer `None` means the original body has not been seen;
	/// `Some(None)` means release must remove the temporary GravityScale.
	pub const fn resume_gravity(&self) -> Option<Option<f32>> {
		self.resume_gravity
	}

	pub const fn search(&self) -> Option<PlacementSearch> {
		self.search
	}
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ActorSupportSystems {
	/// Domain adapters enqueue actors and refresh generated search hints.
	Prepare,
	/// Shared geometry validates the final pose and releases accepted actors.
	Resolve,
	/// Enforce held pose and zero motion after frame motors / before physics.
	Hold,
}

#[derive(Resource, Default)]
struct ActorSupportRuntime;

/// Register once from any actor domain. Works without physics in headless apps.
pub fn register_actor_support(app: &mut App) {
	if app.world().contains_resource::<ActorSupportRuntime>() {
		return;
	}
	app.init_resource::<ActorSupportRuntime>()
		.configure_sets(
			Update,
			(ActorSupportSystems::Prepare, ActorSupportSystems::Resolve).chain(),
		)
		.add_systems(Update, resolve_actor_support.in_set(ActorSupportSystems::Resolve))
		.add_systems(
			PostUpdate,
			hold_actor_support
				.in_set(ActorSupportSystems::Hold)
				.before(TransformSystems::Propagate),
		)
		.add_systems(
			FixedPostUpdate,
			hold_actor_support
				.in_set(ActorSupportSystems::Hold)
				.after(PhysicsSystems::Prepare)
				.before(PhysicsSystems::StepSimulation),
		);
}

fn resolve_actor_support(
	mut commands: Commands,
	spatial: Option<SpatialQuery>,
	locomotion: Option<Res<CharacterLocomotion>>,
	collider_names: Query<(Option<&Name>, Has<durham::TerrainTrimeshCollider>)>,
	mut actors: Query<(
		Entity,
		&mut PendingActorSupport,
		&mut Transform,
		Option<&RigidBody>,
		Option<&Collider>,
		Option<&GravityScale>,
		Option<&mut LinearVelocity>,
		Option<&mut AngularVelocity>,
	)>,
) {
	let Some(spatial) = spatial else {
		for (entity, mut pending, _, body, collider, _, _, _) in &mut actors {
			pending.diagnose(
				entity,
				SupportDiagnostic::MissingSpatialQuery,
				body.copied(),
				collider.is_some(),
			);
		}
		return;
	};
	let locomotion = locomotion.as_deref().copied().unwrap_or_default();
	let filter = PhysicsInteractionLayer::Fixed.query_filter();
	for (entity, mut pending, mut transform, body, collider, gravity, velocity, angular) in
		&mut actors
	{
		if body != Some(&RigidBody::Dynamic) || collider.is_none() {
			pending.diagnose(
				entity,
				SupportDiagnostic::BodyNotReady {
					body: body.copied(),
					collider: collider.is_some(),
				},
				body.copied(),
				collider.is_some(),
			);
			continue;
		}
		pending.resume_gravity.get_or_insert_with(|| gravity.map(|gravity| gravity.0));
		if pending.exhausted {
			continue;
		}
		if !pending.enabled {
			pending.diagnose(entity, SupportDiagnostic::Disabled, body.copied(), true);
			continue;
		}
		let Some(search) = pending.search else {
			pending.diagnose(entity, SupportDiagnostic::MissingSearch, body.copied(), true);
			continue;
		};
		match probe_search_with_locomotion(&spatial, &filter, search, &locomotion) {
			SupportProbeOutcome::Waiting => {
				pending.failure = None;
				pending.diagnose(
					entity,
					SupportDiagnostic::WaitingForPresentation,
					body.copied(),
					true,
				);
			}
			SupportProbeOutcome::Failed(reason) => {
				pending.failure = Some(reason);
				if pending.diagnose(entity, SupportDiagnostic::Failed(reason), body.copied(), true)
				{
					pending.diagnose_geometry(
						entity,
						&spatial,
						&filter,
						&collider_names,
						&locomotion,
					);
				}
				pending.advance_after_failure();
			}
			SupportProbeOutcome::Ready { mut pose, support } => {
				if let Some(minimum) = pending.minimum_center_y {
					pose.position.y = pose.position.y.max(minimum);
				}
				// A raised authored offset needs clearance at its final position,
				// not just at the lower contact accepted by the support cast.
				if clearance_blocked(
					&spatial,
					&filter,
					pose.position,
					pending.hull,
					support.map(|hit| hit.entity),
				) {
					pending.failure = Some(SupportProbeFailure::Clearance);
					if pending.diagnose(
						entity,
						SupportDiagnostic::FinalPoseClearance,
						body.copied(),
						true,
					) {
						pending.diagnose_geometry(
							entity,
							&spatial,
							&filter,
							&collider_names,
							&locomotion,
						);
					}
					pending.advance_after_failure();
					continue;
				}
				pending.diagnose(
					entity,
					SupportDiagnostic::Ready {
						position: pose.position,
						support: support.map(|hit| hit.entity),
					},
					body.copied(),
					true,
				);
				transform.translation = pose.position;
				if let Some(mut velocity) = velocity {
					velocity.0 = Vec3::ZERO;
				}
				if let Some(mut angular) = angular {
					angular.0 = Vec3::ZERO;
				}
				let mut actor = commands.entity(entity);
				actor.try_insert(Position(pose.position));
				match pending.resume_gravity.flatten() {
					Some(gravity) => {
						actor.try_insert(GravityScale(gravity));
					}
					None => {
						actor.remove::<GravityScale>();
					}
				}
				actor.remove::<PendingActorSupport>();
			}
		}
	}
}

fn hold_actor_support(
	mut commands: Commands,
	mut actors: Query<(
		Entity,
		&mut PendingActorSupport,
		&mut Transform,
		Option<&mut Position>,
		Option<&RigidBody>,
		Option<&mut GravityScale>,
		Option<&mut LinearVelocity>,
		Option<&mut AngularVelocity>,
	)>,
) {
	for (entity, mut pending, mut transform, position, body, gravity, velocity, angular) in
		&mut actors
	{
		transform.translation = pending.held_position;
		if let Some(mut position) = position {
			position.0 = pending.held_position;
		}
		if let Some(mut velocity) = velocity {
			velocity.0 = Vec3::ZERO;
		}
		if let Some(mut angular) = angular {
			angular.0 = Vec3::ZERO;
		}
		if body == Some(&RigidBody::Dynamic) {
			pending
				.resume_gravity
				.get_or_insert_with(|| gravity.as_ref().map(|gravity| gravity.0));
			if let Some(mut gravity) = gravity {
				gravity.0 = 0.0;
			} else {
				commands.entity(entity).try_insert(GravityScale(0.0));
			}
		}
	}
}

#[cfg(test)]
mod tests;
