//! Whole-grove visibility before per-branch / per-blade selection.

use bevy::math::Vec3;
use vegetation_shaders::wesl::{lod_bucket, GrassPatchHeader, GrassViewUniform, LodThresholds};

use crate::grove_record::{GroveVisibleSpan, WoodyGroveRecord};
use crate::select::WorldFrustum;

/// Whole-grove cull using AABB frustum and residency distance (matches GPU pass 1).
pub fn woody_grove_visible(
	view: &GrassViewUniform,
	thresholds: LodThresholds,
	record: WoodyGroveRecord,
	wind_strength_m: f32,
	representative_height_m: f32,
) -> bool {
	if !record.is_active() {
		return false;
	}
	let frustum = WorldFrustum::from_clip(view.clip_from_world);
	let center = record.center();
	let radius = record.extent_radius() + wind_strength_m + representative_height_m * 0.15;
	if !frustum.contains_sphere(center, radius.max(0.5)) {
		return false;
	}
	let camera = view.world_position.truncate();
	let distance = camera.distance(center);
	let bucket = lod_bucket(
		distance,
		representative_height_m.max(0.5),
		0.0,
		view.render_height_px(),
		view.fov_y_radians(),
		thresholds,
	);
	bucket < 3
}

pub fn grass_patch_visible(
	view: &GrassViewUniform,
	thresholds: LodThresholds,
	header: &GrassPatchHeader,
) -> bool {
	if !header.is_active() {
		return false;
	}
	let frustum = WorldFrustum::from_clip(view.clip_from_world);
	let center = (header.aabb_min + header.aabb_max) * 0.5;
	let radius = (header.aabb_max - header.aabb_min).length() * 0.5 + 0.5;
	if !frustum.contains_sphere(center, radius) {
		return false;
	}
	let camera = view.world_position.truncate();
	let distance = camera.distance(center);
	lod_bucket(
		distance,
		radius,
		0.0,
		view.render_height_px(),
		view.fov_y_radians(),
		thresholds,
	) < 3
}

/// Map a flat work index into a catalog item index within visible grove spans.
pub fn map_work_index(spans: &[GroveVisibleSpan], work_index: u32, branch: bool) -> Option<u32> {
	let mut cursor = 0u32;
	for span in spans {
		let count = if branch { span.branch_count } else { span.leaf_count };
		let off = if branch { span.branch_off } else { span.leaf_off };
		if work_index < cursor + count {
			return Some(off + (work_index - cursor));
		}
		cursor += count;
	}
	None
}

pub fn visible_spans_from_records(
	view: &GrassViewUniform,
	thresholds: LodThresholds,
	groves: &[WoodyGroveRecord],
	heights: impl Fn(WoodyGroveRecord) -> f32,
	wind_strength_m: f32,
) -> (Vec<GroveVisibleSpan>, u32, u32) {
	let mut spans = Vec::new();
	let mut branches = 0u32;
	let mut leaves = 0u32;
	for &record in groves {
		let height = heights(record);
		if !woody_grove_visible(view, thresholds, record, wind_strength_m, height) {
			continue;
		}
		spans.push(GroveVisibleSpan {
			branch_off: record.branch_off,
			branch_count: record.branch_count,
			leaf_off: record.leaf_off,
			leaf_count: record.leaf_count,
		});
		branches += record.branch_count;
		leaves += record.leaf_count;
	}
	(spans, branches, leaves)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::camera::CameraProjection;
	use bevy::math::Vec4;
	use bevy::prelude::{PerspectiveProjection, Transform};

	fn look(origin: Vec3, target: Vec3, thresholds: LodThresholds) -> GrassViewUniform {
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
		let clip = projection.get_clip_from_view() * view_from_world;
		GrassViewUniform {
			clip_from_world: clip,
			world_from_clip: clip.inverse(),
			world_position: origin.extend(0.0),
			prev_world_position: origin.extend(0.0),
			resolution: Vec4::new(1280.0, 720.0, 1.0, 0.0),
			wind: Vec4::new(0.0, 0.0, 0.0, 0.5),
			lod_distances: Vec4::new(0.0, thresholds.near_m, thresholds.mid_m, thresholds.card_m),
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
		let thresholds = LodThresholds::grove_bush();
		let view = look(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.0, 5.0, 500.0), thresholds);
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
	fn visible_grove_contributes_spans() {
		let thresholds = LodThresholds::grove_tree();
		let view = look(Vec3::new(0.0, 10.0, 30.0), Vec3::new(0.0, 6.0, 0.0), thresholds);
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
		assert_eq!(spans[0].branch_count, 12);
		assert_eq!(branches, 12);
		assert_eq!(leaves, 80);
		assert_eq!(map_work_index(&spans, 3, true), Some(7));
	}
}