//! Terrain is an open triangle surface, unlike the solid floors in other fixtures.

use super::{actor, app};
use avian3d::prelude::*;
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;

use crate::{PendingActorSupport, PlacementSearch};

#[test]
fn open_terrain_mesh_and_four_tile_seam_complete_physics_handoff() -> anyhow::Result<()> {
	for tiled in [false, true] {
		let mut app = app();
		let ranges = if tiled {
			vec![
				(-8.0, 0.0, -8.0, 0.0),
				(0.0, 8.0, -8.0, 0.0),
				(-8.0, 0.0, 0.0, 8.0),
				(0.0, 8.0, 0.0, 8.0),
			]
		} else {
			vec![(-8.0, 8.0, -8.0, 8.0)]
		};
		for (left, right, back, front) in ranges {
			app.world_mut().spawn((
				Transform::IDENTITY,
				RigidBody::Static,
				Collider::trimesh(
					vec![
						Vec3::new(left, 0.0, back),
						Vec3::new(right, 0.0, back),
						Vec3::new(right, 0.0, front),
						Vec3::new(left, 0.0, front),
					],
					vec![[0, 2, 1], [0, 3, 2]],
				),
				PhysicsInteractionLayer::fixed_layers(),
			));
		}
		for _ in 0..3 {
			app.update();
		}
		let entity = actor(&mut app, Vec3::Y * 20.0, Some(1.25));
		if let Some(mut pending) = app.world_mut().get_mut::<PendingActorSupport>(entity) {
			let hull = pending.hull();
			pending.observe_search(Some(PlacementSearch::outdoor_hint(Vec2::ZERO, 0.0, hull)));
		}
		for _ in 0..30 {
			app.update();
			if app.world().get::<PendingActorSupport>(entity).is_none() {
				break;
			}
		}
		assert!(
			app.world().get::<PendingActorSupport>(entity).is_none(),
			"tiled={tiled}, pending={:?}",
			app.world().get::<PendingActorSupport>(entity)
		);
		let position = app
			.world()
			.get::<Position>(entity)
			.ok_or_else(|| anyhow::anyhow!("missing actor"))?
			.0;
		assert!(position.y > 0.8 && position.y < 1.1, "tiled={tiled}, position={position:?}");
	}
	Ok(())
}

#[test]
fn full_capsule_can_contact_a_neighboring_creased_terrain_triangle() -> anyhow::Result<()> {
	let mut app = app();
	let origin = Vec3::new(16.0, 3.25, -32.0);
	// The center probe (radius .2) sees the flat face. The full capsule
	// (radius .4) also reaches the adjacent 68-degree bank at x=.21.
	let crease = 0.21;
	let high = (8.0 - crease) * 2.5;
	app.world_mut().spawn((
		Transform::from_translation(origin),
		RigidBody::Static,
		Collider::trimesh_with_config(
			vec![
				Vec3::new(-8.0, 0.0, -8.0),
				Vec3::new(crease, 0.0, -8.0),
				Vec3::new(8.0, high, -8.0),
				Vec3::new(-8.0, 0.0, 8.0),
				Vec3::new(crease, 0.0, 8.0),
				Vec3::new(8.0, high, 8.0),
			],
			vec![[0, 4, 1], [0, 3, 4], [1, 5, 2], [1, 4, 5]],
			TrimeshFlags::MERGE_DUPLICATE_VERTICES,
		),
		PhysicsInteractionLayer::fixed_layers(),
	));
	for _ in 0..3 {
		app.update();
	}
	let entity = actor(&mut app, origin + Vec3::Y * 20.0, Some(1.25));
	if let Some(mut pending) = app.world_mut().get_mut::<PendingActorSupport>(entity) {
		let hull = pending.hull();
		pending.observe_search(Some(PlacementSearch::outdoor_hint(origin.xz(), origin.y, hull)));
	}
	for _ in 0..30 {
		app.update();
		if app.world().get::<PendingActorSupport>(entity).is_none() {
			break;
		}
	}
	assert!(
		app.world().get::<PendingActorSupport>(entity).is_none(),
		"creased mesh pending={:?}",
		app.world().get::<PendingActorSupport>(entity)
	);
	let position = app
		.world()
		.get::<Position>(entity)
		.ok_or_else(|| anyhow::anyhow!("missing actor"))?
		.0;
	assert!((position.xz() - origin.xz()).length() < 0.4);
	assert!(position.y > origin.y + 0.8 && position.y < origin.y + 1.3, "position={position:?}");
	Ok(())
}

#[test]
fn collinear_mesh_triangle_does_not_claim_volume_above_retained_support() -> anyhow::Result<()> {
	use crate::PlacementSupportHit;
	use crate::actor::handoff::SupportGeometry;
	use bevy::ecs::system::SystemState;

	let mut app = app();
	let mesh = Collider::try_trimesh_with_config(
		vec![
			Vec3::new(-8.0, 0.0, -8.0),
			Vec3::new(8.0, 0.0, -8.0),
			Vec3::new(8.0, 0.0, 8.0),
			Vec3::new(-8.0, 0.0, 8.0),
			Vec3::new(-2.0, -2.0, -2.0),
			Vec3::ZERO,
			Vec3::new(2.0, 2.0, 2.0),
		],
		vec![[0, 2, 1], [0, 3, 2], [4, 5, 6]],
		TrimeshFlags::MERGE_DUPLICATE_VERTICES,
	)?;
	// Parry's degenerate-triangle point query reports interior at this
	// off-surface point inside the AABB of a diagonal line.
	let above_floor = Vec3::Y * 0.05;
	assert!(mesh.contains_point(Vec3::ZERO, Quat::IDENTITY, above_floor));
	let line_direction = Vec3::ONE.normalize();
	assert!(above_floor.cross(line_direction).length() > 0.03);
	let floor = app
		.world_mut()
		.spawn((
			Transform::IDENTITY,
			RigidBody::Static,
			mesh,
			PhysicsInteractionLayer::fixed_layers(),
		))
		.id();
	for _ in 0..3 {
		app.update();
	}
	let search =
		PlacementSearch::outdoor_hint(Vec2::ZERO, 0.0, characters::LocomotionCapsule::HUMANOID);
	let support = PlacementSupportHit::new(floor, Vec3::ZERO, Vec3::Y);
	let mut state = SystemState::<SupportGeometry>::new(app.world_mut());
	let geometry = state.get(app.world())?;
	let result = geometry.clear_pose(
		search.standing_at(0.0),
		Some(support),
		search,
		&PhysicsInteractionLayer::animated_layers(),
	);
	assert!(result.is_ok(), "open mesh incorrectly claims solid interior: {result:?}");
	Ok(())
}

#[test]
fn oriented_closed_triangle_mesh_retains_solid_containment_guard() -> anyhow::Result<()> {
	use crate::PlacementSupportHit;
	use crate::actor::handoff::{PoseRejection, SupportGeometry};
	use bevy::ecs::system::SystemState;

	let mut app = app();
	let mesh = Collider::trimesh_from_mesh_with_config(
		&Mesh::from(Cuboid::new(4.0, 4.0, 4.0)),
		TrimeshFlags::MERGE_DUPLICATE_VERTICES | TrimeshFlags::ORIENTED,
	)
	.ok_or_else(|| anyhow::anyhow!("failed to construct oriented closed mesh"))?;
	assert!(mesh.contains_point(Vec3::ZERO, Quat::IDENTITY, Vec3::Y * 0.05));
	let floor = app
		.world_mut()
		.spawn((
			Transform::IDENTITY,
			RigidBody::Static,
			mesh,
			PhysicsInteractionLayer::fixed_layers(),
		))
		.id();
	for _ in 0..3 {
		app.update();
	}
	let search =
		PlacementSearch::outdoor_hint(Vec2::ZERO, 0.0, characters::LocomotionCapsule::HUMANOID);
	let mut state = SystemState::<SupportGeometry>::new(app.world_mut());
	let geometry = state.get(app.world())?;
	let result = geometry.clear_pose(
		search.standing_at(0.0),
		Some(PlacementSupportHit::new(floor, Vec3::ZERO, Vec3::Y)),
		search,
		&PhysicsInteractionLayer::animated_layers(),
	);
	assert_eq!(result, Err(PoseRejection::SupportContaining));
	Ok(())
}
