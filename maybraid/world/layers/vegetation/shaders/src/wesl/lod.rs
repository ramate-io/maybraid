//! CPU mirror of `lod.wesl`.

use bevy::math::{Mat4, Vec3};

use crate::wesl::layout::{
	GrassViewUniform, GroveRecord, VisibleGroveSpan, GRASS_LOD_CARD, GRASS_LOD_MID, GRASS_LOD_NEAR,
};

/// Far-card / drop distance for grove-owned trees (the 1 km present ring).
pub const GROVE_TREE_CARD_M: f32 = 1_000.0;

/// Distance (metres) and density (`0..=1`) thresholds. Nested: card ⊂ mid ⊂ near.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LodThresholds {
	pub near_m: f32,
	pub mid_m: f32,
	pub card_m: f32,
	pub near_density: f32,
	pub mid_density: f32,
	pub card_density: f32,
	pub min_height_px: f32,
}

impl LodThresholds {
	pub fn field_default() -> Self {
		Self {
			near_m: 18.0,
			mid_m: 48.0,
			card_m: 90.0,
			near_density: 1.0,
			mid_density: 0.72,
			card_density: 0.38,
			min_height_px: 1.15,
		}
	}

	/// Small bushes keep a crown silhouette farther than grass turf.
	pub fn bush_default() -> Self {
		Self {
			near_m: 12.0,
			mid_m: 32.0,
			card_m: 110.0,
			near_density: 1.0,
			mid_density: 0.60,
			card_density: 0.28,
			min_height_px: 2.0,
		}
	}

	/// Trees keep height and crown silhouette well past bush card distance.
	pub fn tree_default() -> Self {
		Self {
			near_m: 28.0,
			mid_m: 72.0,
			card_m: 220.0,
			near_density: 1.0,
			mid_density: 0.48,
			card_density: 0.16,
			min_height_px: 3.2,
		}
	}

	/// Grove-owned trees: High/Medium geometry, then far cards out to the 1 km
	/// present ring. Finite `card_m` drops the grove past residency.
	pub fn grove_tree() -> Self {
		Self {
			near_m: 400.0,
			mid_m: 520.0,
			card_m: GROVE_TREE_CARD_M,
			near_density: 1.0,
			mid_density: 1.0,
			card_density: 0.22,
			min_height_px: 1.0,
		}
	}

	/// Grove-owned bushes keep a silhouette until the legacy ~110 m drop.
	pub fn grove_bush() -> Self {
		Self {
			near_m: 32.0,
			mid_m: 80.0,
			card_m: 110.0,
			near_density: 1.0,
			mid_density: 0.60,
			card_density: 0.28,
			min_height_px: 2.0,
		}
	}

	/// Grove-owned grass keeps turf until the legacy ~90 m drop.
	pub fn grove_grass() -> Self {
		Self {
			near_m: 60.0,
			mid_m: 80.0,
			card_m: 90.0,
			near_density: 1.0,
			mid_density: 0.72,
			card_density: 0.38,
			min_height_px: 1.15,
		}
	}

	pub fn nested_ok(self) -> bool {
		self.near_density >= self.mid_density && self.mid_density >= self.card_density
	}
}

/// Projected plant height at the actual render resolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ProjectedSize {
	pub height_px: f32,
}

impl ProjectedSize {
	pub fn height_px(
		height_m: f32,
		distance_m: f32,
		view_height_px: f32,
		fov_y_radians: f32,
	) -> Self {
		Self { height_px: projected_height_px(height_m, distance_m, view_height_px, fov_y_radians) }
	}
}

/// Selected geometry bucket plus whether the plant is submitted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LodDecision {
	pub bucket: u32,
}

impl LodDecision {
	pub fn is_submitted(self) -> bool {
		self.bucket < 3
	}
}

pub fn projected_height_px(
	height_m: f32,
	distance_m: f32,
	view_height_px: f32,
	fov_y_radians: f32,
) -> f32 {
	let dist = distance_m.max(1e-3);
	let denom = 2.0 * (fov_y_radians * 0.5).tan();
	view_height_px * (height_m / dist) / denom.max(1e-5)
}

pub fn lod_fade(distance_m: f32, start_m: f32, end_m: f32) -> f32 {
	((end_m - distance_m) / (end_m - start_m).max(1e-3)).clamp(0.0, 1.0)
}

pub fn nested_keep(rank: f32, density: f32) -> bool {
	rank < density.clamp(0.0, 1.0)
}

pub fn lod_bucket(
	distance_m: f32,
	height_m: f32,
	rank: f32,
	view_height_px: f32,
	fov_y_radians: f32,
	thresholds: LodThresholds,
) -> u32 {
	let px = projected_height_px(height_m, distance_m, view_height_px, fov_y_radians);
	if px < thresholds.min_height_px && distance_m > thresholds.card_m {
		return 3;
	}
	if distance_m < thresholds.near_m && nested_keep(rank, thresholds.near_density) {
		return GRASS_LOD_NEAR;
	}
	if distance_m < thresholds.mid_m && nested_keep(rank, thresholds.mid_density) {
		return GRASS_LOD_MID;
	}
	if distance_m < thresholds.card_m && nested_keep(rank, thresholds.card_density) {
		return GRASS_LOD_CARD;
	}
	3
}

/// Clip-space sphere test matching `lod.wesl` / the select shaders.
pub fn sphere_in_frustum(center: Vec3, radius: f32, clip: Mat4) -> bool {
	let p = clip * center.extend(1.0);
	let slack = radius * clip.x_axis.truncate().length() + 0.05;
	let w = p.w + slack;
	w > 0.0 && p.x.abs() <= w && p.y.abs() <= w && p.z >= -slack && p.z <= w
}

/// Grove-level frustum + distance LOD. `None` means the whole grove is dropped.
pub fn grove_lod(
	record: GroveRecord,
	view: &GrassViewUniform,
	thresholds: LodThresholds,
) -> Option<u32> {
	if !record.is_active() {
		return None;
	}
	let (center, radius) = record.sphere();
	if !sphere_in_frustum(center, radius, view.clip_from_world) {
		return None;
	}
	let distance = view.world_position.truncate().distance(center);
	let bucket = lod_bucket(
		distance,
		record.height_m(),
		0.0,
		view.render_height_px(),
		view.fov_y_radians(),
		thresholds,
	);
	(bucket < 3).then_some(bucket)
}

/// Fill exclusive prefixes. Returns the compact item count for pass 2.
pub fn prefix_visible_spans(spans: &mut [VisibleGroveSpan]) -> u32 {
	let mut prefix = 0u32;
	for span in spans {
		span.prefix = prefix;
		prefix = prefix.saturating_add(span.count);
	}
	prefix
}

/// Map a compact pass-2 index onto `(item_index, lod)`.
pub fn span_item_index(compact_index: u32, spans: &[VisibleGroveSpan]) -> Option<(u32, u32)> {
	if spans.is_empty() {
		return None;
	}
	let mut lo = 0usize;
	let mut hi = spans.len();
	while lo < hi {
		let mid = (lo + hi) / 2;
		let span = spans[mid];
		if compact_index < span.prefix {
			hi = mid;
		} else if compact_index >= span.prefix.saturating_add(span.count) {
			lo = mid + 1;
		} else {
			return Some((span.item_index(compact_index), span.lod));
		}
	}
	None
}

/// Compact live groves that survive frustum + drop distance.
pub fn compact_grove_spans(
	groves: &[GroveRecord],
	view: &GrassViewUniform,
	thresholds: LodThresholds,
	span_of: impl Fn(GroveRecord) -> (u32, u32),
) -> Vec<VisibleGroveSpan> {
	let mut spans = Vec::new();
	for grove in groves {
		let Some(lod) = grove_lod(*grove, view, thresholds) else {
			continue;
		};
		let (offset, count) = span_of(*grove);
		if count == 0 {
			continue;
		}
		spans.push(VisibleGroveSpan { offset, count, prefix: 0, lod });
	}
	prefix_visible_spans(&mut spans);
	spans
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::wesl::hash::VegetationHash;

	#[test]
	fn nested_density_is_stable_across_distance() -> anyhow::Result<()> {
		let t = LodThresholds::field_default();
		assert!(t.nested_ok());
		assert!(LodThresholds::bush_default().nested_ok());
		assert!(LodThresholds::tree_default().nested_ok());
		assert!(LodThresholds::grove_tree().nested_ok());
		assert!(LodThresholds::grove_bush().nested_ok());
		assert!(LodThresholds::grove_grass().nested_ok());
		let grove = LodThresholds::grove_tree();
		assert_eq!(lod_bucket(50.0, 12.0, 0.0, 1080.0, 1.0, grove), GRASS_LOD_NEAR);
		assert_eq!(lod_bucket(200.0, 12.0, 0.0, 1080.0, 1.0, grove), GRASS_LOD_NEAR);
		assert_eq!(lod_bucket(450.0, 12.0, 0.0, 1080.0, 1.0, grove), GRASS_LOD_MID);
		assert_eq!(lod_bucket(900.0, 12.0, 0.0, 1080.0, 1.0, grove), GRASS_LOD_CARD);
		assert_eq!(lod_bucket(1_100.0, 12.0, 0.0, 1080.0, 1.0, grove), 3);
		assert_eq!(LodThresholds::grove_bush().card_m, LodThresholds::bush_default().card_m);
		assert_eq!(LodThresholds::grove_grass().card_m, LodThresholds::field_default().card_m);
		assert_eq!(
			lod_bucket(50.0, 0.55, 0.0, 720.0, 1.0, LodThresholds::grove_grass()),
			GRASS_LOD_NEAR
		);
		assert_eq!(lod_bucket(200.0, 0.55, 0.0, 720.0, 1.0, LodThresholds::grove_grass()), 3);
		assert_eq!(
			lod_bucket(50.0, 1.4, 0.0, 720.0, 1.0, LodThresholds::grove_bush()),
			GRASS_LOD_MID
		);
		assert_eq!(lod_bucket(200.0, 1.4, 0.0, 720.0, 1.0, LodThresholds::grove_bush()), 3);
		let mut far_ids = Vec::new();
		for i in 0..64u32 {
			let rank = VegetationHash::density_rank(VegetationHash::plant_id(11, i));
			let far = lod_bucket(70.0, 0.55, rank, 720.0, 1.0, t);
			if far == GRASS_LOD_CARD {
				far_ids.push(i);
				let mid = lod_bucket(30.0, 0.55, rank, 720.0, 1.0, t);
				assert!(mid == GRASS_LOD_MID || mid == GRASS_LOD_NEAR);
			}
		}
		assert!(!far_ids.is_empty());
		Ok(())
	}

	#[test]
	fn skyline_height_is_not_dropped_for_subpixel_width_alone() -> anyhow::Result<()> {
		let t = LodThresholds::field_default();
		// Distant but still tall in pixels (crest) stays at least a card.
		let bucket = lod_bucket(80.0, 0.7, 0.01, 1080.0, 1.0, t);
		assert_eq!(bucket, GRASS_LOD_CARD);
		Ok(())
	}

	#[test]
	fn reduced_resolution_shrinks_projected_size() -> anyhow::Result<()> {
		let full = projected_height_px(0.6, 20.0, 1080.0, 1.0);
		let half = projected_height_px(0.6, 20.0, 540.0, 1.0);
		assert!((full - 2.0 * half).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn span_prefix_maps_compact_index_onto_item() -> anyhow::Result<()> {
		let mut spans = [
			VisibleGroveSpan { offset: 10, count: 4, prefix: 0, lod: 0 },
			VisibleGroveSpan { offset: 80, count: 2, prefix: 0, lod: 2 },
		];
		assert_eq!(prefix_visible_spans(&mut spans), 6);
		assert_eq!(span_item_index(0, &spans), Some((10, 0)));
		assert_eq!(span_item_index(3, &spans), Some((13, 0)));
		assert_eq!(span_item_index(4, &spans), Some((80, 2)));
		assert_eq!(span_item_index(5, &spans), Some((81, 2)));
		assert_eq!(span_item_index(6, &spans), None);
		Ok(())
	}

	#[test]
	fn inactive_or_far_grove_is_dropped() -> anyhow::Result<()> {
		let live = GroveRecord::active(Vec3::ZERO, Vec3::splat(4.0), 1, (0, 8), (0, 16), (0, 32));
		let mut dead = live;
		dead.flags = 0;
		let mut view = GrassViewUniform {
			clip_from_world: Mat4::IDENTITY,
			world_from_clip: Mat4::IDENTITY,
			world_position: Vec3::new(0.0, 2.0, 8.0).extend(0.0),
			prev_world_position: Vec3::new(0.0, 2.0, 8.0).extend(0.0),
			resolution: bevy::math::Vec4::new(1280.0, 720.0, 1.0, 0.0),
			wind: bevy::math::Vec4::ZERO,
			lod_distances: bevy::math::Vec4::new(0.0, 18.0, 48.0, 90.0),
			lod_density: bevy::math::Vec4::new(1.0, 0.72, 0.38, 1.15),
			counts: bevy::math::Vec4::ZERO,
			light_direction: bevy::math::Vec4::new(0.4, 0.8, 0.3, 0.0),
		};
		view.clip_from_world = Mat4::IDENTITY;
		assert!(grove_lod(dead, &view, LodThresholds::field_default()).is_none());
		let mut far = live;
		far.aabb_min = Vec3::new(0.0, 0.0, 400.0);
		far.aabb_max = Vec3::new(4.0, 4.0, 404.0);
		view.world_position = Vec3::new(0.0, 2.0, 0.0).extend(0.0);
		assert!(grove_lod(far, &view, LodThresholds::grove_grass()).is_none());
		Ok(())
	}
}
