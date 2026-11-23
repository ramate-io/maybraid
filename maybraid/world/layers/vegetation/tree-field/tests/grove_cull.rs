//! CPU mirror of pass-1 grove cull and pass-2 work sizing for tree residency.

use bevy::math::Vec3;
use vegetation_grass_field::{map_work_index, visible_spans_from_records, WoodyGroveRecord};
use vegetation_shaders::wesl::{GrassViewUniform, LodThresholds};

fn look(origin: Vec3, target: Vec3) -> GrassViewUniform {
	use bevy::camera::CameraProjection;
	use bevy::math::Vec4;
	use bevy::prelude::{PerspectiveProjection, Transform};
	let view_from_world = Transform::from_translation(origin)
		.looking_at(target, Vec3::Y)
		.to_matrix()
		.inverse();
	let projection = PerspectiveProjection {
		fov: 1.0,
		aspect_ratio: 16.0 / 9.0,
		near: 0.1,
		far: 5_000.0,
		..Default::default()
	};
	let thresholds = LodThresholds::grove_tree();
	let clip = projection.get_clip_from_view() * view_from_world;
	GrassViewUniform {
		clip_from_world: clip,
		world_from_clip: clip.inverse(),
		world_position: origin.extend(0.0),
		prev_world_position: origin.extend(0.0),
		resolution: Vec4::new(1280.0, 720.0, 1.0, 0.0),
		wind: Vec4::new(0.0, 0.0, 0.0, 0.5),
		lod_distances: Vec4::new(0.85, thresholds.near_m, thresholds.mid_m, thresholds.card_m),
		lod_density: Vec4::new(
			thresholds.near_density,
			thresholds.mid_density,
			thresholds.card_density,
			thresholds.min_height_px,
		),
		counts: Vec4::ZERO,
		light_direction: Vec4::new(0.4, 0.8, 0.3, 0.0),
	}
}

#[test]
fn culled_grove_produces_zero_pass2_work() {
	let thresholds = LodThresholds::grove_tree();
	let view = look(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.0, 5.0, 500.0));
	let far = WoodyGroveRecord {
		aabb_min: Vec3::new(400.0, 0.0, 400.0),
		aabb_max: Vec3::new(410.0, 8.0, 410.0),
		branch_off: 10,
		branch_count: 50,
		leaf_off: 100,
		leaf_count: 200,
		flags: WoodyGroveRecord::FLAG_ACTIVE,
		..Default::default()
	};
	let (spans, branches, leaves) =
		visible_spans_from_records(&view, thresholds, &[far], |_| 6.0, 0.5);
	assert!(spans.is_empty());
	assert_eq!(branches, 0);
	assert_eq!(leaves, 0);
}

#[test]
fn visible_grove_maps_pass2_indices() {
	let thresholds = LodThresholds::grove_tree();
	let view = look(Vec3::new(0.0, 10.0, 30.0), Vec3::new(0.0, 6.0, 0.0));
	let near = WoodyGroveRecord {
		aabb_min: Vec3::new(-5.0, 0.0, -5.0),
		aabb_max: Vec3::new(5.0, 20.0, 5.0),
		branch_off: 4,
		branch_count: 12,
		leaf_off: 40,
		leaf_count: 80,
		flags: WoodyGroveRecord::FLAG_ACTIVE,
		..Default::default()
	};
	let (spans, branches, leaves) =
		visible_spans_from_records(&view, thresholds, &[near], |_| 12.0, 0.5);
	assert_eq!(spans.len(), 1);
	assert_eq!(branches, 12);
	assert_eq!(leaves, 80);
	assert_eq!(map_work_index(&spans, 3, true), Some(7));
	assert_eq!(map_work_index(&spans, 1, false), Some(41));
}