//! End-to-end CPU mirror of GPU select → compact → indirect submit.

use bevy::camera::CameraProjection;
use bevy::math::{Vec2, Vec3, Vec4};
use bevy::prelude::{PerspectiveProjection, Transform};
use vegetation_bush_field::{
	BushField, BushFieldSettings, BushPatch, BushPatchLod, BushRecipe, BushSelector, WoodLod,
};
use vegetation_groves::bush_scrub;
use vegetation_shaders::wesl::{GrassViewUniform, LodThresholds};

fn look_view(origin: Vec3, target: Vec3, height_px: f32) -> GrassViewUniform {
	let view_from_world = Transform::from_translation(origin)
		.looking_at(target, Vec3::Y)
		.to_matrix()
		.inverse();
	let projection = PerspectiveProjection {
		fov: 1.0,
		aspect_ratio: 16.0 / 9.0,
		near: 0.1,
		far: 250.0,
		..Default::default()
	};
	let clip = projection.get_clip_from_view() * view_from_world;
	let t = LodThresholds::bush_default();
	GrassViewUniform {
		clip_from_world: clip,
		world_from_clip: clip.inverse(),
		world_position: origin.extend(0.0),
		prev_world_position: origin.extend(0.0),
		resolution: Vec4::new(1280.0, height_px, 1.0, 0.0),
		wind: Vec4::new(0.92, 0.38, 0.0, 0.22),
		lod_distances: Vec4::new(0.85, t.near_m, t.mid_m, t.card_m),
		lod_density: Vec4::new(t.near_density, t.mid_density, t.card_density, t.min_height_px),
		counts: Vec4::new(0.0, 16384.0, 0.0, 0.0),
		light_direction: Vec4::new(0.4, 0.8, 0.3, 0.0),
	}
}

#[test]
fn wood_and_foliage_draw_together() -> anyhow::Result<()> {
	let settings = BushFieldSettings::default();
	let patch = BushPatch::isolated(3, Vec3::ZERO, BushRecipe::small_scrub());
	let field = BushField::rebuild([&patch], &settings);
	assert!(!field.branches.is_empty());
	assert!(!field.leaves.is_empty());
	let lists = BushSelector::from_view(
		look_view(Vec3::new(0.0, 1.2, 3.5), Vec3::new(0.0, 0.4, 0.0), 720.0),
		&settings,
	)
	.compact(&field.plants, &field.branches, &field.leaves);
	assert!(lists.submitted_wood > 0);
	assert!(lists.submitted_foliage > 0);
	let wood = lists.wood_indirect_args();
	let foliage = lists.foliage_indirect_args();
	assert_eq!(wood[0].index_count, WoodLod::Near.index_count());
	assert_eq!(foliage[0].index_count, 6);
	assert_eq!(wood[0].to_bytes().len(), 20);
	Ok(())
}

#[test]
fn lod_reduces_branch_and_leaf_work() -> anyhow::Result<()> {
	let settings = BushFieldSettings::default();
	let patch = BushPatch::dense_scrub(7, Vec3::new(2.0, 0.0, 2.0), Vec2::splat(12.0), 1.4);
	let field = BushField::rebuild([&patch], &settings);
	assert!(field.plants.len() > 8);
	let near = BushSelector::from_view(
		look_view(Vec3::new(4.0, 1.0, 5.0), Vec3::new(6.0, 0.4, 6.0), 720.0),
		&settings,
	)
	.compact(&field.plants, &field.branches, &field.leaves);
	let far = BushSelector::from_view(
		look_view(Vec3::new(6.0, 12.0, 85.0), Vec3::new(6.0, 0.0, 6.0), 720.0),
		&settings,
	)
	.compact(&field.plants, &field.branches, &field.leaves);
	assert!(far.triangle_count() < near.triangle_count());
	assert!(far.wood_count(0) <= near.wood_count(0));
	Ok(())
}

#[test]
fn ultralow_host_keeps_a_crown_proxy() -> anyhow::Result<()> {
	let settings = BushFieldSettings::default();
	let patch = BushPatch::dense_scrub(5, Vec3::ZERO, Vec2::splat(8.0), 1.5)
		.with_lod(BushPatchLod::UltraLow);
	let field = BushField::rebuild([&patch], &settings);
	assert!(!field.plants.is_empty());
	assert!(!field.branches.is_empty());
	Ok(())
}

#[test]
fn grove_cell_placements_map_onto_gpu_plants() -> anyhow::Result<()> {
	let cell = bush_scrub::definition().cell_extent_xz.x;
	assert!((cell - 2.5).abs() < 1e-4);
	let placements = [
		(Vec3::new(0.0, 0.0, 0.0), 1.0),
		(Vec3::new(cell, 0.0, 0.0), 1.05),
		(Vec3::new(0.0, 0.0, cell), 0.92),
	];
	let patch = BushPatch::from_scrub_placements(11, &placements, BushRecipe::small_scrub());
	let field = BushField::rebuild([&patch], &BushFieldSettings::default());
	assert_eq!(field.plants.len(), 3);
	assert!((field.plants[1].root.x - cell).abs() < 1e-4);
	Ok(())
}

#[test]
fn buffer_limits_truncate_without_stale_parents() -> anyhow::Result<()> {
	let mut settings = BushFieldSettings::default();
	settings.max_plants = 2;
	let patch = BushPatch::dense_scrub(2, Vec3::ZERO, Vec2::splat(10.0), 1.2);
	let field = BushField::rebuild([&patch], &settings);
	assert!(field.plants.len() <= 2);
	for branch in &field.branches {
		assert!((branch.plant_index as usize) < field.plants.len());
		if !branch.is_trunk() {
			assert!((branch.parent_index as usize) < field.branches.len());
		}
	}
	Ok(())
}

#[test]
fn reduced_resolution_drops_more_work() -> anyhow::Result<()> {
	let settings = BushFieldSettings::default();
	let patch = BushPatch::dense_scrub(9, Vec3::ZERO, Vec2::splat(10.0), 1.3);
	let field = BushField::rebuild([&patch], &settings);
	let native = BushSelector::from_view(
		look_view(Vec3::new(5.0, 8.0, 40.0), Vec3::new(5.0, 0.0, 5.0), 1080.0),
		&settings,
	)
	.compact(&field.plants, &field.branches, &field.leaves);
	let half = BushSelector::from_view(
		look_view(Vec3::new(5.0, 8.0, 40.0), Vec3::new(5.0, 0.0, 5.0), 540.0),
		&settings,
	)
	.compact(&field.plants, &field.branches, &field.leaves);
	assert!(half.submitted() <= native.submitted());
	Ok(())
}

#[test]
fn cpu_select_reports_10m_scrub_work() -> anyhow::Result<()> {
	let settings = BushFieldSettings::default();
	let patch = BushPatch::dense_scrub(7, Vec3::ZERO, Vec2::splat(10.0), 1.35);
	let field = BushField::rebuild([&patch], &settings);
	let lists = BushSelector::from_view(
		look_view(Vec3::new(5.0, 1.2, 8.0), Vec3::new(5.0, 0.35, 5.0), 720.0),
		&settings,
	)
	.compact(&field.plants, &field.branches, &field.leaves);
	let plant_bytes = field.plants.len() * 96;
	let branch_bytes = field.branches.len() * 80;
	let leaf_bytes = field.leaves.len() * 64;
	eprintln!(
		"cpu-select 10m@1.35m: plants={} wood={} leaves={} submitted_wood={} submitted_foliage={} tris={} resident_B={}",
		field.plants.len(),
		field.branches.len(),
		field.leaves.len(),
		lists.submitted_wood,
		lists.submitted_foliage,
		lists.triangle_count(),
		plant_bytes + branch_bytes + leaf_bytes,
	);
	assert!(!field.plants.is_empty());
	assert!(lists.submitted_wood > 0);
	assert!(lists.submitted_foliage > 0);
	Ok(())
}
