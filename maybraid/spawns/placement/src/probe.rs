//! Player-arrival adapter for the shared actor support probe.

use avian3d::prelude::SpatialQuery;
use bevy::prelude::*;
use characters::CharacterLocomotion;
use durham::DurhamSurface;
use lod::hcsg::HcsgStorage;
use lod_avian::PhysicsInteractionLayer;
use maybraid_spawns_player::SpawnFailureReason;
use maybraid_spawns_support::{
	probe_search_with_locomotion, terrain_height_at, SupportProbeFailure, SupportProbeOutcome,
};

use crate::{PlacementWork, SupportReadiness};

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

/// Downward support + standing-capsule clearance for every pending column.
/// Without [`SpatialQuery`], every request stays waiting.
pub fn observe_placement_physics(
	spatial: Option<SpatialQuery>,
	surface: Option<DurhamSurface>,
	storage: Option<Res<HcsgStorage>>,
	locomotion: Option<Res<CharacterLocomotion>>,
	mut work: ResMut<PlacementWork>,
) {
	let Some(spatial) = spatial else {
		for pending in work.pending_mut() {
			pending.observe(SupportReadiness::waiting());
		}
		return;
	};
	let locomotion = locomotion.as_deref().copied().unwrap_or_default();
	let filter = PhysicsInteractionLayer::Fixed.query_filter();
	for pending in work.pending_mut() {
		if pending.refreshes_terrain() {
			let height = surface.as_ref().and_then(|surface| {
				terrain_height_at(surface, storage.as_deref(), pending.search.xz)
			});
			let Some(height) = height else {
				pending.observe(SupportReadiness::waiting());
				continue;
			};
			pending.search =
				crate::PlacementSearch::outdoor_hint(pending.search.xz, height, pending.hull);
		}
		match probe_search_with_locomotion(&spatial, &filter, pending.search, &locomotion) {
			SupportProbeOutcome::Waiting => pending.observe(SupportReadiness::waiting()),
			SupportProbeOutcome::Ready { pose, support } => pending.accept(pose, support),
			SupportProbeOutcome::Failed(reason) => pending.fail(match reason {
				SupportProbeFailure::InvalidBounds => SpawnFailureReason::InvalidBounds,
				SupportProbeFailure::Clearance => SpawnFailureReason::Clearance,
				SupportProbeFailure::NoEligibleSupport => SpawnFailureReason::NoEligibleSupport,
				SupportProbeFailure::Placement => SpawnFailureReason::Placement,
			}),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::PlacementSearch;
	use avian3d::prelude::{Collider, PhysicsPlugins, RigidBody};
	use characters::LocomotionCapsule;
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
			"accepted {} want expected",
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
	fn generated_destination_retries_publication_and_late_collider_with_same_identity(
	) -> anyhow::Result<()> {
		use crate::ResolvePlacement;
		use bevy::ecs::system::RunSystemOnce;
		use durham::{
			BaseTerrainNoise, Terrain, TerrainCellLayout, TerrainConfig, WorldBaseTerrain,
		};
		use lod::gen::Id;
		use poi_intelligence::PoiRegistry;
		use std::sync::Arc;

		let mut app = physics_app();
		let layout = TerrainCellLayout::default();
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
		let terrain = Terrain::base_cell_for_test(&layout, 0, 0, base.clone());
		let point = Vec2::splat(8.0);
		let height = terrain.sdf.terrain().height_at_with_all_modulations(point.x, point.y);
		app.insert_resource(layout);
		app.insert_resource(WorldBaseTerrain(base));
		app.init_resource::<HcsgStorage>();
		app.init_resource::<PoiRegistry>();
		let request = ResolvePlacement::humanoid(
			identity(),
			attempt(),
			ArrivalDestination::Fallback,
			Vec3::new(point.x, 0.0, point.y),
		);
		let pending = app
			.world_mut()
			.run_system_once(move |registry: Res<PoiRegistry>, surface: DurhamSurface| {
				request.realize(&registry, &surface)
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(pending.refreshes_terrain());
		app.world_mut().resource_mut::<PlacementWork>().insert(pending);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(ready.is_empty());
		assert!(failed.is_empty());
		assert_eq!(app.world().resource::<PlacementWork>().len(), 1);

		let cell = terrain.cell;
		app.world()
			.resource::<HcsgStorage>()
			.publish(Id::from_cell(cell), Arc::new(terrain), cell);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(ready.is_empty(), "published height alone cannot release the player arrival");
		assert!(failed.is_empty());
		assert_eq!(app.world().resource::<PlacementWork>().len(), 1);
		let refreshed = app
			.world()
			.resource::<PlacementWork>()
			.get(identity())
			.ok_or_else(|| anyhow::anyhow!("placement identity was lost"))?;
		assert!((refreshed.search.hint_surface_y() - height).abs() < 1e-3);

		spawn_fixed(
			&mut app,
			Transform::from_xyz(point.x, height, point.y),
			Collider::cuboid(8.0, 0.2, 8.0),
		);
		let (ready, failed) = drain_outcomes(&mut app);
		assert!(failed.is_empty(), "{failed:?}");
		assert_eq!(ready.len(), 1);
		assert!(ready[0].matches_attempt(identity(), attempt()));
		assert!(
			(ready[0].position.y - height - LocomotionCapsule::HUMANOID.spawn_height()).abs()
				< 0.35
		);
		assert!(app.world().resource::<PlacementWork>().is_empty());
		Ok(())
	}

	#[test]
	fn bodyless_arrival_reads_the_same_configured_locomotion_slope() {
		for (degrees, accepted) in [(57.0_f32, true), (72.0_f32, false)] {
			let mut app = physics_app();
			app.insert_resource(CharacterLocomotion { max_slope_angle: 70.0_f32.to_radians() });
			spawn_fixed(
				&mut app,
				Transform::from_rotation(Quat::from_rotation_z(degrees.to_radians())),
				Collider::cuboid(12.0, 0.2, 12.0),
			);
			insert_search(
				&mut app,
				PlacementSearch::interior_at(Vec3::ZERO, LocomotionCapsule::HUMANOID),
			);
			let (ready, failed) = drain_outcomes(&mut app);
			assert_eq!(ready.len(), usize::from(accepted), "slope={degrees}");
			assert_eq!(failed.len(), usize::from(!accepted), "slope={degrees}");
			if !accepted {
				assert_eq!(failed[0].reason, SpawnFailureReason::Clearance);
			}
		}
	}

	#[test]
	fn unused_support_policy_is_recorded() {
		assert_eq!(SupportPolicy::FixedUpward.accepts_normal(Vec3::Y), true);
		assert!(!SupportPolicy::BuildingWalk.accepts_normal(Vec3::X));
	}
}
