use crate::parts::smoke_explosion::track::*;
use bevy::prelude::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;
pub fn source() -> Result<ExplosionTrack, Box<dyn std::error::Error>> {
	let root =
		std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/vfx/cartoon_explosion");
	Ok(ExplosionTrack::decode(
		serde_json::from_slice(&std::fs::read(root.join("cartoon_explosion.explosion.json"))?)?,
		&std::fs::read(root.join("cartoon_explosion.track.bin"))?,
	)?)
}
#[test]
fn all_authored_reference_states() -> TestResult {
	let asset = source()?;
	let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("../../assets/vfx/cartoon_explosion/reference.json");
	let r: serde_json::Value = serde_json::from_slice(&std::fs::read(root)?)?;
	let frames = r["frames"].as_array().ok_or("missing frames")?;
	assert_eq!(frames.len(), 37);
	for reference in frames {
		let t = reference["time"].as_f64().ok_or("missing time")? as f32;
		let mut out = Frame::default();
		asset.evaluate(t, &mut out);
		let states = reference["states"].as_array().ok_or("missing states")?;
		assert_eq!(out.states.len(), states.len(), "existing set at {t}");
		for (s, r) in out.states.iter().zip(states) {
			assert_eq!(Some(s.id as u64), r["id"].as_u64());
			for (key, values) in [
				("position", s.position.to_array().to_vec()),
				("rotation", s.rotation.to_array().to_vec()),
				("scale", s.scale.to_array().to_vec()),
				("open_dir", s.open_dir.to_array().to_vec()),
				("dissolve", vec![s.dissolve]),
				("tone", vec![s.tone]),
				("openness", vec![s.openness]),
			] {
				let expected = if let Some(a) = r[key].as_array() {
					a.iter().map(|v| v.as_f64().ok_or("number")).collect::<Result<Vec<_>, _>>()?
				} else {
					vec![r[key].as_f64().ok_or("number")?]
				};
				for (a, b) in values.iter().zip(expected) {
					assert!((*a as f64 - b).abs() < 3e-5, "t={t} id={} {key}: {a} vs {b}", s.id);
				}
			}
		}
	}
	Ok(())
}
#[test]
fn truncated_track_rejected_and_zero_scale_not_inverted() -> TestResult {
	let asset = source()?;
	assert!(ExplosionTrack::decode(asset.manifest.clone(), &[0; 32]).is_err());
	let mut f = Frame::default();
	asset.evaluate(0., &mut f);
	assert!(f.states.iter().all(|s| !s.visible()));
	asset.evaluate(14., &mut f);
	assert!(f.states.iter().all(|s| !s.visible()));
	assert_eq!(
		crate::parts::smoke_explosion::sha256::digest(b"abc"),
		"ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
	);
	Ok(())
}

fn asset_app() -> App {
	use bevy::{
		asset::AssetMetaCheck,
		gltf::GltfPlugin,
		image::{CompressedImageFormats, ImageLoader, ImagePlugin},
		mesh::MeshPlugin,
		scene::ScenePlugin,
	};
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		AssetPlugin {
			file_path: format!("{}/../../assets", env!("CARGO_MANIFEST_DIR")),
			meta_check: AssetMetaCheck::Never,
			..default()
		},
		ImagePlugin::default(),
		MeshPlugin,
		ScenePlugin,
		TransformPlugin,
		GltfPlugin::default(),
		bevy::world_serialization::WorldSerializationPlugin,
		crate::parts::packaged::PackagedVfxPlugin,
	));
	app.init_asset::<StandardMaterial>()
		.init_asset::<ExplosionTrack>()
		.init_asset_loader::<TrackLoader>()
		.init_asset::<crate::parts::smoke_explosion::density::CombinedExplosionAsset>()
		.init_asset_loader::<crate::parts::smoke_explosion::density::CombinedLoader>();
	app.register_asset_loader(ImageLoader::new(CompressedImageFormats::BC));
	app.finish();
	app.cleanup();
	app
}
#[test]
fn actual_decoder_and_all_density_probes() -> TestResult {
	use crate::parts::{packaged::StaticVolume, smoke_explosion::density::CombinedExplosionAsset};
	let mut app = asset_app();
	let h: Handle<CombinedExplosionAsset> = app
		.world()
		.resource::<AssetServer>()
		.load("vfx/combined_explosion/combined_explosion.combined.json");
	let begin = std::time::Instant::now();
	loop {
		app.update();
		let server = app.world().resource::<AssetServer>();
		if let Some(bevy::asset::LoadState::Failed(e)) = server.get_load_state(h.id()) {
			return Err(format!("asset failed: {e}").into());
		}
		if let Some(bevy::asset::RecursiveDependencyLoadState::Failed(e)) =
			server.get_recursive_dependency_load_state(h.id())
		{
			return Err(format!("dependency failed: {e}").into());
		}
		if server.is_loaded_with_dependencies(h.id()) {
			break;
		}
		if begin.elapsed().as_secs() > 25 {
			return Err("asset timeout".into());
		}
		std::thread::sleep(std::time::Duration::from_millis(5));
	}
	let asset = app
		.world()
		.resource::<Assets<CombinedExplosionAsset>>()
		.get(&h)
		.ok_or("asset")?;
	let tracks = app.world().resource::<Assets<ExplosionTrack>>();
	let track = tracks.get(&asset.track).ok_or("track")?;
	let volumes = app.world().resource::<Assets<StaticVolume>>();
	asset.validate_fields(track, volumes)?;
	let images = app.world().resource::<Assets<Image>>();
	let mut fields = Vec::new();
	for h in &asset.fields {
		let v = volumes.get(h).ok_or("volume")?;
		let im = images.get(&v.image).ok_or("image")?;
		assert_eq!(
			im.texture_descriptor.format,
			bevy::render::render_resource::TextureFormat::R16Float
		);
		let bytes = im.data.as_ref().ok_or("image data")?;
		let values = bytes
			.chunks_exact(2)
			.map(|b| half(u16::from_le_bytes([b[0], b[1]])))
			.collect::<Vec<_>>();
		assert_eq!(
			values.len(),
			v.manifest.dimensions.iter().map(|x| *x as usize).product::<usize>()
		);
		fields.push(values);
	}
	let rest = |id: usize, p: Vec3| {
		let v = volumes.get(&asset.fields[id]);
		let Some(v) = v else {
			return 0.;
		};
		let m = &v.manifest;
		let lo = Vec3::from_array(m.bounds.min);
		let hi = Vec3::from_array(m.bounds.max);
		if p.cmplt(lo).any() || p.cmpgt(hi).any() {
			return 0.;
		}
		let dims = UVec3::from_array(m.dimensions);
		let q = (p - lo) / (hi - lo) * dims.as_vec3() - Vec3::splat(0.5);
		let a = q.floor().as_ivec3();
		let f = q - q.floor();
		let mut sum = 0.;
		for z in 0..2 {
			for y in 0..2 {
				for x in 0..2 {
					let c = (a + IVec3::new(x, y, z))
						.clamp(IVec3::ZERO, dims.as_ivec3() - IVec3::ONE)
						.as_uvec3();
					let w = Vec3::new(
						if x == 0 { 1. - f.x } else { f.x },
						if y == 0 { 1. - f.y } else { f.y },
						if z == 0 { 1. - f.z } else { f.z },
					);
					sum += fields[id][(c.x + dims.x * (c.y + dims.y * c.z)) as usize]
						* w.element_product();
				}
			}
		}
		sum
	};
	let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("../../assets/vfx/combined_explosion/reference.json");
	let reference: serde_json::Value = serde_json::from_slice(&std::fs::read(root)?)?;
	let probes = reference["probes"].as_array().ok_or("probes")?;
	assert_eq!(probes.len(), 134);
	let mut frame = Frame::default();
	let mut max_error = 0f32;
	for (i, p) in probes.iter().enumerate() {
		let t = p["t"].as_f64().ok_or("time")? as f32;
		let point: [f32; 3] = serde_json::from_value(p["world"].clone())?;
		let point = Vec3::from_array(point);
		track.evaluate(t, &mut frame);
		let primitives = asset.main_primitives(track, &frame, t, volumes);
		let mut main = 0.;
		let mut spikes = 0.;
		for primitive in primitives {
			let d = primitive.density(point, rest);
			main += d;
			if primitive.parameters.w > 1.5 {
				spikes += d;
			}
		}
		let fine = asset.fine_states(track, t).iter().map(|f| f.density(point)).sum::<f32>();
		let cap = |x: f32| 2.2 * (1. - (-x / 2.2).exp());
		for (key, value) in [
			("main_uncapped", main),
			("fine_uncapped", fine),
			("main", cap(main)),
			("fine", fine),
			("composed", cap(main + fine)),
			("spikes", spikes),
		] {
			let expected = p[key].as_f64().ok_or("probe value")? as f32;
			max_error = max_error.max((value - expected).abs());
			assert!(
				(value - expected).abs() < 0.0002 + 0.001 * expected.abs(),
				"probe {i} t={t} {key}: {value} vs {expected}"
			);
		}
	}
	eprintln!("134 probes max absolute error {max_error}");
	Ok(())
}
fn half(h: u16) -> f32 {
	let sign = if h & 0x8000 != 0 { -1. } else { 1. };
	let e = (h >> 10) & 31;
	let m = (h & 1023) as f32;
	if e == 0 {
		sign * m * 2f32.powi(-24)
	} else if e == 31 {
		if m == 0. {
			sign * f32::INFINITY
		} else {
			f32::NAN
		}
	} else {
		sign * (1. + m / 1024.) * 2f32.powi(e as i32 - 15)
	}
}
#[test]
fn placement_heading_and_invalid_transforms() -> TestResult {
	use crate::{parts::smoke_explosion::placement, spawn::VfxSpawn};
	let mut spawn = VfxSpawn {
		seed: Some(13),
		transform: Transform::from_rotation(Quat::from_rotation_y(1.)),
		..default()
	};
	let q = placement(&spawn, Some(Vec3::X))?;
	assert!((spawn.transform.rotation * q * Vec3::X - Vec3::X).length() < 1e-6);
	assert!(placement(&spawn, Some(Vec3::ZERO)).is_err());
	assert!(placement(&spawn, Some(Vec3::NAN)).is_err());
	assert_eq!(placement(&spawn, None)?, placement(&spawn, None)?);
	spawn.transform.rotation = Quat::from_rotation_x(0.1);
	assert!(placement(&spawn, None).is_err());
	Ok(())
}
#[test]
fn grain_fixed_partitions_and_backwards_seek() -> TestResult {
	use crate::parts::smoke_explosion::{density::GrainBirth, grains::GrainSimulation};
	let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
		.join("../../assets/vfx/combined_explosion/combined_explosion.grains.json");
	let v: serde_json::Value = serde_json::from_slice(&std::fs::read(root)?)?;
	let births: Vec<GrainBirth> = serde_json::from_value(v["births"].clone())?;
	let mut a = GrainSimulation::default();
	let mut b = GrainSimulation::default();
	a.advance(2.75, &births);
	for i in 1..=165 {
		b.advance(i as f32 / 60., &births);
	}
	assert_eq!(a.steps, b.steps);
	for (x, y) in a.states.iter().zip(&b.states) {
		assert_eq!(x.position, y.position);
		assert_eq!(x.velocity, y.velocity);
		assert!(x.position.is_finite());
	}
	b.advance(0.8, &births);
	let mut c = GrainSimulation::default();
	c.advance(0.8, &births);
	for (x, y) in b.states.iter().zip(&c.states) {
		assert_eq!(x.position, y.position);
	}
	b.advance(3.95, &births);
	assert!(b.states.iter().all(|s| s.radius == 0.));
	Ok(())
}
#[test]
fn conservative_rotated_bounds_contain_all_corners() -> TestResult {
	use crate::parts::smoke_explosion::density::Primitive;
	for i in 0..100 {
		let model = Mat4::from_scale_rotation_translation(
			Vec3::new(0.2, 3., 1.7),
			Quat::from_euler(EulerRot::XYZ, i as f32 * 0.2, 0.7, -0.2),
			Vec3::new(2., -1., 3.),
		);
		let lo = Vec3::new(-0.4, -0.5, -0.3);
		let hi = Vec3::new(0.6, 0.7, 0.5);
		let (a, b) = Primitive::bounds(model, lo, hi);
		for k in 0..8 {
			let p = model.transform_point3(Vec3::new(
				if k & 1 == 0 { lo.x } else { hi.x },
				if k & 2 == 0 { lo.y } else { hi.y },
				if k & 4 == 0 { lo.z } else { hi.z },
			));
			assert!(p.cmpge(a - Vec3::splat(1e-6)).all() && p.cmple(b + Vec3::splat(1e-6)).all());
		}
	}
	Ok(())
}
#[test]
fn pooled_ownership_pause_concurrency_and_cleanup() -> TestResult {
	use crate::{
		composition::{CartoonExplosionPart, ExplosionLod},
		parts::smoke_explosion::{
			cartoon::{CartoonMaterial, Pool},
			ExplosionDiagnostics, ExplosionLayer, ExplosionLimits, OwnedSlots,
		},
		spawn::{VfxInstance, VfxSpawn},
	};
	use bevy::{asset::RenderAssetUsages, render::storage::ShaderBuffer};
	let mut app = asset_app();
	let track: Handle<ExplosionTrack> = app
		.world()
		.resource::<AssetServer>()
		.load("vfx/cartoon_explosion/cartoon_explosion.explosion.json");
	let start = std::time::Instant::now();
	while !app.world().resource::<AssetServer>().is_loaded_with_dependencies(track.id()) {
		app.update();
		if start.elapsed().as_secs() > 20 {
			return Err("track dependency timeout".into());
		}
		std::thread::sleep(std::time::Duration::from_millis(5));
	}
	app.init_asset::<CartoonMaterial>()
		.init_asset::<ShaderBuffer>()
		.init_resource::<OwnedSlots>()
		.init_resource::<crate::parts::smoke_explosion::FrameCache>()
		.init_resource::<ExplosionLimits>()
		.init_resource::<ExplosionDiagnostics>();
	let values = vec![Vec4::ZERO; (8 * 349 + 3 * 96) * 8];
	let buffer = app
		.world_mut()
		.resource_mut::<Assets<ShaderBuffer>>()
		.add(ShaderBuffer::new(values.clone(), RenderAssetUsages::all()));
	let material = app
		.world_mut()
		.resource_mut::<Assets<CartoonMaterial>>()
		.add(CartoonMaterial { data: buffer.clone() });
	app.insert_resource(Pool { material, buffer, values })
		.add_systems(Update, crate::parts::smoke_explosion::tick);
	let mut roots = Vec::new();
	for _ in 0..2 {
		let root = app
			.world_mut()
			.spawn((
				Transform::IDENTITY,
				VfxInstance {
					name: "cartoon-explosion".into(),
					age: 0.,
					duration: 14.,
					playback: 1.,
					seed: 6,
					armed: true,
				},
			))
			.id();
		roots.push(root);
		let part = CartoonExplosionPart {
			track: track.clone(),
			heading: Some(Vec3::X),
			lod: ExplosionLod::Full,
		};
		part.spawn(
			&mut app.world_mut().commands(),
			root,
			Transform::IDENTITY,
			&VfxSpawn::default(),
		);
	}
	app.world_mut().flush();
	app.update();
	for e in &roots {
		app.world_mut().get_mut::<VfxInstance>(*e).ok_or("root")?.age = 3.;
	}
	app.update();
	assert_eq!(app.world().resource::<OwnedSlots>().0.len(), 2);
	let snapshot = app.world().resource::<Pool>().values.clone();
	app.update();
	assert_eq!(snapshot, app.world().resource::<Pool>().values);
	assert_eq!(app.world_mut().query::<&ExplosionLayer>().iter(app.world()).count(), 2);
	for e in roots {
		app.world_mut().despawn(e);
	}
	app.update();
	assert!(app.world().resource::<OwnedSlots>().0.is_empty());
	assert_eq!(app.world_mut().query::<&ExplosionLayer>().iter(app.world()).count(), 0);
	assert_eq!(
		app.world_mut()
			.query::<&crate::parts::smoke_explosion::ElementSlot>()
			.iter(app.world())
			.count(),
		0
	);
	Ok(())
}

#[test]
fn every_sparse_interval_has_exact_inclusive_pads() -> TestResult {
	let track = source()?;
	for e in &track.manifest.elements {
		let start = e.first_sample as f32 / 30.;
		let end = (e.first_sample + e.sample_count - 1) as f32 / 30.;
		let first = track.sample(e.id, start).ok_or("missing first endpoint")?;
		let last = track.sample(e.id, end).ok_or("missing final endpoint")?;
		let a = track.samples[e.byte_offset / 64];
		let b = track.samples[e.byte_offset / 64 + e.sample_count - 1];
		assert_eq!(first.scale.to_array(), [a[7], a[8], a[9]]);
		assert_eq!(last.scale.to_array(), [b[7], b[8], b[9]]);
		if start > 0. {
			assert!(track.sample(e.id, f32::from_bits(start.to_bits() - 1)).is_none());
		}
		if end < 14. {
			assert!(track.sample(e.id, f32::from_bits(end.to_bits() + 1)).is_none());
		}
	}
	Ok(())
}
