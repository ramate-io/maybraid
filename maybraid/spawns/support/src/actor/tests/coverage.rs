//! Terrain presentation must cover the whole spawn footprint before release.
use super::{actor, app};
use crate::actor::handoff::{PoseRejection, SupportGeometry};
use crate::{PendingActorSupport, PlacementSearch, PlacementSupportHit};
use avian3d::prelude::*;
use bevy::{ecs::system::SystemState, prelude::*};
use characters::LocomotionCapsule;
use chunk::cascade::CascadeChunk;
use lod_avian::PhysicsInteractionLayer;

fn tile(app: &mut App, min: Vec2, max: Vec2, height: f32) -> Entity {
	let origin = Vec3::new(min.x, -10.0, min.y);
	let extent = Vec3::new(max.x - min.x, 20.0, max.y - min.y);
	app.world_mut()
		.spawn((
			Transform::from_translation(origin),
			RigidBody::Static,
			Collider::trimesh(
				vec![
					Vec3::new(0.0, height + 10.0, 0.0),
					Vec3::new(extent.x, height + 10.0, 0.0),
					Vec3::new(extent.x, height + 10.0, extent.z),
					Vec3::new(0.0, height + 10.0, extent.z),
				],
				vec![[0, 2, 1], [0, 3, 2]],
			),
			PhysicsInteractionLayer::fixed_layers(),
			durham::TerrainTrimeshCollider,
			CascadeChunk { origin, size: extent.max_element(), extent: Some(extent), ..default() },
		))
		.id()
}

fn clear_pose(app: &mut App, retained: Entity) -> anyhow::Result<Result<Vec3, PoseRejection>> {
	let search = PlacementSearch::outdoor_hint(Vec2::ZERO, 0.0, LocomotionCapsule::HUMANOID);
	let mut state = SystemState::<SupportGeometry>::new(app.world_mut());
	let geometry = state.get(app.world())?;
	Ok(geometry.clear_pose(
		search.standing_at(0.0),
		Some(PlacementSupportHit::new(retained, Vec3::ZERO, Vec3::Y)),
		search,
		&PhysicsInteractionLayer::animated_layers(),
	))
}

const QUADRANTS: [(f32, f32, f32, f32); 4] =
	[(-8.0, 0.0, 0.0, 8.0), (-8.0, 0.0, -8.0, 0.0), (0.0, 8.0, 0.0, 8.0), (0.0, 8.0, -8.0, 0.0)];

#[test]
fn one_two_and_three_ready_tiles_wait_without_advancing_then_four_release() -> anyhow::Result<()> {
	let mut app = app();
	let entity = actor(&mut app, Vec3::Y * 20.0, Some(1.25));
	if let Some(mut pending) = app.world_mut().get_mut::<PendingActorSupport>(entity) {
		*pending = PendingActorSupport::new(Vec3::Y * 20.0, LocomotionCapsule::HUMANOID, None)
			.with_nearby_terrain(8.0);
		let hull = pending.hull();
		pending.observe_search(Some(PlacementSearch::outdoor_hint(Vec2::ZERO, 0.0, hull)));
	}
	for (index, (left, right, back, front)) in QUADRANTS.into_iter().enumerate() {
		tile(&mut app, Vec2::new(left, back), Vec2::new(right, front), 0.0);
		for _ in 0..8 {
			app.update();
		}
		if index < 3 {
			let pending = app
				.world()
				.get::<PendingActorSupport>(entity)
				.ok_or_else(|| anyhow::anyhow!("released with {} tiles", index + 1))?;
			assert_eq!(pending.candidate, 0);
			assert_eq!(pending.failure, None);
			assert!(!pending.exhausted());
			assert_eq!(
				app.world().get::<Position>(entity).map(|position| position.0),
				Some(Vec3::Y * 20.0)
			);
		}
	}
	assert!(app.world().get::<PendingActorSupport>(entity).is_none());
	assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(1.25));
	Ok(())
}

#[test]
fn ineligible_or_moved_neighbor_does_not_count_as_presented_terrain() -> anyhow::Result<()> {
	for case in 0..8 {
		let mut app = app();
		let tiles = QUADRANTS.map(|(left, right, back, front)| {
			tile(&mut app, Vec2::new(left, back), Vec2::new(right, front), 0.0)
		});
		for _ in 0..3 {
			app.update();
		}
		let neighbor = tiles[3];
		match case {
			0 => {
				app.world_mut().entity_mut(neighbor).insert(Sensor);
			}
			1 => {
				app.world_mut().entity_mut(neighbor).insert(ColliderDisabled);
			}
			2 => {
				app.world_mut().entity_mut(neighbor).insert(RigidBodyDisabled);
			}
			3 => {
				app.world_mut().entity_mut(neighbor).insert(CollisionLayers::NONE);
			}
			4 => {
				let parent = app.world_mut().spawn(RigidBodyDisabled).id();
				app.world_mut().entity_mut(neighbor).insert(ColliderOf { body: parent });
			}
			5 => {
				app.world_mut().entity_mut(neighbor).insert(Position(Vec3::splat(100.0)));
			}
			6 => {
				app.world_mut()
					.entity_mut(neighbor)
					.insert(Rotation(Quat::from_rotation_y(0.1)));
			}
			_ => {
				app.world_mut()
					.get_mut::<Collider>(neighbor)
					.ok_or_else(|| anyhow::anyhow!("missing collider"))?
					.set_scale(Vec3::splat(2.0), 10);
			}
		}
		assert_eq!(
			clear_pose(&mut app, tiles[0])?,
			Err(PoseRejection::TerrainCoverage),
			"case={case}"
		);
	}
	Ok(())
}

#[test]
fn fully_presented_cliff_does_not_require_floor_across_the_whole_footprint() -> anyhow::Result<()> {
	let mut app = app();
	let support = tile(&mut app, Vec2::new(-8.0, -8.0), Vec2::new(0.1, 8.0), 0.0);
	tile(&mut app, Vec2::new(0.1, -8.0), Vec2::new(8.0, 8.0), -8.0);
	for _ in 0..3 {
		app.update();
	}
	assert!(clear_pose(&mut app, support)?.is_ok());
	Ok(())
}

#[test]
fn footprint_detects_an_interior_hole_between_center_and_corner_probes() -> anyhow::Result<()> {
	let mut app = app();
	let support = tile(&mut app, Vec2::splat(-8.0), Vec2::splat(8.0), 0.0);
	app.world_mut()
		.get_mut::<CascadeChunk>(support)
		.ok_or_else(|| anyhow::anyhow!("missing chunk"))?
		.omit = Some(bevy::math::bounding::Aabb3d::new(
		Vec3::new(0.2, 0.0, 0.2),
		Vec3::new(0.05, 10.0, 0.05),
	));
	for _ in 0..3 {
		app.update();
	}
	assert_eq!(clear_pose(&mut app, support)?, Err(PoseRejection::TerrainCoverage));
	Ok(())
}

#[test]
fn losing_neighbor_coverage_during_settling_reholds_without_advancing() -> anyhow::Result<()> {
	let mut app = app();
	let tiles = QUADRANTS.map(|(left, right, back, front)| {
		tile(&mut app, Vec2::new(left, back), Vec2::new(right, front), 0.0)
	});
	for _ in 0..3 {
		app.update();
	}
	let entity = actor(&mut app, Vec3::Y * 20.0, Some(1.25));
	if let Some(mut pending) = app.world_mut().get_mut::<PendingActorSupport>(entity) {
		*pending = PendingActorSupport::new(Vec3::Y * 20.0, LocomotionCapsule::HUMANOID, None)
			.with_nearby_terrain(8.0);
		pending.observe_search(Some(PlacementSearch::outdoor_hint(
			Vec2::ZERO,
			0.0,
			LocomotionCapsule::HUMANOID,
		)));
	}
	app.update();
	assert!(app
		.world()
		.get::<PendingActorSupport>(entity)
		.is_some_and(|pending| pending.settling.is_some()));
	app.world_mut().entity_mut(tiles[3]).insert(ColliderDisabled);
	app.update();
	let pending = app
		.world()
		.get::<PendingActorSupport>(entity)
		.ok_or_else(|| anyhow::anyhow!("released with a missing neighbor"))?;
	assert!(pending.settling.is_none());
	assert_eq!(pending.failure, None);
	assert_eq!(pending.candidate, 0);
	assert_eq!(
		app.world().get::<Position>(entity).map(|position| position.0),
		Some(Vec3::Y * 20.0)
	);
	app.world_mut().entity_mut(tiles[3]).remove::<ColliderDisabled>();
	for _ in 0..8 {
		app.update();
	}
	assert!(app.world().get::<PendingActorSupport>(entity).is_none());
	assert_eq!(app.world().get::<GravityScale>(entity).map(|gravity| gravity.0), Some(1.25));
	Ok(())
}
