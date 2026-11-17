//! Sphere catalog for world [`material_ref::MaterialRef`] recipes,
//! plus mythical-chain preview commands.

mod camera;
mod capture;
mod chain_capture;
mod chain_lighting;
mod chain_preview;
pub mod commands;
mod ground;
mod preview;
mod shape;
mod shoot;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use preview::PreviewConfig;

use avian3d::prelude::PhysicsPlugins;
use bevy::prelude::*;
use bikanwarda_encounter::chains::BikanwardaChainPhysicsPlugin;
use chain_capture::{
	tick_capture as tick_chain_capture, tick_video as tick_chain_video, CaptureSet, VideoCapture,
};
use chain_lighting::{setup_lighting, sync_lighting};
use chain_preview::{
	apply_pending_hit, pose_skinned_tube, sync_preview as sync_chain_preview, sync_visual_style,
	tick_sweep, ChainPreview, LightingMode, PendingChainHit,
};
use damage::DamagePlugin;
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use game_commands::GameCommandDrawerConfig;
use ground::setup_ground;
use maybraid_world::VisualStylePlugin;
use preview::{sync_preview, PreviewHost};
use projectiles::ProjectilesPlugin;
use shape::setup_meshes;
use shoot::{fly_shots, tick_shoot, ShootConfig};
use world_materials::WorldMaterialsPlugin;

pub struct WorldMaterialsPlaygroundPlugin;

impl Plugin for WorldMaterialsPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		let capturing = capture::CaptureSession::from_env();
		let mut command_plugin =
			GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config());
		if capturing.is_some() {
			command_plugin = command_plugin
				.with_drawer_config(GameCommandDrawerConfig { open_at_start: false, ..default() });
		}
		app.init_resource::<PreviewConfig>()
		.init_resource::<PreviewHost>()
		.init_resource::<ShootConfig>()
		.init_resource::<ChainPreview>()
		.init_resource::<LightingMode>()
		.init_resource::<PendingChainHit>()
		.init_resource::<CaptureSet>()
		.init_resource::<VideoCapture>()
		.add_plugins(PhysicsPlugins::default())
		.add_plugins(ProjectilesPlugin)
		.add_plugins(DamagePlugin)
		.add_plugins(BikanwardaChainPhysicsPlugin)
		.add_plugins(WorldMaterialsPlugin)
		.add_plugins(VisualStylePlugin)
		.add_plugins(command_plugin);
		if let Some(session) = capturing {
			app.insert_resource(session).add_systems(
				Startup,
				(
					capture::apply_column_preview,
					capture::hide_capture_hud.after(game_commands::ui::setup_debug_ui),
				),
			);
		}
		app.add_systems(
			Startup,
			(camera::setup_camera, setup_lighting, setup_ground, setup_meshes),
		)
		.add_systems(
			Update,
			(
				camera::release_modifiers_on_focus_change.before(camera::camera_controller),
				camera::camera_controller.run_if(not(resource_exists::<capture::CaptureSession>)),
				camera::apply_host_camera_frame
					.run_if(not(resource_exists::<capture::CaptureSession>)),
				sync_lighting,
				sync_visual_style,
				apply_pending_hit,
				tick_sweep,
				pose_skinned_tube,
				sync_preview.after(capture_command_line_input::<PlaygroundCommand>),
				sync_chain_preview.after(capture_command_line_input::<PlaygroundCommand>),
				tick_shoot.after(sync_preview),
				fly_shots,
				tick_chain_capture.after(sync_chain_preview),
				tick_chain_video.after(sync_chain_preview),
				chain_capture::hide_hud_during_evidence,
				ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
			),
		);
		if app.world().contains_resource::<capture::CaptureSession>() {
			app.add_systems(Update, capture::tick_capture.after(sync_preview));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::time::TimeUpdateStrategy;
	use bikanwarda_encounter::chains::{ChainAnchor, ChainRoot};
	use clap::Parser;
	use damage::Health;
	use game_commands::command::GameCommand;
	use projectiles::{Flight, ProjectileSource};
	use world_materials::chain::MythicalChainState;

	#[test]
	fn playground_app_builds_with_chain_physics_plugin() {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			TransformPlugin,
			bevy::mesh::MeshPlugin,
			PhysicsPlugins::default(),
			ProjectilesPlugin,
			DamagePlugin,
			BikanwardaChainPhysicsPlugin,
		));
		
		for _ in 0..5 {
			app.update();
		}
	}

	#[test]
	fn anchored_command_spawns_fixture_with_links_and_colliders() {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			TransformPlugin,
			bevy::mesh::MeshPlugin,
			PhysicsPlugins::default(),
			ProjectilesPlugin,
			DamagePlugin,
			BikanwardaChainPhysicsPlugin,
		));
		app.finish();
		
		let tension_mode = bikanwarda_encounter::chains::ChainTensionMode::Taut;
		commands::spawn_anchored_chain_fixture(&mut app.world_mut(), tension_mode);
		
		for _ in 0..3 {
			app.update();
		}
		
		let mut found_root = false;
		let mut found_anchors = 0;
		let mut found_joints = 0;
		let mut found_colliders = 0;
		
		for archetype in app.world().archetypes().iter() {
			for archetype_entity in archetype.entities() {
				let entity = archetype_entity.id();
				if app.world().get::<ChainRoot>(entity).is_some() {
					found_root = true;
				}
				if app.world().get::<ChainAnchor>(entity).is_some() {
					found_anchors += 1;
				}
				if let Some(name) = app.world().get::<Name>(entity) {
					if name.as_str().contains("chain-joint") {
						found_joints += 1;
					}
					if name.as_str().contains("chain-link") {
						found_colliders += 1;
					}
				}
			}
		}
		
		assert!(found_root, "Should spawn chain root");
		assert_eq!(found_anchors, 2, "Should spawn 2 anchors");
		assert_eq!(found_joints, 12, "Should spawn 12 joints");
		assert_eq!(found_colliders, 12, "Should spawn 12 colliders");
	}

	#[test]
	fn live_projectile_contact_yields_one_coalesced_damage_event() {
		use avian3d::prelude::*;
		use damage::HitPayload;
		use lod_avian::PhysicsInteractionLayer;
		
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			TransformPlugin,
			bevy::mesh::MeshPlugin,
			PhysicsPlugins::default(),
			ProjectilesPlugin,
			DamagePlugin,
			BikanwardaChainPhysicsPlugin,
		))
		.insert_resource(TimeUpdateStrategy::ManualDuration(std::time::Duration::from_millis(16)));
		app.finish();
		
		let tension_mode = bikanwarda_encounter::chains::ChainTensionMode::Catenary { sag_ratio: 10 };
		commands::spawn_anchored_chain_fixture(&mut app.world_mut(), tension_mode);
		
		for _ in 0..5 {
			app.update();
		}
		
		let mut chain_root = None;
		for archetype in app.world().archetypes().iter() {
			for archetype_entity in archetype.entities() {
				let entity = archetype_entity.id();
				if app.world().get::<ChainRoot>(entity).is_some() {
					chain_root = Some(entity);
					break;
				}
			}
		}
		
		let root = chain_root.expect("Chain root should exist");
		let initial_health = app.world().get::<Health>(root).unwrap().current;
		
		let damage_per_hit = 25.0;
		let projectile_start = Vec3::new(-2.0, 2.0, 0.0);
		let projectile_velocity = Vec3::new(15.0, 0.0, 0.0);
		let source = app.world_mut().spawn(Name::new("weapon")).id();
		
		let _projectile = app
			.world_mut()
			.spawn((
				Name::new("test-projectile"),
				Flight::spawn(projectile_start, 100.0, 100.0, 10.0),
				ProjectileSource(source),
				HitPayload { amount: damage_per_hit },
				Transform::from_translation(projectile_start),
				LinearVelocity(projectile_velocity),
				Collider::sphere(0.05),
				Sensor,
				PhysicsInteractionLayer::projectile_layers(),
				RigidBody::Dynamic,
				GravityScale(0.0),
			))
			.id();
		
		let mut damage_applied = false;
		for _ in 0..60 {
			app.update();
			
			if let Some(health) = app.world().entity(root).get::<Health>() {
				if health.current < initial_health {
					damage_applied = true;
					let damage_taken = initial_health - health.current;
					assert!(
						(damage_taken - damage_per_hit).abs() < 5.0,
						"Damage coalescing failed: expected ~{damage_per_hit}, got {damage_taken}"
					);
					break;
				}
			}
		}
		
		assert!(damage_applied, "Projectile should hit chain and apply damage");
	}

	#[test]
	fn parses_show_hex() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("show hex")?;
		let PlaygroundCommand::Show { recipe } = command else {
			return Err("expected show".into());
		};
		assert_eq!(recipe, "hex");
		Ok(())
	}

	#[test]
	fn parses_shoot() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("shoot")?;
		assert!(matches!(command, PlaygroundCommand::Shoot { action: None }));
		Ok(())
	}

	#[test]
	fn parses_shoot_stop() -> Result<(), String> {
		let command = PlaygroundCommand::try_parse_from(["materials", "shoot", "stop"])
			.map_err(|err| err.to_string())?;
		assert!(matches!(
			command,
			PlaygroundCommand::Shoot { action: Some(commands::ShootAction::Stop) }
		));
		Ok(())
	}

	#[test]
	fn parses_mesh_blast() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("mesh blast")?;
		let PlaygroundCommand::Mesh { shape } = command else {
			return Err("expected mesh".into());
		};
		assert_eq!(shape, crate::shape::PreviewShape::Blast);
		Ok(())
	}

	#[test]
	fn parses_mesh_ring() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("mesh ring")?;
		let PlaygroundCommand::Mesh { shape } = command else {
			return Err("expected mesh".into());
		};
		assert_eq!(shape, crate::shape::PreviewShape::Ring);
		Ok(())
	}

	#[test]
	fn parses_mesh_cylinder() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("mesh cylinder")?;
		let PlaygroundCommand::Mesh { shape } = command else {
			return Err("expected mesh".into());
		};
		assert_eq!(shape, crate::shape::PreviewShape::Cylinder);
		Ok(())
	}

	#[test]
	fn parses_mesh_taper() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("mesh taper")?;
		let PlaygroundCommand::Mesh { shape } = command else {
			return Err("expected mesh".into());
		};
		assert_eq!(shape, crate::shape::PreviewShape::Taper);
		Ok(())
	}

	#[test]
	fn show_doric_uses_the_column_recipe() -> Result<(), String> {
		let material = commands::material_for_show("doric");
		assert_eq!(material.name, material_ref::MaterialId::named("doric_column"));
		Ok(())
	}

	#[test]
	fn parses_chain_show_pair() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("chain show pair")?;
		let PlaygroundCommand::Chain { action: commands::ChainCommand::Show { state } } = command
		else {
			return Err("expected chain show".into());
		};
		assert_eq!(state, "pair");
		Ok(())
	}

	#[test]
	fn parses_chain_near_breaking_and_night() -> Result<(), String> {
		let show = <PlaygroundCommand as GameCommand>::parse_line("chain show near-breaking")?;
		assert!(matches!(
			show,
			PlaygroundCommand::Chain { action: commands::ChainCommand::Show { state } }
				if state == "near-breaking"
		));
		let lighting = <PlaygroundCommand as GameCommand>::parse_line("chain lighting night")?;
		assert!(matches!(
			lighting,
			PlaygroundCommand::Chain { action: commands::ChainCommand::Lighting { mode } }
				if mode == "night"
		));
		assert_eq!(
			MythicalChainState::from_label("near-breaking"),
			Some(MythicalChainState::NearBreaking)
		);
		Ok(())
	}

	#[test]
	fn parses_chain_sweep_stop() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("chain sweep stop")?;
		assert!(matches!(
			command,
			PlaygroundCommand::Chain { action: commands::ChainCommand::Sweep { action: Some(action) } }
				if action == "stop"
		));
		Ok(())
	}

	#[test]
	fn parses_chain_capture_dir() -> Result<(), String> {
		let command =
			<PlaygroundCommand as GameCommand>::parse_line("chain capture /tmp/chain-previews")?;
		assert!(matches!(
			command,
			PlaygroundCommand::Chain { action: commands::ChainCommand::Capture { dir: Some(dir) } }
				if dir.ends_with("chain-previews")
		));
		Ok(())
	}

	#[test]
	fn parses_chain_hit_style_and_video() -> Result<(), String> {
		let hit = <PlaygroundCommand as GameCommand>::parse_line("chain hit 6")?;
		assert!(matches!(
			hit,
			PlaygroundCommand::Chain { action: commands::ChainCommand::Hit { link: 6 } }
		));
		let style = <PlaygroundCommand as GameCommand>::parse_line("chain style retro")?;
		assert!(matches!(
			style,
			PlaygroundCommand::Chain { action: commands::ChainCommand::Style { mode } }
				if mode == "retro"
		));
		let video = <PlaygroundCommand as GameCommand>::parse_line("chain video /tmp/chain-video")?;
		assert!(matches!(
			video,
			PlaygroundCommand::Chain { action: commands::ChainCommand::Video { dir } }
				if dir.ends_with("chain-video")
		));
		Ok(())
	}

	#[test]
	fn parses_show_chain_alias() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("show chain")?;
		let PlaygroundCommand::Show { recipe } = command else {
			return Err("expected show".into());
		};
		assert_eq!(recipe, "chain");
		Ok(())
	}
}
