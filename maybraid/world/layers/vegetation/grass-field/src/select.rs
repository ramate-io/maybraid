//! CPU selection matching the compute shader. Tests drive this path end to end.

use bevy::math::{Mat4, Vec3, Vec4};
use vegetation_shaders::wesl::{
	lod_bucket, BladeDescriptor, DrawIndexedIndirect, GeometryLod, GrassPatchHeader,
	GrassViewUniform, LodThresholds, GRASS_LOD_COUNT,
};

use crate::field::GrassFieldSettings;

/// Clip-space frustum matching `select.wesl`. Shared by grass compute and the bush follow-up.
#[derive(Clone, Copy, Debug)]
pub struct WorldFrustum {
	pub clip_from_world: Mat4,
	/// Gribb/Hartmann planes, retained for the bush follow-up's coarse patch culling.
	pub planes: [Vec4; 6],
}

impl WorldFrustum {
	/// World-space planes from a clip-from-world matrix.
	pub fn from_clip(clip_from_world: Mat4) -> Self {
		let c = clip_from_world;
		let col = |i: usize| c.col(i);
		Self {
			clip_from_world,
			planes: [
				col(3) + col(0),
				col(3) - col(0),
				col(3) + col(1),
				col(3) - col(1),
				col(3) + col(2),
				col(3) - col(2),
			],
		}
	}

	/// Matches `select.wesl` clip-space sphere test (includes wind slack).
	pub fn contains_sphere(self, center: Vec3, radius: f32) -> bool {
		let clip = self.clip_from_world;
		let p = clip * center.extend(1.0);
		let slack = radius * clip.x_axis.truncate().length() + 0.05;
		let w = p.w + slack;
		w > 0.0 && p.x.abs() <= w && p.y.abs() <= w && p.z >= -slack && p.z <= w
	}
}

/// Gribb/Hartmann world-space frustum planes from a clip-from-world matrix.
pub fn frustum_planes(clip_from_world: Mat4) -> [Vec4; 6] {
	WorldFrustum::from_clip(clip_from_world).planes
}

/// Compacted per-LOD instance lists plus overflow counts.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CompactedLists {
	pub indices: [Vec<u32>; GRASS_LOD_COUNT],
	pub overflow: [u32; GRASS_LOD_COUNT],
	pub considered: u32,
	pub submitted: u32,
}

impl CompactedLists {
	pub fn instance_count(&self, lod: usize) -> u32 {
		self.indices[lod].len() as u32
	}

	pub fn triangle_count(&self) -> u32 {
		GeometryLod::Near.index_count() / 3 * self.instance_count(0)
			+ GeometryLod::Mid.index_count() / 3 * self.instance_count(1)
			+ GeometryLod::Card.index_count() / 3 * self.instance_count(2)
	}

	/// Matches `finalize.wesl`: per-LOD `draw_indexed_indirect` args, no CPU readback on GPU.
	pub fn indirect_args(&self) -> [DrawIndexedIndirect; GRASS_LOD_COUNT] {
		[
			DrawIndexedIndirect::empty(GeometryLod::Near.index_count())
				.with_instances(self.instance_count(0)),
			DrawIndexedIndirect::empty(GeometryLod::Mid.index_count())
				.with_instances(self.instance_count(1)),
			DrawIndexedIndirect::empty(GeometryLod::Card.index_count())
				.with_instances(self.instance_count(2)),
		]
	}
}

/// Mirrors `select.wesl`. Rank-based overflow keeps identity deterministic.
pub struct GrassSelector {
	pub view: GrassViewUniform,
	pub thresholds: LodThresholds,
	pub capacities: [u32; GRASS_LOD_COUNT],
}

impl GrassSelector {
	pub fn from_view(view: GrassViewUniform, settings: &GrassFieldSettings) -> Self {
		Self {
			thresholds: LodThresholds {
				near_m: view.lod_distances.y,
				mid_m: view.lod_distances.z,
				card_m: view.lod_distances.w,
				near_density: view.lod_density.x,
				mid_density: view.lod_density.y,
				card_density: view.lod_density.z,
				min_height_px: view.lod_density.w,
			},
			view,
			capacities: [settings.max_per_lod; GRASS_LOD_COUNT],
		}
	}

	pub fn classify(&self, blade: &BladeDescriptor) -> Option<u32> {
		let camera = self.view.world_position.truncate();
		let radius = blade.conservative_extent_m(self.view.wind.w);
		let frustum = WorldFrustum::from_clip(self.view.clip_from_world);
		if !frustum.contains_sphere(blade.root + blade.normal * (blade.height_m * 0.5), radius) {
			return None;
		}
		let distance = camera.distance(blade.root);
		let bucket = lod_bucket(
			distance,
			blade.height_m,
			blade.density_rank(),
			self.view.render_height_px(),
			self.view.fov_y_radians(),
			self.thresholds,
		);
		GeometryLod::from_bucket(bucket).map(|lod| lod.bucket())
	}

	pub fn compact(
		&self,
		blades: &[BladeDescriptor],
		patches: &[GrassPatchHeader],
	) -> CompactedLists {
		self.compact_spans(blades, patches, None)
	}

	pub fn compact_spans(
		&self,
		blades: &[BladeDescriptor],
		patches: &[GrassPatchHeader],
		spans: Option<&[crate::grove_record::GroveVisibleSpan]>,
	) -> CompactedLists {
		if matches!(spans, Some(spans) if spans.is_empty()) {
			return CompactedLists::default();
		}
		let owned: Vec<GrassPatchHeader>;
		let (patches, blade_total): (&[GrassPatchHeader], u32) = match spans {
			None => (patches, blades.len() as u32),
			Some(spans) => {
				let total = spans.iter().map(|span| span.branch_count).sum();
				owned = spans
					.iter()
					.map(|span| GrassPatchHeader {
						aabb_min: Vec3::ZERO,
						blade_offset: span.branch_off,
						aabb_max: Vec3::ZERO,
						blade_count: span.branch_count,
						seed: 0,
						coverage: 1.0,
						species: 0,
						flags: GrassPatchHeader::FLAG_ACTIVE,
					})
					.collect();
				(&owned, total)
			}
		};
		let mut lists = CompactedLists::default();
		let keep_fraction = if blade_total == 0 {
			1.0
		} else {
			(self.capacities[0] as f32 / blade_total as f32).min(1.0)
		};
		for patch in patches {
			if !patch.is_active() {
				continue;
			}
			for index in patch.blade_range() {
				let Some(blade) = blades.get(index as usize) else {
					continue;
				};
				lists.considered += 1;
				if blade.height_m <= 0.0 {
					continue;
				}
				if blade.density_rank() >= keep_fraction {
					continue;
				}
				let Some(bucket) = self.classify(blade) else {
					continue;
				};
				let lod = bucket as usize;
				if lists.indices[lod].len() as u32 >= self.capacities[lod] {
					lists.overflow[lod] += 1;
					continue;
				}
				lists.indices[lod].push(index);
				lists.submitted += 1;
			}
		}
		lists
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::field::{GrassField, GrassFieldSettings};
	use crate::patch::GrassPatch;
	use bevy::camera::CameraProjection;
	use bevy::math::{Vec2, Vec4};
	use bevy::prelude::{PerspectiveProjection, Transform};

	fn look_view(origin: Vec3, target: Vec3, height_px: f32) -> GrassViewUniform {
		let view_from_world = Transform::from_translation(origin)
			.looking_at(target, Vec3::Y)
			.to_matrix()
			.inverse();
		let projection = PerspectiveProjection {
			fov: 1.0,
			aspect_ratio: 16.0 / 9.0,
			near: 0.1,
			far: 200.0,
			..Default::default()
		};
		let clip = projection.get_clip_from_view() * view_from_world;
		GrassViewUniform {
			clip_from_world: clip,
			world_from_clip: clip.inverse(),
			world_position: origin.extend(0.0),
			prev_world_position: origin.extend(0.0),
			resolution: Vec4::new(1280.0, height_px, 1.0, 0.0),
			wind: Vec4::new(0.92, 0.38, 0.0, 0.22),
			lod_distances: {
				let t = LodThresholds::field_default();
				Vec4::new(0.85, t.near_m, t.mid_m, t.card_m)
			},
			lod_density: {
				let t = LodThresholds::field_default();
				Vec4::new(t.near_density, t.mid_density, t.card_density, t.min_height_px)
			},
			counts: Vec4::new(0.0, 65536.0, 0.0, 0.0),
			light_direction: Vec4::new(0.4, 0.8, 0.3, 0.0),
		}
	}

	#[test]
	fn zero_blades_submits_nothing() -> anyhow::Result<()> {
		let settings = GrassFieldSettings::default();
		let selector = GrassSelector::from_view(
			look_view(Vec3::new(0.0, 2.0, 8.0), Vec3::ZERO, 720.0),
			&settings,
		);
		let lists = selector.compact(&[], &[]);
		assert_eq!(lists.submitted, 0);
		assert_eq!(lists.triangle_count(), 0);
		Ok(())
	}

	#[test]
	fn overflow_is_rank_stable() -> anyhow::Result<()> {
		let mut settings = GrassFieldSettings::default();
		settings.max_per_lod = 8;
		settings.max_blades = 64;
		let patch = GrassPatch::dense_field(3, Vec3::ZERO, Vec2::splat(6.0), 0.35);
		let field = GrassField::rebuild([&patch], &settings);
		let selector = GrassSelector::from_view(
			look_view(Vec3::new(3.0, 1.5, 10.0), Vec3::new(3.0, 0.0, 3.0), 720.0),
			&settings,
		);
		let a = selector.compact(&field.blades, &field.patches);
		let b = selector.compact(&field.blades, &field.patches);
		assert_eq!(a.indices, b.indices);
		assert!(a.instance_count(0) <= 8);
		Ok(())
	}

	#[test]
	fn lod_reduces_submitted_work() -> anyhow::Result<()> {
		let settings = GrassFieldSettings::default();
		let patch = GrassPatch::dense_field(5, Vec3::ZERO, Vec2::splat(10.0), 0.4);
		let field = GrassField::rebuild([&patch], &settings);
		let near = GrassSelector::from_view(
			look_view(Vec3::new(5.0, 1.2, 6.0), Vec3::new(5.0, 0.0, 5.0), 720.0),
			&settings,
		)
		.compact(&field.blades, &field.patches);
		let far = GrassSelector::from_view(
			look_view(Vec3::new(5.0, 12.0, 70.0), Vec3::new(5.0, 0.0, 5.0), 720.0),
			&settings,
		)
		.compact(&field.blades, &field.patches);
		assert!(far.triangle_count() < near.triangle_count());
		assert!(far.instance_count(0) < near.instance_count(0));
		Ok(())
	}

	#[test]
	fn off_camera_shadow_view_keeps_casters() -> anyhow::Result<()> {
		let settings = GrassFieldSettings::default();
		let patch = GrassPatch::isolated_clump(2, Vec3::new(40.0, 0.0, 0.0), 24);
		let field = GrassField::rebuild([&patch], &settings);
		let camera = GrassSelector::from_view(
			look_view(Vec3::new(-20.0, 2.0, 0.0), Vec3::new(-40.0, 0.0, 0.0), 720.0),
			&settings,
		)
		.compact(&field.blades, &field.patches);
		let mut shadow_view =
			look_view(Vec3::new(40.0, 4.0, 12.0), Vec3::new(40.0, 0.0, 0.0), 1024.0);
		shadow_view.resolution.w = 1.0;
		let shadow =
			GrassSelector::from_view(shadow_view, &settings).compact(&field.blades, &field.patches);
		assert_eq!(camera.submitted, 0);
		assert!(shadow.submitted > 0);
		Ok(())
	}

	#[test]
	fn reduced_resolution_drops_more_work() -> anyhow::Result<()> {
		let settings = GrassFieldSettings::default();
		let patch = GrassPatch::dense_field(8, Vec3::ZERO, Vec2::splat(12.0), 0.45);
		let field = GrassField::rebuild([&patch], &settings);
		let native = GrassSelector::from_view(
			look_view(Vec3::new(6.0, 8.0, 40.0), Vec3::new(6.0, 0.0, 6.0), 1080.0),
			&settings,
		)
		.compact(&field.blades, &field.patches);
		let half = GrassSelector::from_view(
			look_view(Vec3::new(6.0, 8.0, 40.0), Vec3::new(6.0, 0.0, 6.0), 540.0),
			&settings,
		)
		.compact(&field.blades, &field.patches);
		assert!(half.submitted <= native.submitted);
		Ok(())
	}

	#[test]
	fn culled_grove_span_list_is_zero_select_work() -> anyhow::Result<()> {
		let settings = GrassFieldSettings::default();
		let patch = GrassPatch::isolated_clump(1, Vec3::ZERO, 12);
		let field = GrassField::rebuild([&patch], &settings);
		let lists = GrassSelector::from_view(
			look_view(Vec3::new(0.0, 1.5, 6.0), Vec3::ZERO, 720.0),
			&settings,
		)
		.compact_spans(&field.blades, &field.patches, Some(&[]));
		assert_eq!(lists.considered, 0);
		assert_eq!(lists.submitted, 0);
		Ok(())
	}

	#[test]
	fn compact_writes_tight_indirect_args() -> anyhow::Result<()> {
		let settings = GrassFieldSettings::default();
		let patch = GrassPatch::isolated_clump(1, Vec3::ZERO, 12);
		let field = GrassField::rebuild([&patch], &settings);
		let lists = GrassSelector::from_view(
			look_view(Vec3::new(0.0, 1.5, 6.0), Vec3::ZERO, 720.0),
			&settings,
		)
		.compact(&field.blades, &field.patches);
		let args = lists.indirect_args();
		assert_eq!(args[0].index_count, GeometryLod::Near.index_count());
		assert_eq!(args[0].instance_count, lists.instance_count(0));
		assert_eq!(args[0].to_bytes().len(), 20);
		assert_eq!(
			lists.triangle_count(),
			args.iter().map(|a| a.index_count / 3 * a.instance_count).sum::<u32>()
		);
		Ok(())
	}
}
