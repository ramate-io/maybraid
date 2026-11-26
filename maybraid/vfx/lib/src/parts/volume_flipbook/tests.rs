use crate::lifecycle::{gate_vfx_instances, tick_vfx_instances};
use crate::parts::volume_flipbook::{
	resolve_volumes, sync_volume_placement, tick_volumes, ResolvedVolume, VolumeProxyMesh,
};
use crate::spawn::VfxLayerLife;
use crate::{
	spawn_vfx, EffectDefinition, EffectLayer, PackagedVfxPlugin, PackagedVolumeFlipbook,
	VfxInstance, VfxLoadFailure, VfxSpawn, VolumeFlipbookManifest, VolumeFlipbookMaterial,
};
use bevy::prelude::*;
use bevy::scene::ScenePlugin;
use bevy::time::TimeUpdateStrategy;
use std::time::{Duration, Instant};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const PATH: &str = "vfx/fixtures/volume_puffs/puffs.volume_flipbook.json";
fn manifest() -> Result<VolumeFlipbookManifest, Box<dyn std::error::Error>> {
	Ok(VolumeFlipbookManifest::from_json(include_bytes!(
		"../../../../../assets/vfx/fixtures/volume_puffs/puffs.volume_flipbook.json"
	))?)
}
fn app_at(file_path: String) -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin { file_path, meta_check: bevy::asset::AssetMetaCheck::Never, ..default() },
		bevy::image::ImagePlugin::default(),
		PackagedVfxPlugin,
		ScenePlugin,
		TransformPlugin,
	))
	.init_asset::<Mesh>()
	.init_asset::<VolumeFlipbookMaterial>()
	.init_resource::<VolumeProxyMesh>()
	.add_message::<VfxLoadFailure>()
	.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(50)))
	.add_systems(Update, (resolve_volumes, tick_volumes, tick_vfx_instances).chain())
	.add_systems(
		PostUpdate,
		(
			gate_vfx_instances,
			sync_volume_placement.after(bevy::transform::TransformSystems::Propagate),
		),
	);
	app.register_asset_loader(bevy::image::ImageLoader::new(
		bevy::image::CompressedImageFormats::NONE,
	));
	app.finish();
	app.cleanup();
	app
}
fn app() -> App {
	app_at(format!("{}/../../assets", env!("CARGO_MANIFEST_DIR")))
}
fn wait(app: &mut App, ready: impl Fn(&World) -> bool) -> TestResult {
	let start = Instant::now();
	while !ready(app.world()) {
		app.update();
		if start.elapsed() > Duration::from_secs(15) {
			return Err("volume readiness timed out".into());
		}
		std::thread::sleep(Duration::from_millis(1));
	}
	Ok(())
}
fn spawn(app: &mut App, path: &str, delay: f32, playback: f32) -> Entity {
	let definition = EffectDefinition::new(
		"test-volume",
		[EffectLayer::volume_flipbook_path(path).with_name("density").with_delay(delay)],
	);
	let root = spawn_vfx(
		&mut app.world_mut().commands(),
		&definition,
		VfxSpawn { playback, seed: Some(7), ..default() },
	);
	app.world_mut().flush();
	root
}
fn child(app: &App, root: Entity) -> Result<Entity, Box<dyn std::error::Error>> {
	Ok(*app
		.world()
		.get::<Children>(root)
		.and_then(|c| c.first())
		.ok_or("missing layer")?)
}

#[test]
fn validates_memory_layout_paths_and_timing() -> TestResult {
	let original = manifest()?;
	assert_eq!(original.byte_len(), Some(221184));
	for mutate in [
		|m: &mut VolumeFlipbookManifest| m.frames.clear(),
		|m: &mut VolumeFlipbookManifest| m.frames[0] = "../escape.ktx2".into(),
		|m: &mut VolumeFlipbookManifest| m.dimensions[0] = 129,
		|m: &mut VolumeFlipbookManifest| m.dimensions[2] = 1,
		|m: &mut VolumeFlipbookManifest| m.fps = f32::INFINITY,
		|m: &mut VolumeFlipbookManifest| m.index_to_local = [0.; 16],
		|m: &mut VolumeFlipbookManifest| m.frames = vec!["frame.ktx2".into(); 65],
		|m: &mut VolumeFlipbookManifest| {
			m.dimensions = [128; 3];
			m.frames = vec!["frame.ktx2".into(); 64];
		},
	] {
		let mut m = original.clone();
		mutate(&mut m);
		assert!(m.validate().is_err());
	}
	// A depth-one grid can be a valid static volume, but this ImageLoader maps it
	// to D2; it must never reach the volume material's D3 binding.
	let mut flat = original.clone();
	flat.dimensions[2] = 1;
	flat.bounds.max[2] = flat.bounds.min[2] + 3.2 / 24.;
	flat.frame_manifest(&flat.frames[0]).validate()?;
	assert!(flat.validate().is_err());
	let clock = original.clock_at(0.75);
	assert_eq!((clock.frame_0, clock.frame_1, clock.fract), (1., 2., 0.5));
	assert_eq!(original.clock_at(-5.).frame_0, 0.);
	assert_eq!(original.clock_at(99.).frame_1, 7.);
	assert_eq!(original.clock_at(99.).opacity, 0.);
	let texture = original.local_to_texture();
	assert!(texture.transform_point3(Vec3::splat(-1.6)).abs_diff_eq(Vec3::ZERO, 1e-6));
	assert!(texture.transform_point3(Vec3::splat(1.6)).abs_diff_eq(Vec3::ONE, 1e-6));
	Ok(())
}

#[test]
fn shares_real_images_but_keeps_clocks_materials_and_cleanup_independent() -> TestResult {
	let mut app = app();
	app.world_mut().resource_mut::<Time<Virtual>>().pause();
	let slow = spawn(&mut app, PATH, 0., 0.5);
	let fast = spawn(&mut app, PATH, 0., 2.);
	let a = child(&app, slow)?;
	let b = child(&app, fast)?;
	assert!(app.world().get::<VfxLayerLife>(a).ok_or("life")?.waiting_for_emitter);
	wait(&mut app, |w| {
		[slow, fast].iter().all(|r| w.get::<VfxInstance>(*r).is_some_and(|i| i.armed))
	})?;
	assert_eq!(app.world().get::<VfxLayerLife>(a).ok_or("life")?.age, 0.);
	assert_eq!(
		app.world().get::<ResolvedVolume>(a).ok_or("asset")?.0,
		app.world().get::<ResolvedVolume>(b).ok_or("asset")?.0
	);
	assert_eq!(app.world().get::<Mesh3d>(a), app.world().get::<Mesh3d>(b));
	let handle = app.world().get::<ResolvedVolume>(a).ok_or("asset")?.0.clone();
	let asset = app
		.world()
		.resource::<Assets<PackagedVolumeFlipbook>>()
		.get(&handle)
		.ok_or("loaded asset")?;
	assert_eq!(asset.frames.len(), 8);
	for frame in &asset.frames {
		let image = app.world().resource::<Assets<Image>>().get(frame).ok_or("image")?;
		assert_eq!(
			image.texture_descriptor.dimension,
			bevy::render::render_resource::TextureDimension::D3
		);
		assert_eq!(
			image.texture_descriptor.format,
			bevy::render::render_resource::TextureFormat::R16Float
		);
	}
	drop(handle);
	app.update(); // First visible frame stays at age zero even while paused.
	app.world_mut().resource_mut::<Time<Virtual>>().unpause();
	for _ in 0..8 {
		app.update();
	}
	let age_a = app.world().get::<VfxLayerLife>(a).ok_or("life")?.age;
	let age_b = app.world().get::<VfxLayerLife>(b).ok_or("life")?.age;
	assert!(age_a > 0.);
	assert!((age_b - 4. * age_a).abs() < 1e-5);
	assert_ne!(
		app.world().get::<MeshMaterial3d<VolumeFlipbookMaterial>>(a),
		app.world().get::<MeshMaterial3d<VolumeFlipbookMaterial>>(b)
	);
	app.world_mut().resource_mut::<Time<Virtual>>().pause();
	app.update();
	assert_eq!(app.world().get::<VfxLayerLife>(a).ok_or("life")?.age, age_a);
	app.world_mut().despawn(slow);
	assert!(app.world().get_entity(a).is_err());
	app.world_mut().resource_mut::<Time<Virtual>>().unpause();
	for _ in 0..90 {
		app.update();
	}
	assert!(app.world().get_entity(fast).is_err());
	for _ in 0..10 {
		app.update();
	}
	assert_eq!(app.world().resource::<Assets<VolumeFlipbookMaterial>>().len(), 0);
	assert_eq!(app.world().resource::<Assets<PackagedVolumeFlipbook>>().len(), 0);
	Ok(())
}

#[test]
fn delay_and_cancellation_never_start_or_resurrect_unready_layers() -> TestResult {
	let mut app = app();
	let root = spawn(&mut app, PATH, 0.3, 1.);
	assert!(app.world().get::<Children>(root).is_none());
	wait(&mut app, |w| w.get::<Children>(root).is_some())?;
	let layer = child(&app, root)?;
	assert_eq!(app.world().get::<VfxLayerLife>(layer).ok_or("life")?.age, 0.);
	wait(&mut app, |w| w.get::<Visibility>(layer) == Some(&Visibility::Inherited))?;
	assert_eq!(app.world().get::<VfxLayerLife>(layer).ok_or("life")?.age, 0.);
	app.world_mut().despawn(root);
	for delay in [0., 10.] {
		let root = spawn(&mut app, PATH, delay, 1.);
		app.world_mut().despawn(root);
	}
	for _ in 0..12 {
		app.update();
	}
	assert_eq!(app.world_mut().query::<&ResolvedVolume>().iter(app.world()).count(), 0);
	Ok(())
}

#[test]
fn failed_dependencies_report_recipe_layer_and_path_and_cancel_delayed_root() -> TestResult {
	let dir = std::env::temp_dir().join(format!("volume-failure-{}", crate::seed::generate()));
	std::fs::create_dir_all(&dir)?;
	let mut m = manifest()?;
	m.frames[0] = "missing.ktx2".into();
	std::fs::write(dir.join("broken.volume_flipbook.json"), serde_json::to_vec(&m)?)?;
	let mut app = app_at(dir.to_string_lossy().into_owned());
	let root = spawn(&mut app, "broken.volume_flipbook.json", 0.2, 1.);
	wait(&mut app, |w| w.get_entity(root).is_err())?;
	let messages = app.world().resource::<Messages<VfxLoadFailure>>();
	let mut cursor = messages.get_cursor();
	let failure = cursor.read(messages).last().ok_or("missing failure")?;
	assert_eq!(failure.effect, "test-volume");
	assert!(failure.layer.contains("density"));
	assert_eq!(failure.path, "broken.volume_flipbook.json");
	assert!(failure.error.contains("missing.ktx2"));
	std::fs::remove_dir_all(dir)?;
	Ok(())
}

#[test]
fn ray_coordinates_preserve_nested_nonuniform_and_mirrored_placement() -> TestResult {
	let m = manifest()?;
	let asset = PackagedVolumeFlipbook { manifest: m.clone(), frames: vec![Handle::default(); 8] };
	let parent = Transform::from_xyz(2., 3., 4.)
		.with_rotation(Quat::from_rotation_y(0.7))
		.with_scale(Vec3::new(-2., 3., 0.5));
	let child = Transform::from_xyz(0.3, 0.2, -0.5)
		.with_rotation(Quat::from_rotation_z(0.6))
		.with_scale(Vec3::splat(0.8));
	let world = GlobalTransform::from(parent).mul_transform(child);
	let mut material = VolumeFlipbookMaterial::new(&asset, Color::WHITE, 1.);
	material.set_transform(&world);
	let point = Vec3::new(0.1, -0.4, 0.7);
	let actual = material
		.local_to_texture
		.transform_point3(material.world_to_local.transform_point3(world.transform_point(point)));
	assert!(actual.abs_diff_eq(m.local_to_texture().transform_point3(point), 1e-5));
	material.set_transform(&GlobalTransform::from(Transform::from_scale(Vec3::ZERO)));
	assert_eq!(material.settings.w, 0.);
	assert!(material.world_to_local.is_finite());
	Ok(())
}

#[test]
fn missing_frame_pixels_hold_the_entire_instance_at_zero_until_ready() -> TestResult {
	let mut app = app();
	let root = spawn(&mut app, PATH, 0., 1.);
	let layer = child(&app, root)?;
	wait(&mut app, |w| w.get::<MeshMaterial3d<VolumeFlipbookMaterial>>(layer).is_some())?;
	let resolved = app.world().get::<ResolvedVolume>(layer).ok_or("resolved asset")?.0.clone();
	let frame = app
		.world()
		.resource::<Assets<PackagedVolumeFlipbook>>()
		.get(&resolved)
		.ok_or("volume")?
		.frames[7]
		.clone();
	let pixels = app
		.world_mut()
		.resource_mut::<Assets<Image>>()
		.remove(frame.id())
		.ok_or("image")?;
	for _ in 0..12 {
		app.update();
	}
	assert!(!app.world().get::<VfxInstance>(root).ok_or("root")?.armed);
	assert_eq!(app.world().get::<VfxLayerLife>(layer).ok_or("life")?.age, 0.);
	app.world_mut().resource_mut::<Assets<Image>>().insert(frame.id(), pixels)?;
	wait(&mut app, |w| w.get::<Visibility>(layer) == Some(&Visibility::Inherited))?;
	assert_eq!(app.world().get::<VfxLayerLife>(layer).ok_or("life")?.age, 0.);
	Ok(())
}
