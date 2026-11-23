//! Whole-grove CPU visibility before per-item selection. No GPU grove-cull pass.

use bevy::math::Vec3;
use vegetation_shaders::wesl::{lod_bucket, GrassPatchHeader, GrassViewUniform, LodThresholds};

use crate::catalog::GroveId;
use crate::grove_record::{GroveVisibleSpan, VisibleGrove, WoodyGroveRecord};
use crate::select::{aabb_nearest_distance, WorldFrustum};

/// Metres of stickiness on nested band edges and the finite drop distance.
pub const GROVE_BAND_HYSTERESIS_M: f32 = 16.0;

/// Pick a grove LOD band (`0..=2`) or `None` when the grove is dropped.
pub fn grove_lod_band(
	distance_m: f32,
	thresholds: LodThresholds,
	previous: Option<u32>,
) -> Option<u32> {
	let hyst = GROVE_BAND_HYSTERESIS_M;
	if let Some(prev) = previous.filter(|&band| band < 3) {
		if distance_m >= thresholds.card_m + hyst {
			return None;
		}
		let stay = match prev {
			0 => distance_m < thresholds.near_m + hyst,
			1 => distance_m >= thresholds.near_m - hyst && distance_m < thresholds.mid_m + hyst,
			2 => distance_m >= thresholds.mid_m - hyst && distance_m < thresholds.card_m + hyst,
			_ => false,
		};
		if stay {
			return Some(prev);
		}
	} else if distance_m >= thresholds.card_m {
		return None;
	}
	let bucket = lod_bucket(distance_m, 12.0, 0.0, 1080.0, 1.0, thresholds);
	(bucket < 3).then_some(bucket)
}

pub fn woody_grove_visible(
	view: &GrassViewUniform,
	thresholds: LodThresholds,
	record: WoodyGroveRecord,
	wind_strength_m: f32,
	previous: Option<u32>,
) -> Option<u32> {
	if !record.is_active() {
		return None;
	}
	let pad = Vec3::splat(wind_strength_m.max(0.5));
	let aabb_min = record.aabb_min - pad;
	let aabb_max = record.aabb_max + pad;
	let frustum = WorldFrustum::from_clip(view.clip_from_world);
	if !frustum.contains_aabb(aabb_min, aabb_max) {
		return None;
	}
	let camera = view.world_position.truncate();
	grove_lod_band(aabb_nearest_distance(aabb_min, aabb_max, camera), thresholds, previous)
}

pub fn grass_patch_visible(
	view: &GrassViewUniform,
	thresholds: LodThresholds,
	header: &GrassPatchHeader,
	previous: Option<u32>,
) -> Option<u32> {
	if !header.is_active() {
		return None;
	}
	let pad = Vec3::splat(0.5);
	let aabb_min = header.aabb_min - pad;
	let aabb_max = header.aabb_max + pad;
	let frustum = WorldFrustum::from_clip(view.clip_from_world);
	if !frustum.contains_aabb(aabb_min, aabb_max) {
		return None;
	}
	let camera = view.world_position.truncate();
	grove_lod_band(aabb_nearest_distance(aabb_min, aabb_max, camera), thresholds, previous)
}

/// Map a flat work index into a catalog item index within visible grove spans.
pub fn map_work_index(spans: &[GroveVisibleSpan], work_index: u32, branch: bool) -> Option<u32> {
	map_work_item(spans, work_index, branch).map(|(index, _lod)| index)
}

/// Map a compacted work index onto `(catalog_index, grove_lod)`.
pub fn map_work_item(
	spans: &[GroveVisibleSpan],
	work_index: u32,
	branch: bool,
) -> Option<(u32, u32)> {
	let mut cursor = 0u32;
	for span in spans {
		let count = if branch { span.branch_count } else { span.leaf_count };
		let off = if branch { span.branch_off } else { span.leaf_off };
		if work_index < cursor + count {
			return Some((off + (work_index - cursor), span.lod));
		}
		cursor += count;
	}
	None
}

pub fn cull_woody_groves(
	view: &GrassViewUniform,
	thresholds: LodThresholds,
	groves: impl IntoIterator<Item = (GroveId, WoodyGroveRecord)>,
	previous: &impl Fn(GroveId) -> Option<u32>,
	wind_strength_m: f32,
) -> Vec<VisibleGrove> {
	let _span = bevy::log::info_span!("grove cull").entered();
	let mut visible = Vec::new();
	for (id, record) in groves {
		let Some(lod) =
			woody_grove_visible(view, thresholds, record, wind_strength_m, previous(id))
		else {
			continue;
		};
		visible.push(VisibleGrove {
			id,
			lod,
			span: GroveVisibleSpan {
				branch_off: record.branch_off,
				branch_count: record.branch_count,
				leaf_off: record.leaf_off,
				leaf_count: record.leaf_count,
				lod,
			},
		});
	}
	visible
}

pub fn cull_grass_patches(
	view: &GrassViewUniform,
	thresholds: LodThresholds,
	patches: impl IntoIterator<Item = (GroveId, GrassPatchHeader)>,
	previous: &impl Fn(GroveId) -> Option<u32>,
) -> Vec<VisibleGrove> {
	let _span = bevy::log::info_span!("grove cull").entered();
	let mut visible = Vec::new();
	for (id, header) in patches {
		let Some(lod) = grass_patch_visible(view, thresholds, &header, previous(id)) else {
			continue;
		};
		visible.push(VisibleGrove {
			id,
			lod,
			span: GroveVisibleSpan {
				branch_off: header.blade_offset,
				branch_count: header.blade_count,
				leaf_off: 0,
				leaf_count: 0,
				lod,
			},
		});
	}
	visible
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::catalog::GroveEncodeStats;
	use crate::grove_retain::{GroveSubmit, RetainedGroveFrame};
	use bevy::camera::CameraProjection;
	use bevy::math::{Vec3, Vec4};
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

	fn active_grove(aabb_min: Vec3, aabb_max: Vec3) -> WoodyGroveRecord {
		WoodyGroveRecord {
			aabb_min,
			aabb_max,
			branch_off: 4,
			branch_count: 12,
			leaf_off: 40,
			leaf_count: 80,
			flags: WoodyGroveRecord::FLAG_ACTIVE,
			..Default::default()
		}
	}

	#[test]
	fn culled_grove_produces_zero_select_work() {
		let thresholds = LodThresholds::grove_bush();
		let view = look(Vec3::new(0.0, 10.0, 0.0), Vec3::new(0.0, 5.0, 500.0), thresholds);
		let far = active_grove(Vec3::new(400.0, 0.0, 400.0), Vec3::new(410.0, 8.0, 410.0));
		let visible = cull_woody_groves(&view, thresholds, [(GroveId(1), far)], &|_| None, 0.5);
		assert!(visible.is_empty());
		assert_eq!(map_work_index(&[], 0, true), None);
	}

	#[test]
	fn visible_grove_contributes_spans() {
		let thresholds = LodThresholds::grove_tree();
		let view = look(Vec3::new(0.0, 10.0, 30.0), Vec3::new(0.0, 6.0, 0.0), thresholds);
		let near = active_grove(Vec3::new(-5.0, 0.0, -5.0), Vec3::new(5.0, 20.0, 5.0));
		let visible = cull_woody_groves(&view, thresholds, [(GroveId(9), near)], &|_| None, 0.5);
		assert_eq!(visible.len(), 1);
		assert_eq!(visible[0].span.branch_count, 12);
		assert_eq!(visible[0].span.lod, visible[0].lod);
		assert_eq!(map_work_item(&[visible[0].span], 3, true), Some((7, visible[0].lod)));
	}

	#[test]
	fn band_hysteresis_holds_near_past_nominal_edge() {
		let t = LodThresholds::grove_tree();
		assert_eq!(grove_lod_band(399.0, t, None), Some(0));
		assert_eq!(grove_lod_band(410.0, t, None), Some(1));
		assert_eq!(grove_lod_band(410.0, t, Some(0)), Some(0));
		assert_eq!(grove_lod_band(t.card_m + 1.0, t, Some(2)), Some(2));
		assert_eq!(grove_lod_band(t.card_m + GROVE_BAND_HYSTERESIS_M + 1.0, t, Some(2)), None);
	}

	#[test]
	fn finite_card_drops_bush_before_nested_tree_bands() {
		let t = LodThresholds::grove_bush();
		assert!(t.near_m < t.mid_m && t.mid_m < t.card_m);
		assert_eq!(grove_lod_band(20.0, t, None), Some(0));
		assert_eq!(grove_lod_band(50.0, t, None), Some(0));
		assert_eq!(grove_lod_band(70.0, t, None), Some(1));
		assert_eq!(grove_lod_band(95.0, t, None), Some(2));
		assert_eq!(grove_lod_band(120.0, t, None), None);
		assert_eq!(grove_lod_band(t.card_m + 1.0, t, Some(2)), Some(2));
		assert_eq!(grove_lod_band(t.card_m + GROVE_BAND_HYSTERESIS_M + 1.0, t, Some(2)), None);
	}

	#[test]
	fn frustum_entry_dirties_empty_set() {
		let thresholds = LodThresholds::grove_tree();
		let away = look(Vec3::new(0.0, 10.0, 30.0), Vec3::new(0.0, 5.0, 500.0), thresholds);
		let toward = look(Vec3::new(0.0, 10.0, 30.0), Vec3::new(0.0, 6.0, 0.0), thresholds);
		let grove = active_grove(Vec3::new(-5.0, 0.0, -5.0), Vec3::new(5.0, 20.0, 5.0));
		assert!(
			cull_woody_groves(&away, thresholds, [(GroveId(1), grove)], &|_| None, 0.5).is_empty()
		);
		assert_eq!(
			cull_woody_groves(&toward, thresholds, [(GroveId(1), grove)], &|_| None, 0.5).len(),
			1
		);
	}

	#[test]
	fn aabb_keeps_on_screen_lateral_grove_that_sphere_culls() {
		let thresholds = LodThresholds::grove_grass();
		// 60×25×60 m grove, 40 m ahead, 84 m to the right. The far-left corner
		// sits inside a 1 rad vertical FOV; the bounding sphere centre does not.
		let origin = Vec3::new(0.0, 12.5, 0.0);
		let view = look(origin, Vec3::new(0.0, 12.5, 40.0), thresholds);
		let grove = active_grove(Vec3::new(54.0, 0.0, 10.0), Vec3::new(114.0, 25.0, 70.0));
		let frustum = WorldFrustum::from_clip(view.clip_from_world);
		let center = grove.center();
		let radius = (grove.aabb_max - grove.aabb_min).length() * 0.5;
		assert!(
			!frustum.contains_sphere(center, radius),
			"sphere test must still reject this case so the AABB regression is real"
		);
		let visible = cull_woody_groves(&view, thresholds, [(GroveId(1), grove)], &|_| None, 0.0);
		assert_eq!(visible.len(), 1, "p-vertex AABB cull must keep the on-screen grove");
		let nearest = aabb_nearest_distance(grove.aabb_min, grove.aabb_max, origin);
		assert!(nearest < origin.distance(center));
		assert_eq!(visible[0].lod, grove_lod_band(nearest, thresholds, None).unwrap());
	}

	fn prepare_encode(
		previous: &mut RetainedGroveFrame,
		view: &GrassViewUniform,
		thresholds: LodThresholds,
		records: &[(GroveId, WoodyGroveRecord)],
		previous_lod: Option<u32>,
		uploads: &mut u32,
	) -> GroveEncodeStats {
		let visible =
			cull_woody_groves(view, thresholds, records.iter().copied(), &|_| previous_lod, 0.5);
		let submit =
			GroveSubmit::plan(previous, 1, 1280, 720, 1.0, thresholds.card_m, visible, true, 0, 0);
		let mut encode = GroveEncodeStats::default();
		let stamped = submit.stamp(previous, &mut encode, |_| {
			*uploads += 1;
			false
		});
		GroveSubmit::record_select(
			&mut encode,
			stamped.dirty,
			true,
			stamped.work_branches,
			stamped.work_leaves,
		);
		encode.draws = 6;
		encode
	}

	#[test]
	fn prepare_encode_skips_select_on_still_camera_then_redirties_on_move() {
		let thresholds = LodThresholds::grove_tree();
		let toward = look(Vec3::new(0.0, 10.0, 30.0), Vec3::new(0.0, 6.0, 0.0), thresholds);
		let away = look(Vec3::new(0.0, 10.0, 30.0), Vec3::new(0.0, 5.0, 500.0), thresholds);
		let grove = active_grove(Vec3::new(-5.0, 0.0, -5.0), Vec3::new(5.0, 20.0, 5.0));
		let records = [(GroveId(1), grove)];
		let mut retained = RetainedGroveFrame::default();
		let mut uploads = 0u32;

		let stats1 =
			prepare_encode(&mut retained, &toward, thresholds, &records, None, &mut uploads);
		assert_eq!(uploads, 1);
		assert_eq!(stats1.span_uploads, 1);
		assert!(stats1.dispatches > 0);
		assert_eq!(stats1.draws, 6);
		assert_eq!(stats1.retained_clean, 0);
		assert_eq!(retained.groves.len(), 1);
		let lod = retained.groves[0].lod;

		let stats2 =
			prepare_encode(&mut retained, &toward, thresholds, &records, Some(lod), &mut uploads);
		assert_eq!(uploads, 1, "still camera must not rewrite visible spans");
		assert_eq!(stats2.dispatches, 0);
		assert_eq!(stats2.span_uploads, 0);
		assert_eq!(stats2.draws, 6);
		assert_eq!(stats2.retained_clean, 1);

		let stats3 =
			prepare_encode(&mut retained, &away, thresholds, &records, Some(lod), &mut uploads);
		assert_eq!(uploads, 1, "empty visible set does not upload spans");
		assert!(stats3.dispatches > 0);
		assert_eq!(stats3.span_uploads, 0);
		assert!(retained.groves.is_empty());
	}
}
