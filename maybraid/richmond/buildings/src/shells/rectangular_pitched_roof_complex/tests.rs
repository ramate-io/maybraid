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
