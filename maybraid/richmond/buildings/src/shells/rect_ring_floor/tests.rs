use bevy_math::{Vec2, Vec3};
use lod::gen::LodSceneLevel;
use richmond_building_components::panels::PanelGeometry;
use richmond_building_components::BuildingComponents;

use crate::openings::{MapsOpenings, Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::ClippedRectangularStripPiece;
use crate::shells::ortho::OrthoSide;

use super::geometry::solid_runs;
use super::{RectRingFloor, RectRingFloorParams, RectRingFloorSlab};

#[test]
fn default_constructs_outer_and_inner_walls() {
	let r = RectRingFloorParams::default().build();
	// 4 outer + 4 inner sides, no omits ⇒ 8 wall runs.
	assert_eq!(r.wall_count(), 8);
	assert!(!r.has_floor());
}

#[test]
fn inner_courtyard_smaller_than_outer() {
	let r = RectRingFloorParams::default().build();
	assert!(r.params().inner.x < r.params().outer.x);
	assert!(r.params().inner.y < r.params().outer.y);
}

#[test]
fn omit_mid_south_removes_outer_run() {
	let full = RectRingFloorParams::default().build();
	let omitted = RectRingFloorParams::default()
		.omit_outer_mid(OrthoSide::South, 8.0)
		.build();
	assert!(omitted.wall_count() < full.wall_count());
	// Full south omit on outer: remaining outer sides (3) + inner (4) = 7.
	assert_eq!(omitted.wall_count(), 7);
}

#[test]
fn solid_runs_merge_and_split() {
	let runs = solid_runs(10.0, &[(2.0, 4.0), (3.5, 5.0), (8.0, 9.0)]);
	assert_eq!(runs, vec![(0.0, 2.0), (5.0, 8.0), (9.0, 10.0)]);
}

#[test]
fn walls_are_rectangle_kits() {
	let r = RectRingFloorParams::default().build();
	for w in r.walls() {
		assert!(!w.pieces().is_empty());
		assert!(w
			.pieces()
			.iter()
			.flat_map(|p| p.panels())
			.all(|p| matches!(p.geometry, PanelGeometry::Rectangle(_))));
	}
}

#[test]
fn passage_on_outer_south_maps() {
	let r = RectRingFloorParams::default()
		.openings(Openings::new().with(
			"door",
			RectRingFloor::side_passage_opening(
				OrthoSide::South,
				Vec3::ZERO,
				Vec2::new(8.0, 6.0),
				1.2,
				2.1,
			),
		))
		.build();
	assert!(r.mapped_opening(&OpeningId::new("door")).is_some());
	assert!(r.walls().iter().any(|w| matches!(
		w.pieces()[0],
		ClippedRectangularStripPiece::Clipped(_)
	)));
}

#[test]
fn solid_floor_has_frame_pieces() {
	let r = RectRingFloorParams::default()
		.floor(RectRingFloorSlab::Solid)
		.build();
	assert!(r.has_floor());
	let panels = r.panel_nodes_for_level(LodSceneLevel::High);
	assert!(panels.len() > r.wall_count());
}

#[test]
fn cuts_slab_can_remove_a_frame_band() {
	let mut openings = Openings::new();
	// Cover the entire south frame band (outer −Z strip).
	openings.insert(
		"shaft",
		Opening::new(
			bevy_math::bounding::Aabb3d::from_min_max(
				Vec3::new(-5.0, -0.5, -3.5),
				Vec3::new(5.0, 0.5, -1.0),
			),
			OpeningLabel::Shaft,
		),
	);
	let solid = RectRingFloorParams::default()
		.floor(RectRingFloorSlab::Solid)
		.build();
	let cut = RectRingFloorParams::default()
		.floor(RectRingFloorSlab::Solid)
		.openings(openings)
		.build();
	let solid_n = solid.panel_nodes_for_level(LodSceneLevel::High).len();
	let cut_n = cut.panel_nodes_for_level(LodSceneLevel::High).len();
	assert!(cut_n < solid_n, "solid={solid_n} cut={cut_n}");
}
