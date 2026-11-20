//! End-to-end CPU mirror of GPU select → compact → indirect submit.

use bevy::camera::CameraProjection;
use bevy::math::{Vec2, Vec3};
use bevy::prelude::{PerspectiveProjection, Transform};
use vegetation_grass_field::{
	GrassField, GrassFieldSettings, GrassPatch, GrassPatchLod, GrassSelector, SurfaceSample,
};
use vegetation_shaders::wesl::{DrawIndexedIndirect, GeometryLod, GrassViewUniform, LodThresholds};

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
		resolution: bevy::math::Vec4::new(1280.0, height_px, 1.0, 0.0),
		wind: bevy::math::Vec4::new(0.92, 0.38, 0.0, 0.22),
		lod_distances: {
			let t = LodThresholds::field_default();
			bevy::math::Vec4::new(0.85, t.near_m, t.mid_m, t.card_m)
		},
		lod_density: {
			let t = LodThresholds::field_default();
			bevy::math::Vec4::new(t.near_density, t.mid_density, t.card_density, t.min_height_px)
		},
		counts: bevy::math::Vec4::new(0.0, 65536.0, 0.0, 0.0),
		light_direction: bevy::math::Vec4::new(0.4, 0.8, 0.3, 0.0),
	}
}

#[test]
fn meadow_lod_reduces_submitted_work() -> anyhow::Result<()> {
	let settings = GrassFieldSettings::default();
	let meadow = GrassPatch::dense_field(7, Vec3::new(4.0, 0.0, 4.0), Vec2::splat(16.0), 0.22);
	let field = GrassField::rebuild([&meadow], &settings);
	assert!(field.blades.len() > 1_000);
	let ground = GrassSelector::from_view(
		look_view(Vec3::new(6.0, 0.55, 7.5), Vec3::new(10.0, 0.35, 12.0), 720.0),
		&settings,
	)
	.compact(&field.blades, &field.patches);
	let orbit = GrassSelector::from_view(
		look_view(Vec3::new(10.0, 3.2, 16.0), Vec3::new(8.0, 0.3, 8.0), 720.0),
		&settings,
	)
	.compact(&field.blades, &field.patches);
	let overhead = GrassSelector::from_view(
		look_view(Vec3::new(12.0, 22.0, 12.0), Vec3::new(12.0, 0.0, 12.0), 720.0),
		&settings,
	)
	.compact(&field.blades, &field.patches);
	let far = GrassSelector::from_view(
		look_view(Vec3::new(12.0, 8.0, 70.0), Vec3::new(12.0, 0.0, 12.0), 720.0),
		&settings,
	)
	.compact(&field.blades, &field.patches);
	assert!(ground.submitted > 0);
	assert!(orbit.triangle_count() <= ground.triangle_count());
	assert!(overhead.instance_count(0) <= orbit.instance_count(0));
	assert!(far.triangle_count() < overhead.triangle_count());
	assert_eq!(far.instance_count(0), 0);
	Ok(())
}

#[test]
fn generation_through_indirect_args() -> anyhow::Result<()> {
	let settings = GrassFieldSettings::default();
	let meadow = GrassPatch::dense_field(7, Vec3::new(-4.0, 0.0, -4.0), Vec2::splat(12.0), 0.3);
	let clump = GrassPatch::isolated_clump(3, Vec3::new(20.0, 1.0, -6.0), 20);
	let field = GrassField::rebuild([&meadow, &clump], &settings);
	assert!(!field.blades.is_empty());
	assert_eq!(field.patches.len(), 2);

	let selector = GrassSelector::from_view(
		look_view(Vec3::new(2.0, 1.6, 10.0), Vec3::new(2.0, 0.0, 2.0), 720.0),
		&settings,
	);
	let lists = selector.compact(&field.blades, &field.patches);
	let args = lists.indirect_args();
	assert_eq!(args[0].index_count, GeometryLod::Near.index_count());
	assert_eq!(args.iter().map(|arg| arg.instance_count).sum::<u32>(), lists.submitted);
	let packed: Vec<u8> = args.iter().flat_map(|arg| arg.to_bytes()).collect();
	assert_eq!(packed.len(), 60);
	assert_eq!(
		DrawIndexedIndirect::empty(30).to_bytes().len(),
		20,
		"indirect stride stays tightly packed"
	);
	Ok(())
}

#[test]
fn host_ultralow_emits_no_blades() -> anyhow::Result<()> {
	let settings = GrassFieldSettings::default();
	let patch = GrassPatch::dense_field(1, Vec3::ZERO, Vec2::splat(6.0), 0.4)
		.with_lod(GrassPatchLod::UltraLow);
	let field = GrassField::rebuild([&patch], &settings);
	assert!(field.blades.is_empty());
	Ok(())
}

#[test]
fn slope_and_negative_roots_survive_select() -> anyhow::Result<()> {
	let settings = GrassFieldSettings::default();
	let slope = GrassPatch::sloped_field(
		5,
		Vec3::new(-20.0, 2.0, -8.0),
		Vec2::splat(4.0),
		0.4,
		Vec3::new(0.3, 1.0, 0.0),
	);
	let field = GrassField::rebuild([&slope], &settings);
	assert!(field.blades.iter().all(|blade| blade.root.x < 0.0));
	assert!(field.blades.iter().any(|blade| blade.normal.x > 0.0));
	let lists = GrassSelector::from_view(
		look_view(Vec3::new(-18.0, 3.5, -2.0), Vec3::new(-18.0, 2.0, -6.0), 720.0),
		&settings,
	)
	.compact(&field.blades, &field.patches);
	assert!(lists.submitted > 0);
	Ok(())
}

#[test]
fn zero_density_surface_sample_is_skipped() -> anyhow::Result<()> {
	let settings = GrassFieldSettings::default();
	let mut patch = GrassPatch::new(
		1,
		vegetation_grass_field::GrassSpecies::meadow(),
		0.0,
		vec![SurfaceSample::planar(Vec3::ZERO)],
	);
	patch.coverage = 0.0;
	assert!(patch.descriptors().is_empty());
	let field = GrassField::rebuild([&patch], &settings);
	assert!(field.is_empty());
	Ok(())
}
