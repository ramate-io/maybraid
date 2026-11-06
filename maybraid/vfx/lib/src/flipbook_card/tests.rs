use std::time::Duration;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use bevy::time::TimeUpdateStrategy;

use crate::composition::{EffectRecipe, FlipbookPart};
use crate::effects::dust::Dust;
use crate::flipbook_card::{tick_dust, DustMaterial, VfxDust};
use crate::layers::dust::DustLayer;
use crate::lifecycle::{gate_vfx_instances, tick_vfx_instances};
use crate::packaged::{FlipbookManifest, PackagedFlipbook};
use crate::spawn::{spawn_vfx, Effect, VfxLayerLife, VfxSpawn};

fn manifest() -> Result<FlipbookManifest, Box<dyn std::error::Error>> {
	Ok(serde_json::from_str(include_str!(
		"../../../../assets/vfx/dust/rolling_wave.flipbook.json"
	))?)
}

fn dust_recipe(flipbook: Handle<PackagedFlipbook>, mesh: Handle<Mesh>) -> Dust {
	Dust::new(DustLayer::new(FlipbookPart { flipbook, mesh, duration: Dust::ROLLING_WAVE_DURATION }))
}

#[test]
fn interpolation_clamps_and_material_clocks_are_independent()
-> Result<(), Box<dyn std::error::Error>> {
	let asset = PackagedFlipbook { manifest: manifest()?, image: Handle::default() };
	assert!(DustMaterial::supports(&asset));
	let mut first = DustMaterial::new(&asset, Color::WHITE, 1.0);
	let second = DustMaterial::new(&asset, Color::WHITE, 1.0);
	first.set_age(&asset, 0.1);
	assert_eq!(first.time.truncate(), Vec3::new(1.0, 2.0, 0.5));
	assert_eq!(second.time.x, 0.0);
	assert_eq!(first.image, second.image);
	first.set_age(&asset, 50.0);
	assert_eq!(first.time.truncate(), Vec3::new(48.0, 48.0, 0.0));
	assert_eq!(first.center.truncate(), Vec3::new(0.0, 0.55, 0.0));
	let mut irregular = asset.clone();
	irregular.manifest.views[1].azimuth_degrees = 41.0;
	assert!(!DustMaterial::supports(&irregular));
	Ok(())
}

#[test]
fn concurrent_instances_pause_cancel_and_finish() -> Result<(), Box<dyn std::error::Error>> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()))
		.init_asset::<Image>()
		.init_asset::<Mesh>()
		.init_asset::<PackagedFlipbook>()
		.init_asset::<DustMaterial>()
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(100)))
		.add_systems(Update, (tick_dust, tick_vfx_instances).chain())
		.add_systems(PostUpdate, gate_vfx_instances);
	let image = app.world_mut().resource_mut::<Assets<Image>>().add(Image::default());
	let flipbook = app
		.world_mut()
		.resource_mut::<Assets<PackagedFlipbook>>()
		.add(PackagedFlipbook { manifest: manifest()?, image });
	let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(Rectangle::default());
	let definition = dust_recipe(flipbook, mesh).effect_definition();
	let slow = spawn_vfx(
		&mut app.world_mut().commands(),
		&definition,
		VfxSpawn { playback: 0.5, ..default() },
	);
	let fast = spawn_vfx(
		&mut app.world_mut().commands(),
		&definition,
		VfxSpawn { playback: 2.0, ..default() },
	);
	app.world_mut().flush();
	for _ in 0..6 {
		app.update();
	}
	let instances: Vec<_> = app
		.world_mut()
		.query::<(&crate::membership::VfxMemberOf, &VfxLayerLife, &MeshMaterial3d<DustMaterial>)>()
		.iter(app.world())
		.map(|(member, life, material)| (member.root(), life.age, material.0.clone()))
		.collect();
	assert_eq!(instances.len(), 2);
	let slow_state =
		instances.iter().find(|state| state.0 == slow).ok_or("missing slow instance")?;
	let fast_state =
		instances.iter().find(|state| state.0 == fast).ok_or("missing fast instance")?;
	assert!(slow_state.1 > 0.0);
	assert!((fast_state.1 - slow_state.1 * 4.0).abs() < 1e-5);
	assert_ne!(slow_state.2, fast_state.2);
	let materials = app.world().resource::<Assets<DustMaterial>>();
	assert_eq!(
		materials.get(&slow_state.2).ok_or("missing slow material")?.image,
		materials.get(&fast_state.2).ok_or("missing fast material")?.image
	);
	app.world_mut().resource_mut::<Time<Virtual>>().pause();
	app.update();
	let paused_age: f32 = app
		.world_mut()
		.query_filtered::<&VfxLayerLife, With<VfxDust>>()
		.iter(app.world())
		.map(|life| life.age)
		.sum();
	assert!((paused_age - slow_state.1 - fast_state.1).abs() < 1e-5);
	app.world_mut().despawn(slow);
	assert_eq!(
		app.world_mut().query_filtered::<Entity, With<VfxDust>>().iter(app.world()).count(),
		1
	);
	app.world_mut().resource_mut::<Time<Virtual>>().unpause();
	for _ in 0..50 {
		app.update();
	}
	assert!(app.world().get_entity(fast).is_err());
	assert_eq!(
		app.world_mut().query_filtered::<Entity, With<VfxDust>>().iter(app.world()).count(),
		0
	);
	Ok(())
}

#[test]
fn failed_asset_cancels_waiting_root_and_children() -> Result<(), Box<dyn std::error::Error>> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default(), crate::packaged::PackagedVfxPlugin))
		.init_asset::<Image>()
		.init_asset::<Mesh>()
		.init_asset::<DustMaterial>()
		.add_systems(Update, tick_dust);
	let flipbook =
		app.world().resource::<AssetServer>().load("missing-vfx-test/dust.flipbook.json");
	let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(Rectangle::default());
	let definition = dust_recipe(flipbook, mesh).effect_definition();
	let root = spawn_vfx(&mut app.world_mut().commands(), &definition, VfxSpawn::default());
	app.world_mut().flush();
	let deadline = std::time::Instant::now() + Duration::from_secs(5);
	while app.world().get_entity(root).is_ok() && std::time::Instant::now() < deadline {
		app.update();
		std::thread::sleep(Duration::from_millis(5));
	}
	assert!(
		app.world().get_entity(root).is_err(),
		"a failed load must not leave an immortal invisible root"
	);
	assert_eq!(
		app.world_mut().query_filtered::<Entity, With<VfxDust>>().iter(app.world()).count(),
		0
	);
	Ok(())
}

#[test]
fn spawn_scene_retains_the_dust_recipe_on_the_root() -> Result<(), Box<dyn std::error::Error>> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin))
		.init_asset::<Image>()
		.init_asset::<Mesh>()
		.init_asset::<PackagedFlipbook>();
	let image = app.world_mut().resource_mut::<Assets<Image>>().add(Image::default());
	let flipbook = app
		.world_mut()
		.resource_mut::<Assets<PackagedFlipbook>>()
		.add(PackagedFlipbook { manifest: manifest()?, image });
	let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(Rectangle::default());
	let recipe = dust_recipe(flipbook, mesh);
	let root = app
		.world_mut()
		.commands()
		.spawn_scene(Effect { recipe: recipe.clone(), spawn: VfxSpawn::default() })
		.id();
	app.world_mut().flush();
	let dust = app.world().get::<Dust>(root).ok_or("missing Dust recipe on root")?;
	assert_eq!(dust.layer.part.duration, Dust::ROLLING_WAVE_DURATION);
	assert!(app.world().get::<crate::spawn::VfxInstance>(root).is_some());
	Ok(())
}
