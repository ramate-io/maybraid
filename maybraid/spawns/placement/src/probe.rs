//! Support and clearance against actual collider geometry.
//!
//! AABBs are not used as occupancy. A downward shape cast proves a floor;
//! a capsule intersection at the standing pose proves clearance. The
//! supporting surface is excluded from the obstruction set.

use avian3d::prelude::{Collider, ShapeCastConfig, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;
use maybraid_spawns_player::SpawnFailureReason;
use world_player::player_position_above_surface;

use crate::pending::PlacementWork;
use crate::request::{ActorClearance, PlacementPolicy};
use crate::support::SupportReadiness;
use crate::terrain::PlacementPose;

/// Interior authored floors must be hit within this vertical window.
pub const INTERIOR_FLOOR_TOLERANCE: f32 = 1.25;
const OUTDOOR_CAST_DISTANCE: f32 = 64.0;
const SUPPORT_PROBE_RADIUS: f32 = 0.25;

#[derive(Resource, Default)]
struct PlacementPhysics;

/// Observe each pending pose with [`SpatialQuery`]. Idempotent. Requires
/// `PhysicsPlugins` so the collider tree exists.
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
	Ready(PlacementPose),
	Blocked,
}

/// Downward support + standing-capsule clearance for every pending pose.
pub fn observe_placement_physics(spatial: SpatialQuery, mut work: ResMut<PlacementWork>) {
	let filter = PhysicsInteractionLayer::Fixed.query_filter();
	for pending in work.pending_mut() {
		let Some(pose) = pending.pose else {
			continue;
		};
		match probe_pose(&spatial, &filter, pose) {
			ProbeOutcome::Waiting => pending.observe(SupportReadiness::new(false, false)),
			ProbeOutcome::Ready(next) => {
				pending.pose = Some(next);
				pending.observe(SupportReadiness::new(true, true));
			}
			ProbeOutcome::Blocked => pending.fail(SpawnFailureReason::Clearance),
		}
	}
}

fn probe_pose(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	pose: PlacementPose,
) -> ProbeOutcome {
	match pose.policy {
		PlacementPolicy::Airborne => {
			if clearance_blocked(spatial, filter, pose.position, pose.clearance, None) {
				ProbeOutcome::Blocked
			} else {
				ProbeOutcome::Ready(pose)
			}
		}
		PlacementPolicy::OutdoorGround => {
			match find_support(spatial, filter, pose.position, OUTDOOR_CAST_DISTANCE) {
				None => occupancy_or_wait(spatial, filter, pose),
				Some(support) => finalize_standing(spatial, filter, pose, support),
			}
		}
		PlacementPolicy::InteriorFloor => {
			let authored = pose.position - Vec3::Y * pose.clearance.surface_lift();
			let origin = authored + Vec3::Y * 0.45;
			match find_support(spatial, filter, origin, INTERIOR_FLOOR_TOLERANCE + 0.5) {
				None => occupancy_or_wait(spatial, filter, pose),
				Some(support) => {
					if (authored.y - support.point.y).abs() > INTERIOR_FLOOR_TOLERANCE {
						occupancy_or_wait(spatial, filter, pose)
					} else {
						finalize_standing(spatial, filter, pose, support)
					}
				}
			}
		}
	}
}

fn occupancy_or_wait(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	pose: PlacementPose,
) -> ProbeOutcome {
	if clearance_blocked(spatial, filter, pose.position, pose.clearance, None) {
		ProbeOutcome::Blocked
	} else {
		ProbeOutcome::Waiting
	}
}

fn finalize_standing(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	mut pose: PlacementPose,
	support: SupportHit,
) -> ProbeOutcome {
	pose.position = player_position_above_surface(support.point);
	if clearance_blocked(spatial, filter, pose.position, pose.clearance, Some(support.entity)) {
		ProbeOutcome::Blocked
	} else {
		ProbeOutcome::Ready(pose)
	}
}

#[derive(Clone, Copy, Debug)]
struct SupportHit {
	entity: Entity,
	point: Vec3,
}

fn find_support(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	standing: Vec3,
	max_distance: f32,
) -> Option<SupportHit> {
	let probe = Collider::cylinder(SUPPORT_PROBE_RADIUS, 0.04);
	let config = ShapeCastConfig::from_max_distance(max_distance);
	let mut hits =
		spatial.shape_hits(&probe, standing, Quat::IDENTITY, Dir3::NEG_Y, 8, &config, filter);
	hits.sort_by(|left, right| left.distance.total_cmp(&right.distance));
	hits.into_iter()
		.find(|hit| hit.normal1.y > 0.55)
		.map(|hit| SupportHit { entity: hit.entity, point: hit.point1 })
}

fn clearance_blocked(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	standing: Vec3,
	clearance: ActorClearance,
	support: Option<Entity>,
) -> bool {
	let capsule = Collider::capsule(ActorClearance::RADIUS, clearance.height.max(0.05));
	spatial
		.shape_intersections(&capsule, standing, Quat::IDENTITY, filter)
		.into_iter()
		.any(|entity| support != Some(entity))
}

#[cfg(test)]
mod tests {
	use super::*;
	use avian3d::prelude::{Collider, PhysicsPlugins, RigidBody};
	use maybraid_spawns_player::{
		ArrivalDestination, SpawnIdentity, SpawnOwner, SpawnRequestId, SpawnSessionId,
	};

	use crate::pending::PendingPlacement;
	use crate::request::{PlacementFailed, PlacementReady};
	use crate::{register_placement_physics, PlacementPolicy, PlacementWork};

	fn identity() -> SpawnIdentity {
		SpawnIdentity::new(SpawnRequestId::new(1), SpawnOwner::new(SpawnSessionId::new(1)))
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

	fn insert_pose(app: &mut App, pose: PlacementPose) {
		app.world_mut().resource_mut::<PlacementWork>().insert(PendingPlacement::new(
			identity(),
			ArrivalDestination::Fallback,
			pose,
		));
	}

	fn drain_outcomes(app: &mut App) -> (Vec<PlacementReady>, Vec<PlacementFailed>) {
		app.update();
		app.update();
		let ready = app.world_mut().resource_mut::<Messages<PlacementReady>>().drain().collect();
		let failed = app.world_mut().resource_mut::<Messages<PlacementFailed>>().drain().collect();
		(ready, failed)
	}

	fn standing_above(floor: Vec3) -> Vec3 {
		player_position_above_surface(floor)
	}

	#[test]
	fn missing_collider_never_becomes_ready() {
		let mut app = physics_app();
		insert_pose(
			&mut app,
			PlacementPose::interior_at(Vec3::new(0.0, 4.0, 0.0), ActorClearance::HUMANOID),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(ready.is_empty());
		assert!(failed.is_empty());
		assert_eq!(app.world().resource::<PlacementWork>().len(), 1);
	}

	#[test]
	fn interior_void_is_not_ready() {
		let mut app = physics_app();
		// Ring floor: the authored point sits in the stairwell hole.
		spawn_fixed(&mut app, Transform::from_xyz(-3.0, 4.0, 0.0), Collider::cuboid(2.0, 0.2, 6.0));
		spawn_fixed(&mut app, Transform::from_xyz(3.0, 4.0, 0.0), Collider::cuboid(2.0, 0.2, 6.0));
		insert_pose(
			&mut app,
			PlacementPose::interior_at(Vec3::new(0.0, 4.0, 0.0), ActorClearance::HUMANOID),
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
		insert_pose(&mut app, PlacementPose::interior_at(Vec3::ZERO, ActorClearance::HUMANOID));
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(ready.is_empty());
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
		// Thin wall rotated 45°. Its unrotated AABB covers the origin; the
		// actual slab does not.
		spawn_fixed(
			&mut app,
			Transform {
				translation: Vec3::new(4.0, 2.0, 4.0),
				rotation: Quat::from_rotation_y(std::f32::consts::FRAC_PI_4),
				scale: Vec3::ONE,
			},
			Collider::cuboid(0.3, 4.0, 8.0),
		);
		insert_pose(
			&mut app,
			PlacementPose {
				position: standing_above(Vec3::ZERO),
				policy: PlacementPolicy::OutdoorGround,
				clearance: ActorClearance::HUMANOID,
			},
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(failed.is_empty(), "AABB overlap is not occupancy: {failed:?}");
		assert_eq!(ready.len(), 1, "{ready:?}");

		insert_pose(
			&mut app,
			PlacementPose {
				position: standing_above(Vec3::new(4.0, 0.0, 4.0)),
				policy: PlacementPolicy::OutdoorGround,
				clearance: ActorClearance::HUMANOID,
			},
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(ready.is_empty(), "standing in the rotated wall must fail");
		assert_eq!(failed.len(), 1);
		assert_eq!(failed[0].reason, SpawnFailureReason::Clearance);
	}

	#[test]
	fn outdoor_spot_inside_a_building_aabb_is_accepted() {
		let mut app = physics_app();
		spawn_fixed(
			&mut app,
			Transform::from_xyz(0.0, 0.0, 0.0),
			Collider::cuboid(20.0, 0.2, 20.0),
		);
		// Tall building volume whose AABB covers the origin, offset so the
		// standing capsule does not intersect the collider.
		spawn_fixed(&mut app, Transform::from_xyz(12.0, 8.0, 0.0), Collider::cuboid(8.0, 8.0, 8.0));
		insert_pose(
			&mut app,
			PlacementPose {
				position: standing_above(Vec3::ZERO),
				policy: PlacementPolicy::OutdoorGround,
				clearance: ActorClearance::HUMANOID,
			},
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(failed.is_empty());
		assert_eq!(ready.len(), 1);
		assert!(ready[0].support_ready);
	}

	#[test]
	fn interior_floor_releases_when_the_surface_exists() {
		let mut app = physics_app();
		spawn_fixed(&mut app, Transform::from_xyz(0.0, 12.0, 0.0), Collider::cuboid(6.0, 0.2, 6.0));
		insert_pose(
			&mut app,
			PlacementPose::interior_at(Vec3::new(0.0, 12.0, 0.0), ActorClearance::HUMANOID),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(failed.is_empty());
		assert_eq!(ready.len(), 1);
		assert!((ready[0].position.y - standing_above(Vec3::new(0.0, 12.0, 0.0)).y).abs() < 0.35);
	}
}
