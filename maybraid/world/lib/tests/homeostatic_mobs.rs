//! Integration tests for homeostatic mob activation and behavior.
//! Exercises automated player-transform transitions through the real plugin stack.
//! Native locomotion and rendering remain separate runtime validation.

use bevy::prelude::*;
use maybraid_world::{WorldLayersPlugin, WorldMaterialRefPlugin};
use world_player::Player as WorldPlayer;

/// Minimal test app with full world layers (including homeostatic mobs)
fn test_app() -> App {
	let mut app = App::new();
	app.insert_resource(maybraid_virtual_pad_controller::PadHidPolicy::External);
	app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
		std::time::Duration::from_millis(100),
	));
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin::default(),
		bevy::mesh::MeshPlugin,
		bevy::state::app::StatesPlugin,
		bevy::scene::ScenePlugin,
		bevy::world_serialization::WorldSerializationPlugin,
		bevy::transform::TransformPlugin,
		bevy::input::InputPlugin,
		bevy::gizmos::GizmoPlugin,
		bevy::window::WindowPlugin { primary_window: None, ..default() },
	))
	.init_asset::<StandardMaterial>()
	// Player flinch/movement require the asset store; their audio backend remains optional.
	.init_asset::<maybraid_audio::AudioClip>()
	// Match the documented WorldLayers prerequisites and the strict shared camera fixture.
	// Input/window plugins supply their real resources without opening a native window.
	.add_plugins((
		world_materials::WorldMaterialsShadersPlugin,
		WorldMaterialRefPlugin,
		player_camera::PlayerCameraPlugin,
		WorldLayersPlugin,
	));
	app.finish();
	app.cleanup();
	app.world_mut().spawn((
		WorldPlayer,
		lod::LodViewer,
		Transform::default(),
		GlobalTransform::default(),
	));

	app
}

#[test]
fn entering_flagged_cell_activates_one_mob() -> anyhow::Result<()> {
	let mut app = test_app();

	// Find a flagged cell by checking the deterministic flag function
	let cell_size = 200.0;
	let cell_fraction = 0.33;
	let mut flagged_cell = None;

	for ix in 0..20 {
		for iz in 0..20 {
			let cell = barking::HomeostaticCellIndex(ix, iz);
			if cell.should_host_mob(cell_fraction) {
				flagged_cell = Some(cell);
				break;
			}
		}
		if flagged_cell.is_some() {
			break;
		}
	}

	let flagged_cell =
		flagged_cell.ok_or_else(|| anyhow::anyhow!("should find at least one flagged cell"))?;
	let cell_center = flagged_cell.center(cell_size);

	// Move player to the flagged cell
	let mut player_query = app.world_mut().query_filtered::<&mut Transform, With<WorldPlayer>>();
	if let Ok(mut transform) = player_query.single_mut(app.world_mut()) {
		transform.translation = cell_center;
	}

	// Run updates to allow activation
	for _ in 0..20 {
		app.update();
	}

	// Check that exactly one homeostatic mob was activated
	let active_mobs = app.world_mut().query::<&barking::HomeostaticMob>().iter(app.world()).count();

	assert_eq!(active_mobs, 1, "entering flagged cell should activate exactly one homeostatic mob");

	Ok(())
}

#[test]
fn standing_in_neighbor_cell_does_not_activate() -> anyhow::Result<()> {
	let mut app = test_app();

	let cell_size = 200.0;
	let cell_fraction = 0.33;

	// Find a flagged cell with an unflagged neighbor
	let mut test_cells = None;
	for ix in 0..20 {
		for iz in 0..20 {
			let cell = barking::HomeostaticCellIndex(ix, iz);
			if cell.should_host_mob(cell_fraction) {
				let neighbor = barking::HomeostaticCellIndex(ix + 1, iz);
				if !neighbor.should_host_mob(cell_fraction) {
					test_cells = Some((cell, neighbor));
					break;
				}
			}
		}
		if test_cells.is_some() {
			break;
		}
	}

	let (_, unflagged_neighbor) = test_cells
		.ok_or_else(|| anyhow::anyhow!("should find flagged cell with unflagged neighbor"))?;
	let neighbor_center = unflagged_neighbor.center(cell_size);

	// Move player to the UNFLAGGED neighbor cell
	let mut player_query = app.world_mut().query_filtered::<&mut Transform, With<WorldPlayer>>();
	if let Ok(mut transform) = player_query.single_mut(app.world_mut()) {
		transform.translation = neighbor_center;
	}

	// Run updates
	for _ in 0..20 {
		app.update();
	}

	// Check that no homeostatic mob was activated
	let active_mobs = app.world_mut().query::<&barking::HomeostaticMob>().iter(app.world()).count();

	assert_eq!(
		active_mobs, 0,
		"standing in unflagged neighbor cell should not activate homeostatic mob"
	);

	Ok(())
}

#[test]
fn reentering_cell_reactivates_mob_without_duplicates() -> anyhow::Result<()> {
	let mut app = test_app();

	let cell_size = 200.0;
	let cell_fraction = 0.33;

	// Find a flagged cell
	let mut flagged_cell = None;
	for ix in 0..20 {
		for iz in 0..20 {
			let cell = barking::HomeostaticCellIndex(ix, iz);
			if cell.should_host_mob(cell_fraction) {
				flagged_cell = Some(cell);
				break;
			}
		}
		if flagged_cell.is_some() {
			break;
		}
	}

	let flagged_cell =
		flagged_cell.ok_or_else(|| anyhow::anyhow!("should find at least one flagged cell"))?;
	let cell_center = flagged_cell.center(cell_size);

	// First entry: activate mob
	{
		let mut player_query =
			app.world_mut().query_filtered::<&mut Transform, With<WorldPlayer>>();
		if let Ok(mut transform) = player_query.single_mut(app.world_mut()) {
			transform.translation = cell_center;
		}
	}

	for _ in 0..20 {
		app.update();
	}

	let first_activation_count =
		app.world_mut().query::<&barking::HomeostaticMob>().iter(app.world()).count();
	assert_eq!(first_activation_count, 1, "first entry should activate one mob");
	let first_host = app
		.world_mut()
		.query_filtered::<Entity, With<barking::HomeostaticMob>>()
		.single(app.world())
		.map_err(|error| anyhow::anyhow!("expected one first encounter: {error}"))?;

	// Leave the 3×3 neighborhood
	{
		let mut player_query =
			app.world_mut().query_filtered::<&mut Transform, With<WorldPlayer>>();
		if let Ok(mut transform) = player_query.single_mut(app.world_mut()) {
			transform.translation = cell_center + Vec3::new(1000.0, 0.0, 1000.0);
		}
	}

	for _ in 0..20 {
		app.update();
	}

	// The destination may legitimately activate its own encounter. Only the
	// original host must retire after leaving its neighborhood.
	assert!(app.world().get_entity(first_host).is_err(), "original encounter should retire");

	// Re-enter the same flagged cell
	{
		let mut player_query =
			app.world_mut().query_filtered::<&mut Transform, With<WorldPlayer>>();
		if let Ok(mut transform) = player_query.single_mut(app.world_mut()) {
			transform.translation = cell_center;
		}
	}

	for _ in 0..20 {
		app.update();
	}

	let reactivation_count =
		app.world_mut().query::<&barking::HomeostaticMob>().iter(app.world()).count();
	assert_eq!(
		reactivation_count, 1,
		"re-entering should reactivate exactly one mob (no duplicates)"
	);

	Ok(())
}

#[test]
fn world_layers_install_shared_and_homeostatic_owners() {
	let mut app = test_app();

	// Just verify the homeostatic plugin can coexist with other layers
	// by running the full plugin stack without errors
	for _ in 0..10 {
		app.update();
	}

	assert!(app.world().contains_resource::<lod::hcsg::HcsgStorage>());
	assert!(app.world().contains_resource::<barking::HomeostaticMobLifecycle>());
}

#[test]
fn home_session_teardown_cleans_hosts_and_members_without_destroying_the_player(
) -> anyhow::Result<()> {
	use damage::Health;
	use maybraid_controllers_core::InputRouter;
	use maybraid_session::{SessionCapabilities, WorldGameplayEnabled};
	use mob_intelligence::MemberOf;
	let mut app = test_app();
	let mut session = SessionCapabilities::default();
	session.install_play(true);
	app.insert_resource(session);
	let cell = (0..20)
		.flat_map(|x| (0..20).map(move |z| barking::HomeostaticCellIndex(x, z)))
		.find(|cell| cell.should_host_mob(0.33))
		.ok_or_else(|| anyhow::anyhow!("missing flagged test cell"))?;
	let player = app
		.world_mut()
		.query_filtered::<Entity, With<WorldPlayer>>()
		.single(app.world())
		.map_err(|error| anyhow::anyhow!("{error:?}"))?;
	app.world_mut()
		.entity_mut(player)
		.insert(Transform::from_translation(cell.center(200.0)));
	for _ in 0..20 {
		app.update();
	}
	// Drive real elapsed-time LOD pulses, bounded by simulated frames rather than wall-clock speed.
	for _ in 0..200 {
		app.update();
		let mut members = app.world_mut().query::<(&mob_intelligence::MemberOf, &damage::Health)>();
		if members
			.iter(app.world())
			.any(|(member, _)| app.world().get::<barking::HomeostaticMob>(member.mob).is_some())
		{
			break;
		}
	}
	let hosts: Vec<_> = app
		.world_mut()
		.query_filtered::<Entity, With<barking::HomeostaticMob>>()
		.iter(app.world())
		.collect();
	assert_eq!(hosts.len(), 1);
	let host = hosts[0];
	let mut bodies: Vec<_> = app
		.world_mut()
		.query::<(Entity, &MemberOf, &Health)>()
		.iter(app.world())
		.filter(|(_, member, _)| member.mob == host)
		.map(|(entity, _, health)| (entity, *health))
		.collect();
	assert!(!bodies.is_empty(), "fixture must materialize actual members before testing teardown");
	{
		let (entity, recorded) =
			bodies.first_mut().ok_or_else(|| anyhow::anyhow!("no body to injure"))?;
		let mut health = app
			.world_mut()
			.get_mut::<Health>(*entity)
			.ok_or_else(|| anyhow::anyhow!("missing body health"))?;
		health.take_damage(1.0);
		*recorded = *health;
	}

	let mut router = InputRouter::default();
	router.set_paused(true);
	assert!(!session.gameplay_enabled(&router));
	app.insert_resource(WorldGameplayEnabled(session.gameplay_enabled(&router)));
	for _ in 0..3 {
		app.update();
	}
	assert!(app.world().get_entity(host).is_ok(), "pause preserves the installed encounter");
	for (entity, health) in &bodies {
		assert_eq!(app.world().get::<Health>(*entity), Some(health));
	}
	router.set_paused(false);
	app.insert_resource(WorldGameplayEnabled(session.gameplay_enabled(&router)));
	app.update();
	assert!(app.world().get_entity(host).is_ok());

	app.world_mut().resource_mut::<SessionCapabilities>().install_menu_shell();
	for _ in 0..20 {
		app.update();
	}
	assert!(app.world().get_entity(player).is_ok());
	assert!(app.world().get_entity(host).is_err());
	for (entity, _) in bodies {
		assert!(app.world().get_entity(entity).is_err());
	}
	assert_eq!(app.world().resource::<barking::PlayerHomeostaticCell>().current, None);
	assert_eq!(app.world_mut().query::<&barking::HomeostaticMob>().iter(app.world()).count(), 0);

	app.world_mut().resource_mut::<SessionCapabilities>().install_play(true);
	for _ in 0..20 {
		app.update();
	}
	assert_eq!(app.world_mut().query::<&barking::HomeostaticMob>().iter(app.world()).count(), 1);
	Ok(())
}
