//! App tests for the message-driven LOD refresh pipeline.

mod test_utils;

use bevy::prelude::*;

use crate::lod_ref::LodNodePose;
use crate::scene::level::LodSceneLevel;
use crate::scene::refresh::LodCullRegionCursor;
use crate::scene::refresh::LodSceneRefreshLevel;

use test_utils::{
	app_bullseye_regions, app_core, app_dual_channel_levels, app_entities_only, app_open_lattice,
	app_spotlight_levels, app_spotlight_regions, host_level, move_viewer, pose, spawn_host,
	spawn_nested_pair, spawn_viewer, BullChan, CullChan, NewCullRegions, NewRegions, SpotChan,
};

#[test]
fn track_updates_pose_on_transform_change() -> anyhow::Result<()> {
	let mut app = app_core();
	let viewer = spawn_viewer(app.world_mut(), Vec3::ZERO);
	app.update();
	assert_eq!(pose(&app, viewer), (Vec3::ZERO, Vec3::ZERO));

	move_viewer(&mut app, viewer, Vec3::new(10.0, 0.0, 0.0));
	app.update();
	assert_eq!(pose(&app, viewer), (Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)));

	app.update();
	assert_eq!(
		pose(&app, viewer),
		(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)),
		"still frame must not collapse previous/current"
	);
	assert!(app.world().entity(viewer).get::<LodNodePose>().is_some());
	Ok(())
}

#[test]
fn spotlight_silent_until_translation_changes() -> anyhow::Result<()> {
	let mut app = app_spotlight_regions();
	spawn_viewer(app.world_mut(), Vec3::ZERO);
	app.update();
	assert!(app.world().resource::<NewRegions<SpotChan>>().regions.is_empty());

	app.update();
	assert!(app.world().resource::<NewRegions<SpotChan>>().regions.is_empty());
	Ok(())
}

#[test]
fn spotlight_emits_cube_on_translation() -> anyhow::Result<()> {
	let mut app = app_spotlight_regions();
	let viewer = spawn_viewer(app.world_mut(), Vec3::ZERO);
	app.update();

	move_viewer(&mut app, viewer, Vec3::new(10.0, 0.0, 0.0));
	app.update();
	let regions = &app.world().resource::<NewRegions<SpotChan>>().regions;
	assert_eq!(regions.len(), 1);
	assert_eq!(regions[0].min.x, -15.0);
	assert_eq!(regions[0].max.x, 35.0);
	Ok(())
}

#[test]
fn bullseye_silent_inside_inner_cell() -> anyhow::Result<()> {
	let mut app = app_bullseye_regions();
	let viewer = spawn_viewer(app.world_mut(), Vec3::new(10.0, 10.0, 10.0));
	app.update();

	move_viewer(&mut app, viewer, Vec3::new(20.0, 20.0, 20.0));
	app.update();
	assert!(app.world().resource::<NewRegions<BullChan>>().regions.is_empty());
	Ok(())
}

#[test]
fn bullseye_emits_outer_cube_on_cell_cross() -> anyhow::Result<()> {
	let mut app = app_bullseye_regions();
	let viewer = spawn_viewer(app.world_mut(), Vec3::new(-25.0, 0.0, 0.0));
	app.update();

	move_viewer(&mut app, viewer, Vec3::new(10.0, 10.0, 10.0));
	app.update();
	let regions = &app.world().resource::<NewRegions<BullChan>>().regions;
	assert_eq!(regions.len(), 1);
	// cell (0,0,0) for 50m → center 25; outer 500 → [-225, 275]
	assert_eq!(regions[0].min.x, -225.0);
	assert_eq!(regions[0].max.x, 275.0);
	Ok(())
}

#[test]
fn spotlight_writes_level_for_host_in_region() -> anyhow::Result<()> {
	let mut app = app_spotlight_levels();
	let viewer = spawn_viewer(app.world_mut(), Vec3::ZERO);
	let host = spawn_host(app.world_mut(), Vec3::ZERO, LodSceneLevel::UltraLow);
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::UltraLow);

	move_viewer(&mut app, viewer, Vec3::new(10.0, 0.0, 0.0));
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::High);
	Ok(())
}

#[test]
fn spotlight_skips_host_outside_region() -> anyhow::Result<()> {
	let mut app = app_spotlight_levels();
	let viewer = spawn_viewer(app.world_mut(), Vec3::ZERO);
	let host = spawn_host(app.world_mut(), Vec3::new(1000.0, 0.0, 0.0), LodSceneLevel::UltraLow);
	app.update();

	move_viewer(&mut app, viewer, Vec3::new(10.0, 0.0, 0.0));
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::UltraLow);
	Ok(())
}

#[test]
fn spotlight_bands_follow_viewer_distance() -> anyhow::Result<()> {
	let mut app = app_spotlight_levels();
	let viewer = spawn_viewer(app.world_mut(), Vec3::ZERO);
	let host = spawn_host(app.world_mut(), Vec3::ZERO, LodSceneLevel::UltraLow);
	app.update();

	move_viewer(&mut app, viewer, Vec3::new(10.0, 0.0, 0.0));
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::High);

	move_viewer(&mut app, viewer, Vec3::new(50.0, 0.0, 0.0));
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::Medium);

	move_viewer(&mut app, viewer, Vec3::new(100.0, 0.0, 0.0));
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::Low);
	Ok(())
}

#[test]
fn idle_frame_does_not_rewrite_level() -> anyhow::Result<()> {
	let mut app = app_spotlight_levels();
	let viewer = spawn_viewer(app.world_mut(), Vec3::ZERO);
	let host = spawn_host(app.world_mut(), Vec3::ZERO, LodSceneLevel::UltraLow);
	app.update();
	move_viewer(&mut app, viewer, Vec3::new(10.0, 0.0, 0.0));
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::High);
	assert_eq!(app.world().resource::<NewRegions<SpotChan>>().regions.len(), 1);

	app.update();
	assert!(app.world().resource::<NewRegions<SpotChan>>().regions.is_empty());
	assert_eq!(host_level(&app, host), LodSceneLevel::High);
	Ok(())
}

#[test]
fn dual_channel_small_move_is_spotlight_only() -> anyhow::Result<()> {
	let mut app = app_dual_channel_levels();
	let viewer = spawn_viewer(app.world_mut(), Vec3::new(10.0, 10.0, 10.0));
	let host = spawn_host(app.world_mut(), Vec3::ZERO, LodSceneLevel::UltraLow);
	app.update();

	move_viewer(&mut app, viewer, Vec3::new(20.0, 10.0, 10.0));
	app.update();
	assert_eq!(app.world().resource::<NewRegions<SpotChan>>().regions.len(), 1);
	assert!(app.world().resource::<NewRegions<BullChan>>().regions.is_empty());
	assert_eq!(host_level(&app, host), LodSceneLevel::High);
	Ok(())
}

#[test]
fn untyped_level_bus_keeps_max_across_messages() -> anyhow::Result<()> {
	let mut app = app_entities_only();
	let host = spawn_host(app.world_mut(), Vec3::ZERO, LodSceneLevel::UltraLow);

	app.world_mut()
		.write_message(LodSceneRefreshLevel { entity: host, level: LodSceneLevel::Low });
	app.world_mut()
		.write_message(LodSceneRefreshLevel { entity: host, level: LodSceneLevel::High });
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::High);
	Ok(())
}

#[test]
fn open_lattice_emits_while_camera_still() -> anyhow::Result<()> {
	let mut app = app_open_lattice();
	spawn_viewer(app.world_mut(), Vec3::ZERO);
	app.update();
	assert_eq!(app.world().resource::<NewCullRegions<CullChan>>().regions.len(), 1);
	let next_after_first = app.world().resource::<LodCullRegionCursor>().next;
	assert_eq!(next_after_first, 1);

	app.update();
	assert_eq!(app.world().resource::<NewCullRegions<CullChan>>().regions.len(), 1);
	assert_eq!(app.world().resource::<LodCullRegionCursor>().next, 2);
	assert_eq!(app.world().resource::<LodCullRegionCursor>().anchor_cell, Some(IVec3::ZERO));
	Ok(())
}

#[test]
fn nested_host_under_hidden_root_is_not_refreshed() -> anyhow::Result<()> {
	let mut app = app_spotlight_levels();
	let viewer = spawn_viewer(app.world_mut(), Vec3::ZERO);
	let (_parent, allowed, blocked) = spawn_nested_pair(app.world_mut());
	app.update();

	move_viewer(&mut app, viewer, Vec3::new(10.0, 0.0, 0.0));
	app.update();
	assert_eq!(host_level(&app, allowed), LodSceneLevel::High);
	assert_eq!(host_level(&app, blocked), LodSceneLevel::UltraLow);
	Ok(())
}
