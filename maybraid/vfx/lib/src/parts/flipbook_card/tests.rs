use std::time::Duration;

use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use bevy::scene::{prelude::bsn, ScenePlugin};
use bevy::time::TimeUpdateStrategy;

use crate::composition::{EffectRecipe, FlipbookPart};
use crate::effects::dust::Dust;
use crate::layers::dust::DustLayer;
use crate::lifecycle::{gate_vfx_instances, tick_vfx_instances};
use crate::parts::flipbook_card::{
	resolve_flipbooks, tick_dust, DustMaterial, FlipbookProxyMesh, ResolvedFlipbook, VfxDust,
};
use crate::parts::packaged::{FlipbookManifest, PackagedFlipbook};
use crate::spawn::{spawn_vfx, Effect, VfxLayerLife, VfxSpawn};

fn manifest() -> Result<FlipbookManifest, Box<dyn std::error::Error>> {
	Ok(serde_json::from_str(include_str!(
		"../../../../../assets/vfx/dust/rolling_wave.flipbook.json"
	))?)
}

fn test_app() -> App {
	test_app_at(format!("{}/../../assets", env!("CARGO_MANIFEST_DIR")))
}

fn test_app_at(file_path: String) -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin { file_path, meta_check: bevy::asset::AssetMetaCheck::Never, ..default() },
		bevy::image::ImagePlugin::default(),
		crate::parts::packaged::PackagedVfxPlugin,
		ScenePlugin,
		TransformPlugin,
	))
	.init_asset::<Mesh>()
	.init_asset::<DustMaterial>()
	.init_resource::<FlipbookProxyMesh>()
	.add_message::<crate::VfxLoadFailure>()
	.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(100)))
	.add_systems(Update, (resolve_flipbooks, tick_dust, tick_vfx_instances).chain())
	.add_systems(PostUpdate, gate_vfx_instances);
	app.register_asset_loader(bevy::image::ImageLoader::new(
		bevy::image::CompressedImageFormats::BC,
	));
	app.finish();
	app.cleanup();
	app
}

fn wait_until(
	app: &mut App,
	ready: impl Fn(&World) -> bool,
) -> Result<(), Box<dyn std::error::Error>> {
	let deadline = std::time::Instant::now() + Duration::from_secs(15);
	while !ready(app.world()) {
		app.update();
		if std::time::Instant::now() >= deadline {
			return Err("timed out waiting for flipbook runtime".into());
		}
		std::thread::sleep(Duration::from_millis(1));
	}
	Ok(())
}

#[test]
fn interpolation_clamps_and_material_clocks_are_independent(
) -> Result<(), Box<dyn std::error::Error>> {
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
	let mut app = test_app();
	app.world_mut().resource_mut::<Time<Virtual>>().pause();
	let definition = Dust::rolling_wave().effect_definition();
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
	wait_until(&mut app, |world| {
		[slow, fast]
			.iter()
			.all(|root| world.get::<crate::VfxInstance>(*root).is_some_and(|i| i.armed))
	})?;
	let resolved: Vec<_> = app
		.world_mut()
		.query::<(&ResolvedFlipbook, &Mesh3d)>()
		.iter(app.world())
		.map(|(asset, mesh)| (asset.0.id(), mesh.0.id()))
		.collect();
	assert_eq!(resolved.len(), 2);
	assert_eq!(resolved[0], resolved[1], "asset and standard proxy are shared");
	assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 1);
	app.world_mut().resource_mut::<Time<Virtual>>().unpause();
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
		app.world_mut()
			.query_filtered::<Entity, With<VfxDust>>()
			.iter(app.world())
			.count(),
		1
	);
	app.world_mut().resource_mut::<Time<Virtual>>().unpause();
	for _ in 0..50 {
		app.update();
	}
	assert!(app.world().get_entity(fast).is_err());
	assert_eq!(
		app.world_mut()
			.query_filtered::<Entity, With<VfxDust>>()
			.iter(app.world())
			.count(),
		0
	);
	Ok(())
}

#[test]
fn failed_asset_cancels_waiting_root_and_children() -> Result<(), Box<dyn std::error::Error>> {
	let mut app = test_app();
	let definition =
		Dust::new(DustLayer::new(FlipbookPart::new("missing-vfx-test/dust.flipbook.json")))
			.effect_definition();
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
		app.world_mut()
			.query_filtered::<Entity, With<VfxDust>>()
			.iter(app.world())
			.count(),
		0
	);
	Ok(())
}

#[test]
fn spawn_scene_retains_the_dust_recipe_on_the_root() -> Result<(), Box<dyn std::error::Error>> {
	let mut app = test_app();
	let recipe = Dust::rolling_wave();
	let root = app
		.world_mut()
		.commands()
		.spawn_scene(Effect { recipe: recipe.clone(), spawn: VfxSpawn::default() })
		.id();
	app.world_mut().flush();
	let dust = app.world().get::<Dust>(root).ok_or("missing Dust recipe on root")?;
	assert_eq!(dust.layer.part.flipbook.path, Dust::ROLLING_WAVE);
	assert_eq!(app.world().get::<Transform>(root), Some(&Transform::IDENTITY));
	assert!(app.world().get::<crate::spawn::VfxInstance>(root).is_some());
	Ok(())
}

#[test]
fn scene_placement_is_preserved_in_either_order_and_layers_stay_relative(
) -> Result<(), Box<dyn std::error::Error>> {
	let mut app = test_app();
	let placement = Transform::from_xyz(3.0, 5.0, -7.0)
		.with_rotation(Quat::from_rotation_y(0.6))
		.with_scale(Vec3::new(13.0, 2.0, 4.0));
	let mut recipe = Dust::rolling_wave();
	recipe.layer.transform = Transform::from_xyz(1.0, 2.0, 3.0)
		.with_rotation(Quat::from_rotation_z(0.3))
		.with_scale(Vec3::new(2.0, 3.0, 4.0));
	recipe.layer.scale = 0.5;
	let settings = VfxSpawn {
		playback: 2.0,
		seed: Some(42),
		tint: Some(Color::srgb(0.2, 0.4, 0.6)),
		intensity: 1.5,
		..default()
	};
	let first = app
		.world_mut()
		.commands()
		.spawn_scene(bsn! {
			~{placement}
			@{Effect::new(recipe.clone()).with_spawn(settings.clone())}
		})
		.id();
	let second = app
		.world_mut()
		.commands()
		.spawn_scene(bsn! {
			@{Effect::new(recipe.clone()).with_spawn(settings)}
			~{placement}
		})
		.id();
	app.world_mut().flush();
	app.update();
	for root in [first, second] {
		assert_eq!(app.world().get::<Transform>(root), Some(&placement));
		assert!(app.world().get::<Dust>(root).is_some());
		let instance = app.world().get::<crate::VfxInstance>(root).ok_or("missing instance")?;
		assert_eq!((instance.seed, instance.playback), (42, 2.0));
		let child = *app
			.world()
			.get::<Children>(root)
			.ok_or("missing children")?
			.first()
			.ok_or("missing layer")?;
		let local = Transform {
			scale: recipe.layer.transform.scale * recipe.layer.scale,
			..recipe.layer.transform
		};
		assert_eq!(app.world().get::<Transform>(child), Some(&local));
		let actual = app
			.world()
			.get::<GlobalTransform>(child)
			.ok_or("missing global transform")?
			.to_matrix();
		assert!(actual.abs_diff_eq(placement.to_matrix() * local.to_matrix(), 1e-4));
		let dust = app.world().get::<VfxDust>(child).ok_or("missing flipbook")?;
		assert_eq!(dust.intensity, 1.5);
		assert_eq!(dust.tint, Color::srgb(0.2, 0.4, 0.6));
	}
	Ok(())
}

#[test]
fn delayed_only_flipbook_survives_realization_and_starts_at_frame_zero(
) -> Result<(), Box<dyn std::error::Error>> {
	let mut app = test_app();
	let mut recipe = Dust::rolling_wave();
	recipe.layer.delay = 0.3;
	let root = app.world_mut().commands().spawn_scene(Effect::new(recipe)).id();
	app.world_mut().flush();
	app.update();
	assert!(app.world().get::<crate::VfxInstance>(root).ok_or("missing root")?.armed);
	assert!(app.world().get::<Children>(root).is_none());
	wait_until(&mut app, |world| world.get::<Children>(root).is_some())?;
	let child = *app
		.world()
		.get::<Children>(root)
		.ok_or("missing children")?
		.first()
		.ok_or("missing child")?;
	assert_eq!(app.world().get::<VfxLayerLife>(child).ok_or("missing life")?.age, 0.0);
	wait_until(&mut app, |world| world.get::<Visibility>(child) == Some(&Visibility::Inherited))?;
	let life = app.world().get::<VfxLayerLife>(child).ok_or("missing loaded life")?;
	assert_eq!(life.age, 0.0);
	assert_eq!(life.duration, manifest()?.duration_seconds());
	assert!(!life.waiting_for_emitter);
	for _ in 0..40 {
		app.update();
	}
	assert!(app.world().get_entity(root).is_err());
	assert!(app.world().get_entity(child).is_err());
	Ok(())
}

#[test]
fn missing_image_keeps_clock_at_zero_and_cancellation_drops_children(
) -> Result<(), Box<dyn std::error::Error>> {
	let mut app = test_app();
	// Inject only the resolved runtime asset, holding back its image dependency.
	let image = app.world_mut().resource_mut::<Assets<Image>>().reserve_handle();
	let asset = app
		.world_mut()
		.resource_mut::<Assets<PackagedFlipbook>>()
		.add(PackagedFlipbook { manifest: manifest()?, image: image.clone() });
	let root = app.world_mut().commands().spawn_scene(Effect::new(Dust::rolling_wave())).id();
	app.world_mut().flush();
	let child = *app
		.world()
		.get::<Children>(root)
		.ok_or("missing children")?
		.first()
		.ok_or("missing child")?;
	app.world_mut().entity_mut(child).insert(ResolvedFlipbook(asset.clone()));
	for _ in 0..10 {
		app.update();
	}
	assert!(!app.world().get::<crate::VfxInstance>(root).ok_or("missing root")?.armed);
	assert_eq!(app.world().get::<VfxLayerLife>(child).ok_or("missing life")?.age, 0.0);
	assert_eq!(app.world().get::<Visibility>(child), Some(&Visibility::Hidden));
	let cancelled = app.world_mut().commands().spawn_scene(Effect::new(Dust::rolling_wave())).id();
	app.world_mut().flush();
	let waiting_child = *app
		.world()
		.get::<Children>(cancelled)
		.ok_or("missing waiting children")?
		.first()
		.ok_or("missing waiting child")?;
	app.world_mut().entity_mut(waiting_child).insert(ResolvedFlipbook(asset));
	app.update();
	assert!(
		app.world()
			.get::<VfxLayerLife>(waiting_child)
			.ok_or("missing waiting life")?
			.waiting_for_emitter
	);
	app.world_mut().despawn(cancelled);
	assert!(app.world().get_entity(waiting_child).is_err());
	app.world_mut()
		.resource_mut::<Assets<Image>>()
		.insert(image.id(), Image::default())?;
	wait_until(&mut app, |world| world.get::<Visibility>(child) == Some(&Visibility::Inherited))?;
	assert_eq!(app.world().get::<VfxLayerLife>(child).ok_or("missing life")?.age, 0.0);
	app.world_mut().despawn(root);
	assert!(app.world().get_entity(child).is_err());
	for _ in 0..5 {
		app.update();
	}
	assert_eq!(app.world().resource::<Assets<DustMaterial>>().len(), 0);
	assert_eq!(app.world().resource::<Assets<PackagedFlipbook>>().len(), 0);
	Ok(())
}

#[test]
fn cancelling_before_resolution_or_delay_never_resurrects_a_layer(
) -> Result<(), Box<dyn std::error::Error>> {
	for delay in [0.0, 10.0] {
		let mut app = test_app();
		let mut recipe = Dust::rolling_wave();
		recipe.layer.delay = delay;
		let root = app.world_mut().commands().spawn_scene(Effect::new(recipe)).id();
		app.world_mut().flush();
		app.world_mut().despawn(root);
		for _ in 0..5 {
			app.update();
		}
		assert_eq!(app.world_mut().query::<&VfxDust>().iter(app.world()).count(), 0);
		assert_eq!(app.world_mut().query::<&ResolvedFlipbook>().iter(app.world()).count(), 0);
		assert_eq!(app.world_mut().query::<&crate::VfxInstance>().iter(app.world()).count(), 0);
		assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 0);
	}
	Ok(())
}

#[test]
fn direct_unregistered_path_and_failed_image_cancel_even_delayed_layers(
) -> Result<(), Box<dyn std::error::Error>> {
	let dir = std::env::temp_dir().join(format!("vfx-dependency-{}", crate::seed::generate()));
	std::fs::create_dir_all(&dir)?;
	let mut fixture = manifest()?;
	fixture.image = "missing.ktx2".into();
	std::fs::write(dir.join("unregistered.flipbook.json"), serde_json::to_vec(&fixture)?)?;
	for delay in [0.0, 0.3] {
		let mut app = test_app_at(dir.to_string_lossy().into_owned());
		let mut recipe = Dust::new(DustLayer::new(FlipbookPart::new("unregistered.flipbook.json")));
		recipe.layer.delay = delay;
		let root = app.world_mut().commands().spawn_scene(Effect::new(recipe)).id();
		app.world_mut().flush();
		wait_until(&mut app, |world| world.get::<Children>(root).is_some())?;
		let child = *app
			.world()
			.get::<Children>(root)
			.ok_or("missing children")?
			.first()
			.ok_or("missing layer")?;
		wait_until(&mut app, |world| world.get_entity(root).is_err())?;
		assert!(app.world().get_entity(child).is_err());
		let failures = app.world().resource::<Messages<crate::VfxLoadFailure>>();
		let mut cursor = failures.get_cursor();
		let failure = cursor.read(failures).last().ok_or("missing failure diagnostic")?;
		assert_eq!(failure.root, root);
		assert_eq!(failure.effect, "dust");
		assert_eq!(failure.path, "unregistered.flipbook.json");
		assert!(failure.layer.contains("0 (unregistered.flipbook.json)"));
		assert!(failure.error.contains("missing.ktx2"));
	}
	std::fs::remove_dir_all(dir)?;
	Ok(())
}
