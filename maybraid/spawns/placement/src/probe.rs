//! Bounded downward shape cast + standing-capsule clearance.
//!
//! Interior and outdoor share this probe. Generated heights define the
//! search column; collider geometry determines the accepted pose. The
//! first ineligible hit fails — the cast does not punch through a wall
//! or roof just because it is unsuitable for standing.

use avian3d::prelude::{Collider, ShapeCastConfig, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use characters::LocomotionCapsule;
use lod_avian::PhysicsInteractionLayer;
use maybraid_spawns_player::SpawnFailureReason;

use crate::pending::PlacementWork;
use crate::request::{PlacementPolicy, PlacementSupportHit};
use crate::support::{column_presentation_ready, SupportReadiness};
use crate::terrain::{PlacementPose, PlacementSearch};

const MAX_HITS: u32 = 8;
const PROBE_HALF_HEIGHT: f32 = 0.02;
const START_LIFT: f32 = 0.05;

#[derive(Resource, Default)]
struct PlacementPhysics;

/// Observe each pending column with [`SpatialQuery`]. Idempotent. A missing
/// collider tree is presentation-not-ready: retain the request, construct
/// nothing. Headless stacks can register this without `PhysicsPlugins`.
pub fn register_placement_physics(app: &mut App) {
	if app.world().contains_resource::<PlacementPhysics>() {
		return;
	}
	crate::register_placement_runtime(app);
	app.init_resource::<PlacementPhysics>()
		.add_systems(Update, observe_placement_physics.in_set(crate::PlacementSystems::Observe));
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ProbeOutcome {
	Waiting,
	Ready { pose: PlacementPose, support: Option<PlacementSupportHit> },
	Failed(SpawnFailureReason),
}

/// Downward support + standing-capsule clearance for every pending column.
/// Without [`SpatialQuery`], every request stays waiting.
pub fn observe_placement_physics(spatial: Option<SpatialQuery>, mut work: ResMut<PlacementWork>) {
	let Some(spatial) = spatial else {
		for pending in work.pending_mut() {
			pending.observe(SupportReadiness::waiting());
		}
		return;
	};
	let filter = PhysicsInteractionLayer::Fixed.query_filter();
	for pending in work.pending_mut() {
		match probe_search(&spatial, &filter, pending.search) {
			ProbeOutcome::Waiting => pending.observe(SupportReadiness::waiting()),
			ProbeOutcome::Ready { pose, support } => pending.accept(pose, support),
			ProbeOutcome::Failed(reason) => pending.fail(reason),
		}
	}
}

fn probe_search(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	search: PlacementSearch,
) -> ProbeOutcome {
	if !search.bounds_valid() {
		return ProbeOutcome::Failed(SpawnFailureReason::InvalidBounds);
	}
	if search.policy == PlacementPolicy::Airborne {
		let pose = PlacementPose::new(
			Vec3::new(search.xz.x, search.max_surface_y, search.xz.y),
			search.policy,
			search.hull,
		);
		return if clearance_blocked(spatial, filter, pose.position, search.hull, None) {
			ProbeOutcome::Failed(SpawnFailureReason::Clearance)
		} else {
			ProbeOutcome::Ready { pose, support: None }
		};
	}
	if !column_presentation_ready(spatial, filter, search) {
		return ProbeOutcome::Waiting;
	}
	cast_column(spatial, filter, search)
}

fn cast_column(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	search: PlacementSearch,
) -> ProbeOutcome {
	let origin = support_origin(search);
	if start_overlaps(spatial, filter, origin, search.hull) {
		return ProbeOutcome::Failed(SpawnFailureReason::Clearance);
	}
	let probe = support_probe(search.hull);
	let config = ShapeCastConfig::from_max_distance(search.cast_distance().max(0.1));
	let mut hits =
		spatial.shape_hits(&probe, origin, Quat::IDENTITY, Dir3::NEG_Y, MAX_HITS, &config, filter);
	hits.sort_by(|left, right| left.distance.total_cmp(&right.distance));
	let Some(hit) = hits.into_iter().next() else {
		return ProbeOutcome::Failed(SpawnFailureReason::NoEligibleSupport);
	};
	if !search.support.accepts_normal(hit.normal1) || !search.contains_surface(hit.point1.y) {
		return ProbeOutcome::Failed(if search.support.accepts_normal(hit.normal1) {
			SpawnFailureReason::NoEligibleSupport
		} else {
			SpawnFailureReason::Clearance
		});
	}
	let standing = search.standing_at(hit.point1.y);
	if (standing.xz() - search.xz).length() > 1e-3 {
		return ProbeOutcome::Failed(SpawnFailureReason::Placement);
	}
	if clearance_blocked(spatial, filter, standing, search.hull, Some(hit.entity)) {
		return ProbeOutcome::Failed(SpawnFailureReason::Clearance);
	}
	ProbeOutcome::Ready {
		pose: PlacementPose::new(standing, search.policy, search.hull),
		support: Some(PlacementSupportHit::new(hit.entity, hit.point1, hit.normal1)),
	}
}

fn support_origin(search: PlacementSearch) -> Vec3 {
	Vec3::new(search.xz.x, search.max_surface_y + START_LIFT, search.xz.y)
}

fn start_overlaps(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	origin: Vec3,
	hull: LocomotionCapsule,
) -> bool {
	!spatial
		.shape_intersections(&support_probe(hull), origin, Quat::IDENTITY, filter)
		.is_empty()
}

fn support_probe(hull: LocomotionCapsule) -> Collider {
	Collider::cylinder((hull.radius * 0.5).max(0.08), PROBE_HALF_HEIGHT * 2.0)
}

fn hull_collider(hull: LocomotionCapsule) -> Collider {
	Collider::capsule(hull.radius.max(0.01), hull.length.max(0.01))
}

fn clearance_blocked(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	standing: Vec3,
	hull: LocomotionCapsule,
	support: Option<Entity>,
) -> bool {
	spatial
		.shape_intersections(&hull_collider(hull), standing, Quat::IDENTITY, filter)
		.into_iter()
		.any(|entity| support != Some(entity))
}

#[cfg(test)]
mod tests {
	use super::*;
	use avian3d::prelude::{Collider, PhysicsPlugins, RigidBody};
	use maybraid_spawns_player::{
		ArrivalDestination, PlacementAttempt, SpawnIdentity, SpawnOwner, SpawnRequestId,
		SpawnSessionId,
	};

	use crate::pending::PendingPlacement;
	use crate::request::{PlacementFailed, PlacementReady, SupportPolicy};
	use crate::{register_placement_physics, PlacementWork};

	fn identity() -> SpawnIdentity {
		SpawnIdentity::new(SpawnRequestId::new(1), SpawnOwner::new(SpawnSessionId::new(1)))
	}

	fn attempt() -> PlacementAttempt {
		PlacementAttempt::new(1)
	}

	fn physics_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			TransformPlugin,
			PhysicsPlugins::default(),
			bevy::asset::AssetPlugin::default(),
			bevy::mesh::MeshPlugin,
		));
		register_placement_physics(&mut app);
		app.finish();
		app
	}

	fn spawn_fixed(app: &mut App, transform: Transform, collider: Collider) -> Entity {
		app.world_mut()
			.spawn((
				transform,
				RigidBody::Static,
				collider,
				PhysicsInteractionLayer::fixed_layers(),
			))
			.id()
	}

	fn insert_search(app: &mut App, search: PlacementSearch) {
		app.world_mut().resource_mut::<PlacementWork>().insert(PendingPlacement::new(
			identity(),
			attempt(),
			ArrivalDestination::Fallback,
			search,
		));
	}

	fn drain_outcomes(app: &mut App) -> (Vec<PlacementReady>, Vec<PlacementFailed>) {
		app.update();
		app.update();
		let ready = app.world_mut().resource_mut::<Messages<PlacementReady>>().drain().collect();
		let failed = app.world_mut().resource_mut::<Messages<PlacementFailed>>().drain().collect();
		(ready, failed)
	}

	#[test]
	fn missing_spatial_query_keeps_the_request() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_placement_physics(&mut app);
		app.finish();
		insert_search(
			&mut app,
			PlacementSearch::outdoor_hint(Vec2::ZERO, 0.0, LocomotionCapsule::HUMANOID),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(ready.is_empty());
		assert!(failed.is_empty());
		assert_eq!(app.world().resource::<PlacementWork>().len(), 1);
	}

	#[test]
	fn missing_collision_presentation_keeps_the_request() {
		let mut app = physics_app();
		insert_search(
			&mut app,
			PlacementSearch::interior_at(Vec3::new(0.0, 4.0, 0.0), LocomotionCapsule::HUMANOID),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(ready.is_empty());
		assert!(failed.is_empty());
		assert_eq!(app.world().resource::<PlacementWork>().len(), 1);
	}

	#[test]
	fn lower_terrain_is_not_accepted_while_the_floor_is_pending() {
		let mut app = physics_app();
		spawn_fixed(
			&mut app,
			Transform::from_xyz(0.0, 0.0, 0.0),
			Collider::cuboid(20.0, 0.2, 20.0),
		);
		insert_search(
			&mut app,
			PlacementSearch::interior_at(Vec3::new(0.0, 12.0, 0.0), LocomotionCapsule::HUMANOID),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(ready.is_empty(), "terrain under the storey is not the interior floor: {ready:?}");
		assert!(failed.is_empty());
		assert_eq!(app.world().resource::<PlacementWork>().len(), 1);
	}

	#[test]
	fn outdoor_lands_on_the_collider_when_generated_height_differs() {
		let mut app = physics_app();
		spawn_fixed(&mut app, Transform::from_xyz(0.0, 4.0, 0.0), Collider::cuboid(8.0, 0.2, 8.0));
		insert_search(
			&mut app,
			PlacementSearch::outdoor_hint(Vec2::ZERO, 0.0, LocomotionCapsule::HUMANOID),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(failed.is_empty(), "{failed:?}");
		assert_eq!(ready.len(), 1);
		let expected = LocomotionCapsule::HUMANOID.spawn_height() + 4.2;
		assert!(
			(ready[0].position.y - expected).abs() < 0.35,
			"accepted {} want ~{expected}",
			ready[0].position.y
		);
		assert_eq!(ready[0].position.xz(), Vec2::ZERO);
		assert!(ready[0].matches_attempt(identity(), attempt()));
	}

	#[test]
	fn interior_uses_the_same_probe_and_respects_the_upper_bound() {
		let mut app = physics_app();
		spawn_fixed(&mut app, Transform::from_xyz(0.0, 12.0, 0.0), Collider::cuboid(6.0, 0.2, 6.0));
		spawn_fixed(&mut app, Transform::from_xyz(0.0, 16.0, 0.0), Collider::cuboid(6.0, 0.2, 6.0));
		insert_search(
			&mut app,
			PlacementSearch::interior_at(Vec3::new(0.0, 12.0, 0.0), LocomotionCapsule::HUMANOID),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(failed.is_empty(), "{failed:?}");
		assert_eq!(ready.len(), 1);
		assert!(
			(ready[0].position.y - (12.2 + LocomotionCapsule::HUMANOID.spawn_height())).abs()
				< 0.35
		);
		assert!(ready[0].position.y < 15.0, "must not stand on the storey above");
	}

	#[test]
	fn interior_void_is_not_ready() {
		let mut app = physics_app();
		spawn_fixed(&mut app, Transform::from_xyz(-3.0, 4.0, 0.0), Collider::cuboid(2.0, 0.2, 6.0));
		spawn_fixed(&mut app, Transform::from_xyz(3.0, 4.0, 0.0), Collider::cuboid(2.0, 0.2, 6.0));
		insert_search(
			&mut app,
			PlacementSearch::interior_at(Vec3::new(0.0, 4.0, 0.0), LocomotionCapsule::HUMANOID),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(ready.is_empty(), "a hole inside the building envelope is not a floor");
		assert!(failed.is_empty());
	}

	#[test]
	fn blocked_headroom_fails_clearance() {
		let mut app = physics_app();
		spawn_fixed(&mut app, Transform::from_xyz(0.0, 0.0, 0.0), Collider::cuboid(8.0, 0.2, 8.0));
		spawn_fixed(&mut app, Transform::from_xyz(0.0, 1.6, 0.0), Collider::cuboid(8.0, 0.15, 8.0));
		insert_search(
			&mut app,
			PlacementSearch::interior_at(Vec3::ZERO, LocomotionCapsule::HUMANOID),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(ready.is_empty());
		assert_eq!(failed.len(), 1);
		assert_eq!(failed[0].reason, SpawnFailureReason::Clearance);
	}

	#[test]
	fn a_wall_in_the_column_fails_instead_of_punching_through() {
		let mut app = physics_app();
		spawn_fixed(&mut app, Transform::from_xyz(0.0, 0.0, 0.0), Collider::cuboid(8.0, 0.2, 8.0));
		// Offset so the support disk hits the floor and the standing hull does not.
		spawn_fixed(&mut app, Transform::from_xyz(0.5, 1.2, 0.0), Collider::cuboid(0.2, 1.2, 2.0));
		insert_search(
			&mut app,
			PlacementSearch::outdoor_hint(Vec2::ZERO, 0.0, LocomotionCapsule::HUMANOID),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(ready.is_empty(), "must not release a capsule inside the wall: {ready:?}");
		assert_eq!(failed.len(), 1);
		assert_eq!(failed[0].reason, SpawnFailureReason::Clearance);
	}

	#[test]
	fn rotated_building_classifies_geometry_not_aabb() {
		let mut app = physics_app();
		spawn_fixed(
			&mut app,
			Transform::from_xyz(0.0, 0.0, 0.0),
			Collider::cuboid(20.0, 0.2, 20.0),
		);
		spawn_fixed(
			&mut app,
			Transform {
				translation: Vec3::new(4.0, 10.0, 4.0),
				rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_4),
				scale: Vec3::ONE,
			},
			Collider::cuboid(0.3, 12.0, 8.0),
		);
		insert_search(
			&mut app,
			PlacementSearch::outdoor_hint(Vec2::ZERO, 0.0, LocomotionCapsule::HUMANOID),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(failed.is_empty(), "AABB overlap is not occupancy: {failed:?}");
		assert_eq!(ready.len(), 1, "{ready:?}");
	}

	#[test]
	fn a_narrow_opening_fails_the_configured_capsule() {
		let mut app = physics_app();
		spawn_fixed(&mut app, Transform::from_xyz(0.0, 0.0, 0.0), Collider::cuboid(8.0, 0.2, 8.0));
		spawn_fixed(&mut app, Transform::from_xyz(-0.7, 1.2, 0.0), Collider::cuboid(0.2, 1.2, 2.0));
		spawn_fixed(&mut app, Transform::from_xyz(0.7, 1.2, 0.0), Collider::cuboid(0.2, 1.2, 2.0));
		insert_search(
			&mut app,
			PlacementSearch::outdoor_hint(Vec2::ZERO, 0.0, LocomotionCapsule::HUMANOID),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(failed.is_empty(), "stock humanoid fits the 1.0 m gap: {failed:?}");
		assert_eq!(ready.len(), 1);

		let wide = LocomotionCapsule::HUMANOID.scaled(2.0);
		insert_search(&mut app, PlacementSearch::outdoor_hint(Vec2::ZERO, 0.0, wide));
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(ready.is_empty(), "the configured hull does not fit: {ready:?}");
		assert_eq!(failed.len(), 1);
		assert_eq!(failed[0].reason, SpawnFailureReason::Clearance);
	}

	#[test]
	fn actual_capsule_dims_determine_clearance() {
		let mut app = physics_app();
		spawn_fixed(&mut app, Transform::from_xyz(0.0, 0.0, 0.0), Collider::cuboid(8.0, 0.2, 8.0));
		spawn_fixed(&mut app, Transform::from_xyz(0.0, 3.2, 0.0), Collider::cuboid(8.0, 0.15, 8.0));
		insert_search(
			&mut app,
			PlacementSearch::interior_at(Vec3::ZERO, LocomotionCapsule::HUMANOID),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(failed.is_empty(), "stock humanoid fits under a 3.2 m slab: {failed:?}");
		assert_eq!(ready.len(), 1);

		let tall = LocomotionCapsule::HUMANOID.with_half_height(2.4);
		insert_search(&mut app, PlacementSearch::interior_at(Vec3::ZERO, tall));
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(ready.is_empty(), "a taller hull must fail the same opening");
		assert_eq!(failed.len(), 1);
		assert_eq!(failed[0].reason, SpawnFailureReason::Clearance);
	}

	#[test]
	fn interior_floor_releases_when_the_surface_exists() {
		let mut app = physics_app();
		spawn_fixed(&mut app, Transform::from_xyz(0.0, 12.0, 0.0), Collider::cuboid(6.0, 0.2, 6.0));
		insert_search(
			&mut app,
			PlacementSearch::interior_at(Vec3::new(0.0, 12.0, 0.0), LocomotionCapsule::HUMANOID),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(failed.is_empty());
		assert_eq!(ready.len(), 1);
		assert!(ready[0].support_ready);
		assert!(ready[0].support.is_some());
	}

	#[test]
	fn unused_support_policy_is_recorded() {
		assert_eq!(SupportPolicy::FixedUpward.accepts_normal(Vec3::Y), true);
		assert!(!SupportPolicy::BuildingWalk.accepts_normal(Vec3::X));
	}
}
