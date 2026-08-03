use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

use super::geometry::{LongAxis, VolumeCandidate};
use super::topology::resolve_junctions;
use super::{EndCap, Overhang, RectangularPitchedRoofComplexParams};

#[test]
fn single_box_long_axis_and_no_valleys() {
	let complex = RectangularPitchedRoofComplexParams::single(10.0, 6.0, 2.0, 4.0)
		.end_cap(EndCap::Hip)
		.build();
	assert_eq!(complex.roofs().len(), 1);
	assert!(complex.valleys().is_empty());
	let ridge = complex.roofs()[0].params().halves[0].ridge_line;
	assert!((ridge.0.z - ridge.1.z).abs() < 1e-4, "ridge along X");
	assert!(ridge.0.x < ridge.1.x);
}

#[test]
fn single_gable_extends_free_ends() {
	let complex = RectangularPitchedRoofComplexParams::single(10.0, 6.0, 2.0, 4.0)
		.end_cap(EndCap::Gable {
			ridge: Overhang::Fixed(0.5),
			eave: Overhang::Fixed(0.4),
		})
		.build();
	assert_eq!(complex.roofs().len(), 1);
	let half = &complex.roofs()[0].params().halves[0];
	assert_eq!(half.draw_in_half_gable_end, (true, true));
	assert_eq!(half.draw_in_half_hip, (false, false));
	// Wall stays at the massing end; ridge/eave project past it (barge overhang).
	let wall0 = half.wall_line.0.x;
	let wall1 = half.wall_line.1.x;
	assert!((wall0 - (-5.0)).abs() < 1e-3, "wall min {wall0}");
	assert!((wall1 - 5.0).abs() < 1e-3, "wall max {wall1}");
	assert!((half.eave_line.0.x - (-5.4)).abs() < 1e-3);
	assert!((half.eave_line.1.x - 5.4).abs() < 1e-3);
	assert!((half.ridge_line.0.x - (-5.5)).abs() < 1e-3);
	assert!((half.ridge_line.1.x - 5.5).abs() < 1e-3);
}

#[test]
fn stepped_presets_vary_ridge_and_eave_heights() {
	let ridge_step = RectangularPitchedRoofComplexParams::l_shape_stepped_ridge().build();
	// Each volume keeps a level ridge at its box top (no angling down to a shared Y).
	for roof in ridge_step.roofs() {
		let (a, b) = roof.params().halves[0].ridge_line;
		assert!(
			(a.y - b.y).abs() < 1e-3,
			"ridge should stay level, got {a:?} → {b:?}"
		);
	}
	let ridge_ys: Vec<f32> = ridge_step
		.roofs()
		.iter()
		.map(|r| r.params().halves[0].ridge_line.0.y)
		.collect();
	assert!(
		ridge_ys.iter().any(|y| (y - 4.2).abs() < 1e-3),
		"missing bar ridge 4.2 in {ridge_ys:?}"
	);
	assert!(
		ridge_ys.iter().any(|y| (y - 5.5).abs() < 1e-3),
		"missing stem ridge 5.5 in {ridge_ys:?}"
	);
	assert!(!ridge_step.valleys().is_empty());

	let eave_step = RectangularPitchedRoofComplexParams::l_shape_stepped_eave().build();
	let eave_ys: Vec<f32> = eave_step
		.roofs()
		.iter()
		.flat_map(|r| {
			let (a, b) = r.params().halves[0].eave_line;
			[a.y, b.y]
		})
		.collect();
	assert!(
		eave_ys.iter().any(|y| (y - 2.0).abs() < 1e-3),
		"missing bar eave 2.0 in {eave_ys:?}"
	);
	assert!(
		eave_ys.iter().any(|y| (y - 3.2).abs() < 1e-3),
		"missing stem eave 3.2 in {eave_ys:?}"
	);
	assert!(!eave_step.valleys().is_empty());

	let t = RectangularPitchedRoofComplexParams::t_shape_stepped().build();
	assert_eq!(t.roofs().len(), 2);
	assert!(!t.valleys().is_empty());
}

#[test]
fn l_shape_marks_junction_and_builds_valley() {
	let complex = RectangularPitchedRoofComplexParams::l_shape().build();
	assert_eq!(complex.roofs().len(), 2);
	assert!(
		!complex.valleys().is_empty(),
		"expected at least one valley at the L corner"
	);
	let v = complex.valleys()[0];
	assert!(v.ridge_point.y > v.eave_point.y);
	// Inner corner of default L is near (+2, +2) in XZ before overhang.
	assert!(v.eave_point.x > 1.5 && v.eave_point.z > 1.5);

	// Outside hip corner: outer eaves meet near (−2−oh, −2−oh).
	let oh = 0.3;
	let mut found_outer = false;
	for roof in complex.roofs() {
		for half in &roof.params().halves {
			for end in [half.eave_line.0, half.eave_line.1] {
				if (end.x - (-2.0 - oh)).abs() < 1e-2 && (end.z - (-2.0 - oh)).abs() < 1e-2 {
					found_outer = true;
				}
			}
		}
	}
	assert!(found_outer, "expected outer eaves to meet at the convex L corner");

	// Ridges stay level at box top.
	for roof in complex.roofs() {
		let (a, b) = roof.params().halves[0].ridge_line;
		assert!((a.y - b.y).abs() < 1e-3);
		assert!((a.y - 4.5).abs() < 1e-3);
	}
}

#[test]
fn t_shape_builds_valleys() {
	let complex = RectangularPitchedRoofComplexParams::t_shape().build();
	assert_eq!(complex.roofs().len(), 2);
	assert!(
		complex.valleys().len() >= 2,
		"T should yield two concave corners, got {}",
		complex.valleys().len()
	);
}

#[test]
fn disjoint_boxes_have_no_valleys() {
	let complex = RectangularPitchedRoofComplexParams::new(vec![
		Aabb3d::from_min_max(Vec3::new(0.0, 2.0, 0.0), Vec3::new(6.0, 4.0, 3.0)),
		Aabb3d::from_min_max(Vec3::new(20.0, 2.0, 0.0), Vec3::new(23.0, 4.0, 8.0)),
	])
	.build();
	assert_eq!(complex.roofs().len(), 2);
	assert!(complex.valleys().is_empty());
}

#[test]
fn junction_detection_l() {
	let mut vols = vec![
		VolumeCandidate::from_aabb(
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -2.0), Vec3::new(8.0, 4.5, 2.0)),
			Overhang::Fixed(0.3),
		),
		VolumeCandidate::from_aabb(
			Aabb3d::from_min_max(Vec3::new(-2.0, 2.5, -2.0), Vec3::new(2.0, 4.5, 8.0)),
			Overhang::Fixed(0.3),
		),
	];
	assert_eq!(vols[0].long_axis, LongAxis::X);
	assert_eq!(vols[1].long_axis, LongAxis::Z);
	let corners = resolve_junctions(&mut vols);
	assert_eq!(corners.len(), 1);
	assert!(!vols[0].end_free[0] || !vols[1].end_free[0]);
}
