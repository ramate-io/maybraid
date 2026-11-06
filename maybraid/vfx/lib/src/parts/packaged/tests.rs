use std::path::Path;
use std::time::{Duration, Instant};

use bevy::asset::{
	AssetMetaCheck, AssetPlugin, LoadState, RecursiveDependencyLoadState, UntypedAssetId,
};
use bevy::image::{
	CompressedImageFormats, ImageAddressMode, ImageFilterMode, ImageLoader, ImagePlugin,
	ImageSampler,
};
use bevy::prelude::*;
use bevy::render::render_resource::{TextureDimension, TextureFormat, TextureViewDimension};

use crate::parts::packaged::loaders::KtxHeader;
use crate::parts::packaged::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

struct Fixture;

impl Fixture {
	fn sampler(image: &Image) -> TestResult {
		let ImageSampler::Descriptor(sampler) = &image.sampler else {
			return Err("packaged texture must use an explicit sampler".into());
		};
		assert_eq!(sampler.address_mode_u, ImageAddressMode::ClampToEdge);
		assert_eq!(sampler.address_mode_v, ImageAddressMode::ClampToEdge);
		assert_eq!(sampler.address_mode_w, ImageAddressMode::ClampToEdge);
		assert_eq!(sampler.mag_filter, ImageFilterMode::Linear);
		assert_eq!(sampler.min_filter, ImageFilterMode::Linear);
		assert_eq!(sampler.lod_max_clamp, 0.0);
		Ok(())
	}

	fn flipbook() -> Result<FlipbookManifest, PackagedAssetError> {
		FlipbookManifest::from_json(include_bytes!(
			"../../../../../assets/vfx/dust/rolling_wave.flipbook.json"
		))
	}

	fn volume() -> VolumeManifest {
		VolumeManifest {
			schema_version: 1,
			image: "density.ktx2".into(),
			dimensions: [2, 3, 4],
			bounds: Bounds3 { min: [-0.5; 3], max: [1.5, 2.5, 3.5] },
			format: VolumeFormat::R16Float,
			density_scale: 1.0,
			index_to_local: Mat4::IDENTITY.to_cols_array(),
		}
	}

	fn app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin {
				file_path: format!("{}/../../assets", env!("CARGO_MANIFEST_DIR")),
				meta_check: AssetMetaCheck::Never,
				..default()
			},
			ImagePlugin::default(),
			PackagedVfxPlugin,
		));
		// No GPU required: exercise UASTC -> BC transcoding and standard image loading.
		app.register_asset_loader(ImageLoader::new(CompressedImageFormats::BC));
		app.finish();
		app.cleanup();
		app
	}

	fn wait(app: &mut App, ids: &[UntypedAssetId]) -> TestResult {
		let start = Instant::now();
		loop {
			app.update();
			let server = app.world().resource::<AssetServer>();
			for id in ids {
				if let Some(LoadState::Failed(error)) = server.get_load_state(*id) {
					return Err(format!("asset load failed: {error}").into());
				}
				if let Some(RecursiveDependencyLoadState::Failed(error)) =
					server.get_recursive_dependency_load_state(*id)
				{
					return Err(format!("dependency load failed: {error}").into());
				}
			}
			if ids.iter().all(|id| server.is_loaded_with_dependencies(*id)) {
				return Ok(());
			}
			if start.elapsed() > Duration::from_secs(15) {
				return Err("asset load timed out".into());
			}
			std::thread::sleep(Duration::from_millis(1));
		}
	}
}

#[test]
fn authored_flipbook_has_gutters_views_and_clamped_end() -> TestResult {
	let mut manifest = Fixture::flipbook()?;
	assert_eq!(manifest.duration_seconds(), 3.2);
	assert_eq!(manifest.views.len(), 8);
	assert_eq!(manifest.frame_at(-1.0), 0.0);
	assert_eq!(manifest.frame_at(100.0), 48.0);
	assert_eq!(manifest.frame_at(0.1), 1.5);
	assert_eq!(manifest.nearest_view(Vec3::Z), 0);
	assert_eq!(manifest.nearest_view(Vec3::X), 2);
	let uv = manifest.frame_uv(48);
	assert!((uv.x - 820.0 / 952.0).abs() < 1e-6);
	assert!((uv.z - 128.0 / 952.0).abs() < 1e-6);
	assert_eq!(manifest.frame_uv(u32::MAX), uv);
	assert_eq!(manifest.opacity_at(100.0), 1.0);
	manifest.end_behavior = EndBehavior::Fade { seconds: 0.5 };
	assert_eq!(manifest.opacity_at(0.0), 1.0);
	assert!((manifest.opacity_at(manifest.duration_seconds() - 0.25) - 0.5).abs() < 1e-6);
	assert_eq!(manifest.opacity_at(100.0), 0.0);
	Ok(())
}

#[test]
fn invalid_flipbook_metadata_is_rejected() -> TestResult {
	let source = Fixture::flipbook()?;
	let mutations: &[fn(&mut FlipbookManifest)] = &[
		|m| m.schema_version = 2,
		|m| m.grid[0] = 0,
		|m| m.frame_count = 50,
		|m| m.gutter_px = u32::MAX,
		|m| m.atlas_size[0] += 1,
		|m| m.fps = 0.0,
		|m| m.fps = f32::NAN,
		|m| m.bounds.max[1] = m.bounds.min[1],
		|m| m.views.clear(),
		|m| m.views[0].layer = 1,
		|m| m.views[1].azimuth_degrees = 0.0,
		|m| m.views[0].elevation_degrees = 91.0,
		|m| m.capture_size[0] = 0.0,
		|m| m.image = "../outside.ktx2".into(),
		|m| m.quad_center[0] = f32::INFINITY,
		|m| m.end_behavior = EndBehavior::Fade { seconds: 0.0 },
		|m| m.end_behavior = EndBehavior::Fade { seconds: 99.0 },
	];
	for (index, mutate) in mutations.iter().enumerate() {
		let mut manifest = source.clone();
		mutate(&mut manifest);
		assert!(manifest.validate().is_err(), "invalid mutation {index} accepted");
	}
	let mut json = serde_json::to_value(source)?;
	json["alpha_mode"] = "additive".into();
	assert!(FlipbookManifest::from_json(&serde_json::to_vec(&json)?).is_err());
	Ok(())
}

#[test]
fn volume_preserves_voxel_centers_and_rejects_bad_transforms() -> TestResult {
	let source = Fixture::volume();
	source.validate()?;
	assert_eq!(source.byte_len(), Some(48));
	assert_eq!(
		source.index_to_local_matrix().transform_point3(Vec3::new(1.0, 2.0, 3.0)),
		Vec3::new(1.0, 2.0, 3.0)
	);
	let mutations: &[fn(&mut VolumeManifest)] = &[
		|m| m.dimensions[0] = 0,
		|m| m.dimensions = [u32::MAX; 3],
		|m| m.density_scale = -1.0,
		|m| m.bounds.min[0] = f32::NAN,
		|m| m.index_to_local[0] = 0.0,
		|m| m.index_to_local[3] = 1.0,
		|m| m.index_to_local[12] = 1.0,
	];
	for mutate in mutations {
		let mut manifest = source.clone();
		mutate(&mut manifest);
		assert!(manifest.validate().is_err());
	}
	Ok(())
}

#[test]
fn malformed_or_mismatched_ktx_is_rejected() -> TestResult {
	let manifest = Fixture::flipbook()?;
	let path =
		Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/vfx/dust/rolling_wave.ktx2");
	let mut bytes = std::fs::read(path)?;
	KtxHeader::parse(&bytes)?.validate_flipbook(&manifest)?;
	let mut alpha_mismatch = manifest.clone();
	alpha_mismatch.alpha_mode = AlphaConvention::Premultiplied;
	assert!(KtxHeader::parse(&bytes)?.validate_flipbook(&alpha_mismatch).is_err());
	assert!(KtxHeader::parse(&bytes[..79]).is_err());
	bytes[20..24].copy_from_slice(&1u32.to_le_bytes());
	assert!(KtxHeader::parse(&bytes)?.validate_flipbook(&manifest).is_err());
	bytes.truncate(104);
	assert!(KtxHeader::parse(&bytes).is_err());
	Ok(())
}

#[test]
fn missing_dependency_and_mismatched_image_fail_through_asset_server() -> TestResult {
	use bevy::asset::io::{memory::MemoryAssetReader, AssetSourceBuilder};
	let reader = MemoryAssetReader::default();
	let mut manifest = Fixture::flipbook()?;
	manifest.image = "absent.ktx2".into();
	reader
		.root
		.insert_asset(Path::new("missing.flipbook.json"), serde_json::to_vec(&manifest)?);
	manifest.image = "rolling_wave.ktx2".into();
	manifest.atlas_size = [136, 136];
	manifest.grid = [1, 1];
	manifest.frame_count = 1;
	reader
		.root
		.insert_asset(Path::new("mismatch.flipbook.json"), serde_json::to_vec(&manifest)?);
	reader.root.insert_asset(
		Path::new("rolling_wave.ktx2"),
		std::fs::read(
			Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/vfx/dust/rolling_wave.ktx2"),
		)?,
	);
	let mut app = App::new();
	app.register_asset_source("fixture", AssetSourceBuilder::new(move || Box::new(reader.clone())));
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin { meta_check: AssetMetaCheck::Never, ..default() },
		PackagedVfxPlugin,
	));
	app.init_asset::<Image>();
	app.finish();
	app.cleanup();
	let server = app.world().resource::<AssetServer>();
	let handles: Vec<Handle<PackagedFlipbook>> = ["missing", "mismatch"]
		.iter()
		.map(|name| server.load(format!("fixture://{name}.flipbook.json")))
		.collect();
	let start = Instant::now();
	loop {
		app.update();
		let server = app.world().resource::<AssetServer>();
		if handles
			.iter()
			.all(|h| matches!(server.get_load_state(h.id()), Some(LoadState::Failed(_))))
		{
			break;
		}
		if start.elapsed() > Duration::from_secs(5) {
			return Err("invalid assets did not fail promptly".into());
		}
		std::thread::sleep(Duration::from_millis(1));
	}
	Ok(())
}

#[test]
fn asset_server_loads_and_shares_authored_flipbook_and_volumes() -> TestResult {
	let mut app = Fixture::app();
	let server = app.world().resource::<AssetServer>();
	let flipbook: Handle<PackagedFlipbook> = server.load("vfx/dust/rolling_wave.flipbook.json");
	let again: Handle<PackagedFlipbook> = server.load("vfx/dust/rolling_wave.flipbook.json");
	assert_eq!(flipbook, again);
	let volume_names = [
		"clouds/billow",
		"clouds/curl",
		"clouds/trail",
		"clouds/cluster",
		"bodies/shoulder",
		"bodies/column",
		"bodies/swept",
	];
	let volumes: Vec<Handle<StaticVolume>> = volume_names
		.iter()
		.map(|name| server.load(format!("vfx/dust/{name}.volume.json")))
		.collect();
	let mut ids = vec![flipbook.id().untyped()];
	ids.extend(volumes.iter().map(|h| h.id().untyped()));
	Fixture::wait(&mut app, &ids)?;
	let asset = app
		.world()
		.resource::<Assets<PackagedFlipbook>>()
		.get(&flipbook)
		.ok_or("missing loaded flipbook")?;
	let image = app
		.world()
		.resource::<Assets<Image>>()
		.get(&asset.image)
		.ok_or("missing shared image")?;
	assert_eq!(image.texture_descriptor.size.depth_or_array_layers, 8);
	Fixture::sampler(image)?;
	assert_eq!(image.texture_descriptor.dimension, TextureDimension::D2);
	assert_eq!(
		image.texture_view_descriptor.as_ref().and_then(|d| d.dimension),
		Some(TextureViewDimension::D2Array)
	);
	let direct: Handle<Image> =
		app.world().resource::<AssetServer>().load("vfx/dust/rolling_wave.ktx2");
	assert_eq!(asset.image, direct);
	for handle in volumes {
		let asset = app
			.world()
			.resource::<Assets<StaticVolume>>()
			.get(&handle)
			.ok_or("missing volume")?;
		let image = app
			.world()
			.resource::<Assets<Image>>()
			.get(&asset.image)
			.ok_or("missing volume image")?;
		assert_eq!(image.texture_descriptor.dimension, TextureDimension::D3);
		Fixture::sampler(image)?;
		assert_eq!(image.texture_descriptor.format, TextureFormat::R16Float);
		assert_eq!(image.data.as_ref().map(Vec::len), asset.manifest.byte_len());
	}
	Ok(())
}
