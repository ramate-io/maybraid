//! Real spatial-query coverage of the entire bounded readiness column.

use anyhow::Result;
use avian3d::prelude::*;
use bevy::{ecs::system::RunSystemOnce, prelude::*, time::TimeUpdateStrategy};
use characters::LocomotionCapsule;
use lod_avian::PhysicsInteractionLayer;
use std::time::Duration;

use crate::{
	PlacementSearch, SupportProbeFailure, SupportProbeOutcome, column_presentation_ready,
	probe_search,
};

fn interior_search() -> PlacementSearch {
	// Nonzero coordinates ensure the readiness volume keeps the authored pose.
	PlacementSearch::interior_at(Vec3::new(11.0, 40.0, -27.0), LocomotionCapsule::HUMANOID)
}

fn query_floor(
	search: PlacementSearch,
	floor_top: Vec3,
	width: f32,
	depth: f32,
) -> Result<(bool, SupportProbeOutcome)> {
	query_scene(search, &[(floor_top, width, depth)])
}

fn query_scene(
	search: PlacementSearch,
	floors: &[(Vec3, f32, f32)],
) -> Result<(bool, SupportProbeOutcome)> {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		PhysicsPlugins::default(),
		bevy::asset::AssetPlugin::default(),
		bevy::mesh::MeshPlugin,
	));
	app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(1.0 / 64.0)));
	app.finish();
	for (floor_top, width, depth) in floors {
		app.world_mut().spawn((
			Transform::from_translation(*floor_top - Vec3::Y * 0.01),
			RigidBody::Static,
			Collider::cuboid(*width, 0.02, *depth),
			PhysicsInteractionLayer::fixed_layers(),
		));
	}
	// Exercise the real collider presentation and Avian spatial tree, including
	// the initial frame before fixed physics starts ticking.
	for _ in 0..4 {
		app.update();
	}
	app.world_mut()
		.run_system_once(move |spatial: SpatialQuery| {
			let filter = PhysicsInteractionLayer::Fixed.query_filter();
			(
				column_presentation_ready(&spatial, &filter, search),
				probe_search(&spatial, &filter, search),
			)
		})
		.map_err(|error| anyhow::anyhow!("{error:?}"))
}

#[test]
fn readiness_reaches_a_valid_floor_near_the_lower_search_edge() -> Result<()> {
	let search = interior_search();
	assert_usable_floor(search, search.min_surface_y + 0.05)
}

#[test]
fn readiness_reaches_a_valid_floor_near_the_upper_search_edge() -> Result<()> {
	let search = interior_search();
	assert_usable_floor(search, search.max_surface_y - 0.05)
}

fn assert_usable_floor(search: PlacementSearch, height: f32) -> Result<()> {
	let floor_top = Vec3::new(search.xz.x, height, search.xz.y);
	let (presented, outcome) = query_floor(search, floor_top, 8.0, 8.0)?;
	assert!(presented, "valid floor near search edge {height} was considered unpublished");
	let SupportProbeOutcome::Ready { pose, support: Some(hit) } = outcome else {
		return Err(anyhow::anyhow!("published edge floor {height} was not usable: {outcome:?}"));
	};
	assert!((hit.point.y - height).abs() < 0.01);
	assert!((pose.position.y - search.standing_at(height).y).abs() < 0.01);
	Ok(())
}

#[test]
fn readiness_includes_collision_near_each_lateral_capsule_edge() -> Result<()> {
	let search = interior_search();
	let height = (search.min_surface_y + search.max_surface_y) * 0.5;
	for direction in [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y] {
		let xz = search.xz + direction * search.hull.radius * 0.95;
		let (presented, outcome) = query_floor(search, Vec3::new(xz.x, height, xz.y), 0.02, 0.02)?;
		assert!(presented, "geometry inside the capsule footprint was missed in {direction:?}");
		// Presentation only admits the bounded support probe. This tiny edge
		// patch cannot support its central disk, so it fails rather than waiting.
		assert_eq!(outcome, SupportProbeOutcome::Failed(SupportProbeFailure::NoEligibleSupport));
	}
	Ok(())
}

#[test]
fn outside_vertical_floors_do_not_mark_an_interior_storey_ready() -> Result<()> {
	let search = interior_search();
	for height in [0.0, search.min_surface_y - 0.25, search.max_surface_y + 0.25] {
		let (presented, outcome) =
			query_floor(search, Vec3::new(search.xz.x, height, search.xz.y), 8.0, 8.0)?;
		assert!(!presented, "floor outside the intended storey at {height} claimed readiness");
		assert_eq!(outcome, SupportProbeOutcome::Waiting);
	}
	Ok(())
}

#[test]
fn geometry_outside_the_lateral_footprint_stays_unpublished() -> Result<()> {
	let search = interior_search();
	let height = (search.min_surface_y + search.max_surface_y) * 0.5;
	for direction in [Vec2::X, Vec2::NEG_X, Vec2::Y, Vec2::NEG_Y] {
		let xz = search.xz + direction * (search.hull.radius + 0.1);
		let (presented, outcome) = query_floor(search, Vec3::new(xz.x, height, xz.y), 0.02, 0.02)?;
		assert!(!presented, "geometry beyond the footprint claimed readiness in {direction:?}");
		assert_eq!(outcome, SupportProbeOutcome::Waiting);
	}
	Ok(())
}

#[test]
fn presented_edge_geometry_does_not_admit_a_floor_outside_the_intended_storey() -> Result<()> {
	let search = interior_search();
	let edge_x = search.xz.x + search.hull.radius * 0.95;
	let edge_height = (search.min_surface_y + search.max_surface_y) * 0.5;
	for height in [search.min_surface_y - 0.25, search.max_surface_y + 0.25] {
		let (presented, outcome) = query_scene(
			search,
			&[
				(Vec3::new(search.xz.x, height, search.xz.y), 8.0, 8.0),
				(Vec3::new(edge_x, edge_height, search.xz.y), 0.02, 0.02),
			],
		)?;
		assert!(presented, "the in-band edge patch should establish presentation");
		assert_eq!(outcome, SupportProbeOutcome::Failed(SupportProbeFailure::NoEligibleSupport));
	}
	Ok(())
}

#[test]
fn minimum_thickness_readiness_does_not_expand_an_exact_support_interval() -> Result<()> {
	let mut search = interior_search();
	search.min_surface_y = 40.0;
	search.max_surface_y = 40.0;
	let (presented, outcome) =
		query_floor(search, Vec3::new(search.xz.x, 40.01, search.xz.y), 8.0, 8.0)?;
	assert!(presented, "the floor intersects the minimum-thickness presentation volume");
	assert_eq!(outcome, SupportProbeOutcome::Failed(SupportProbeFailure::NoEligibleSupport));
	Ok(())
}
