use crate::{
	composition::*,
	lifecycle::{gate_vfx_instances, tick_vfx_instances},
	parts::{
		cartoon_wave::{
			motion::{Deformation, Wave},
			tests::rest,
			Geometry,
		},
		combined::{
			assets::*,
			prepare,
			render::{CombinedMaterial, CombinedRenderer, Payload, Primitive, MAX_PAYLOAD_BYTES},
			simulation::*,
			tick,
		},
		packaged::{PackagedVfxPlugin, StaticVolume},
	},
	spawn::{spawn_vfx, VfxInstance, VfxSpawn},
};
use bevy::{
	asset::{AssetMetaCheck, AssetPlugin, LoadState, RecursiveDependencyLoadState},
	gltf::GltfPlugin,
	image::{CompressedImageFormats, ImageLoader, ImagePlugin},
	math::DVec3,
	mesh::MeshPlugin,
	prelude::*,
	render::{render_resource::TextureFormat, storage::ShaderBuffer},
	scene::ScenePlugin,
	time::TimeUpdateStrategy,
	transform::{TransformPlugin, TransformSystems},
};
use serde_json::Value;
use std::time::{Duration, Instant};
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
fn manifest() -> TestResult<Manifest> {
	Ok(serde_json::from_str(include_str!("../../../../../assets/vfx/dust/combined.combined.json"))?)
}
fn config() -> TestResult<Config> {
	let doc: Value = serde_json::from_str(include_str!(
		"../../../../../assets/vfx/dust/clouds/dynamic_rules.json"
	))?;
	Ok(serde_json::from_value(doc["config"].clone())?)
}
fn sim(scenario: CombinedScenario) -> TestResult<Simulation> {
	Ok(Simulation::new(Wave::new(rest()?), config()?, manifest()?.anchors, scenario))
}
fn v3(v: &Value) -> TestResult<DVec3> {
	Ok(DVec3::new(v[0].as_f64().ok_or("x")?, v[1].as_f64().ok_or("y")?, v[2].as_f64().ok_or("z")?))
}
fn near(a: DVec3, b: DVec3, t: f64) {
	assert!((a - b).abs().max_element() < t, "{a:?} != {b:?} (tol {t})");
}
#[test]
fn independent_hash_birth_and120hz_cloud_grain_snapshots() -> TestResult {
	let reference: Value = serde_json::from_str(include_str!("reference.json"))?;
	for p in reference["delivery"]["hash_probes"].as_array().ok_or("hashes")? {
		let at = |k: &str| -> TestResult<u32> { Ok(p[k].as_u64().ok_or("hash parameter")? as u32) };
		assert_eq!(
			random(at("seed")?, at("region")?, at("tick")?, at("channel")?),
			p["value"].as_f64().ok_or("value")?
		);
	}
	for scenario in reference["scenarios"].as_array().ok_or("scenarios")? {
		let mut max_position = 0f64;
		let mut max_velocity = 0f64;
		let mut max_birth_velocity = 0f64;
		let mode = if scenario["name"] == "normal" {
			CombinedScenario::Normal
		} else {
			CombinedScenario::WindTurn
		};
		let mut s = sim(mode)?;
		for sample in scenario["samples"].as_array().ok_or("samples")? {
			let target = sample["time"].as_f64().ok_or("time")?;
			while s.time() + 1e-8 < target {
				s.advance_to(target);
			}
			let expected = sample["particles"].as_array().ok_or("particles")?;
			let live = s.particles.iter().filter(|p| p.age < p.life).collect::<Vec<_>>();
			assert_eq!(live.len(), expected.len(), "{mode:?},t={target}");
			for (p, e) in live.iter().zip(expected) {
				assert_eq!(p.id, e["id"].as_u64().ok_or("id")? as u32);
				assert_eq!(p.kind, e["kind"].as_u64().ok_or("kind")? as u32);
				assert_eq!(p.variant, e["variant"].as_u64().ok_or("variant")? as usize);
				max_position = max_position.max((p.position - v3(&e["P"])?).abs().max_element());
				max_velocity = max_velocity.max((p.velocity - v3(&e["v"])?).abs().max_element());
				near(p.position, v3(&e["P"])?, 0.0003);
				near(p.velocity, v3(&e["v"])?, 0.0005);
				near(p.scale, v3(&e["scale"])?, 1e-6);
				near(p.color, v3(&e["color"])?, 1e-10);
				assert!((p.age - e["age"].as_f64().ok_or("age")?).abs() < 1e-9);
				assert!(
					(p.concentration - e["concentration"].as_f64().ok_or("concentration")?).abs()
						< 1e-9
				);
				let q = e["q"].as_array().ok_or("q")?;
				for (x, y) in p.rotation.to_array().iter().zip(q) {
					assert!((*x - y.as_f64().ok_or("q component")?).abs() < 1e-8);
				}
			}
			assert_eq!(s.dropped[0], sample["dropped"][0].as_u64().ok_or("drop0")? as u32);
			assert_eq!(s.dropped[1], sample["dropped"][1].as_u64().ok_or("drop1")? as u32);
		}
		let births = scenario["births"].as_array().ok_or("births")?;
		assert_eq!(births.len(), s.births.len());
		for (b, e) in s.births.iter().zip(births) {
			assert_eq!(b.id, e["id"].as_u64().ok_or("birth id")? as u32);
			near(b.rest, v3(&e["rest"])?, 2e-6);
			near(b.position, v3(&e["position"])?, 0.0001);
			max_birth_velocity = max_birth_velocity
				.max((b.inherited - v3(&e["inherited_velocity"])?).abs().max_element());
			near(b.inherited, v3(&e["inherited_velocity"])?, 0.0008);
			near(b.release, v3(&e["release_velocity"])?, 0.0002);
		}
		println!("{mode:?}: max snapshot position {max_position:.9}m, velocity {max_velocity:.9}m/s, birth material velocity {max_birth_velocity:.9}m/s");
	}
	Ok(())
}
#[test]
fn fixed_tick_backlog_partitions_pool_limits_and_world_release() -> TestResult {
	let mut a = sim(CombinedScenario::Normal)?;
	let mut b = sim(CombinedScenario::Normal)?;
	assert!(a.advance_to(2.) > 1.7);
	assert_eq!(a.tick, 32);
	while a.advance_to(2.) > 0. {}
	for i in 0..240 {
		b.advance_to((i + 1) as f64 / 120.);
	}
	assert_eq!(a.tick, 240);
	assert_eq!(b.tick, 240);
	assert_eq!(a.particles.len(), b.particles.len());
	for (x, y) in a.particles.iter().zip(&b.particles) {
		assert_eq!(x.id, y.id);
		assert_eq!(x.position, y.position);
		assert_eq!(x.velocity, y.velocity);
	}
	let before = a.particles.iter().map(|p| (p.id, p.position)).collect::<Vec<_>>();
	let phase = a.pose(2., 0).ok_or("body")?.phase;
	a.scenario = CombinedScenario::WindTurn;
	assert_eq!(phase, a.pose(2., 0).ok_or("turned body")?.phase);
	for ((_, p), now) in before.iter().zip(&a.particles) {
		assert_eq!(*p, now.position);
	}
	a.placement = Mat4::from_translation(Vec3::new(5., 0., 0.)).as_dmat4();
	a.step();
	for (id, p) in before {
		if let Some(now) = a.particles.iter().find(|p| p.id == id) {
			assert!((now.position - p).length() < 0.04, "released particle teleported with body");
		}
	}
	let mut tiny = sim(CombinedScenario::Normal)?;
	tiny.config.max_clouds = 2;
	tiny.config.max_grains = 3;
	for _ in 0..564 {
		tiny.step();
		assert!(tiny.particles.iter().filter(|p| p.kind == 0).count() <= 2);
		assert!(tiny.particles.iter().filter(|p| p.kind == 1).count() <= 3);
	}
	assert!(tiny.dropped[0] > 0 && tiny.dropped[1] > 0);
	assert!(tiny.particles.iter().all(|p| p.age >= p.life));
	Ok(())
}
#[test]
fn body_inverse_bounds_and_cartoon_pose_agreement() -> TestResult {
	let s = sim(CombinedScenario::Normal)?;
	for t in [0.5, 1.37, 2.47, 3.57, 4.] {
		for id in 0..15 {
			let Some(p) = s.pose(t, id) else {
				continue;
			};
			let expected =
				if id < 12 { s.body.parent(t as f32, id) } else { s.body.child(t as f32, id - 12) }
					.ok_or("cartoon")?;
			assert_eq!(p.transform(), expected.transform());
			let support = &manifest()?.rest_support[p.variant];
			let (lo, hi) = p.bounds(Vec3::from_array(support.min), Vec3::from_array(support.max));
			for rest in
				[Vec3::new(0.2, 0.7, -0.3), Vec3::new(-0.8, 0.4, 0.5), Vec3::new(0.1, 1.2, 0.2)]
			{
				let w = p.world(rest);
				near(p.rest(w).as_dvec3(), rest.as_dvec3(), 2e-6);
				assert!(w.cmpge(lo).all() && w.cmple(hi).all());
			}
		}
	}
	let p = Vec3::new(0.2, 0.7, -0.3);
	let c = Deformation::at(1.37, 2);
	near(c.inverse(c.map(p)).as_dvec3(), p.as_dvec3(), 1e-6);
	Ok(())
}

fn asset_app() -> App {
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
		PackagedVfxPlugin,
	));
	app.init_asset::<StandardMaterial>()
		.init_asset::<CombinedAsset>()
		.init_asset_loader::<CombinedLoader>();
	app.register_asset_loader(ImageLoader::new(CompressedImageFormats::BC));
	app.finish();
	app.cleanup();
	app
}
fn load(app: &mut App) -> TestResult<Handle<CombinedAsset>> {
	let h = app
		.world()
		.resource::<AssetServer>()
		.load::<CombinedAsset>("vfx/dust/combined.combined.json");
	let begin = Instant::now();
	loop {
		app.update();
		let server = app.world().resource::<AssetServer>();
		if let Some(LoadState::Failed(e)) = server.get_load_state(h.id()) {
			return Err(format!("Combined load failed:{e}").into());
		}
		if let Some(RecursiveDependencyLoadState::Failed(e)) =
			server.get_recursive_dependency_load_state(h.id())
		{
			return Err(format!("Combined dependency failed:{e}").into());
		}
		if server.is_loaded_with_dependencies(h.id()) {
			return Ok(h);
		}
		if begin.elapsed() > Duration::from_secs(15) {
			return Err("Combined asset timeout".into());
		}
		std::thread::sleep(Duration::from_millis(1));
	}
}
fn half(bits: u16) -> f32 {
	let sign = if bits & 0x8000 == 0 { 1. } else { -1. };
	let e = (bits >> 10) & 31;
	let m = (bits & 1023) as f32;
	if e == 0 {
		sign * m * 2f32.powi(-24)
	} else if e == 31 {
		f32::NAN
	} else {
		sign * (1. + m / 1024.) * 2f32.powi(e as i32 - 15)
	}
}
fn sample(image: &Image, volume: &StaticVolume, local: Vec3) -> TestResult<f32> {
	let data = image.data.as_ref().ok_or("CPU image")?;
	let dims = UVec3::from_array(volume.manifest.dimensions);
	let q = volume.manifest.index_to_local_matrix().inverse().transform_point3(local);
	let i = q.floor().as_ivec3();
	let f = q - q.floor();
	let mut out = 0.;
	for z in 0..2 {
		for y in 0..2 {
			for x in 0..2 {
				let at = i + IVec3::new(x, y, z);
				if at.cmpge(IVec3::ZERO).all() && at.cmplt(dims.as_ivec3()).all() {
					let index =
						(at.x as u32 + dims.x * (at.y as u32 + dims.y * at.z as u32)) as usize * 2;
					let bits =
						u16::from_le_bytes(data.get(index..index + 2).ok_or("voxel")?.try_into()?);
					let w = Vec3::new(
						if x == 0 { 1. - f.x } else { f.x },
						if y == 0 { 1. - f.y } else { f.y },
						if z == 0 { 1. - f.z } else { f.z },
					);
					out += half(bits) * w.x * w.y * w.z;
				}
			}
		}
	}
	Ok(out)
}
#[test]
fn typed_dependencies_decode_spatial_density_and_share_handles() -> TestResult {
	let mut app = asset_app();
	let h = load(&mut app)?;
	let server = app.world().resource::<AssetServer>();
	assert_eq!(h, server.load("vfx/dust/combined.combined.json"));
	let a = app
		.world()
		.resource::<Assets<CombinedAsset>>()
		.get(&h)
		.ok_or("Combined asset")?;
	let cartoon = crate::composition::CartoonWavePart::load(server);
	assert_eq!(cartoon.meshes, a.meshes);
	assert_eq!(a.config.max_clouds, 96);
	assert_eq!(a.config.max_grains, 96);
	let volumes = app.world().resource::<Assets<StaticVolume>>();
	let images = app.world().resource::<Assets<Image>>();
	let probes: Value = serde_json::from_str(include_str!("density_probes.json"))?;
	for (id, handle) in a.bodies.iter().chain(a.clouds.iter()).enumerate() {
		let v = volumes.get(handle).ok_or("volume")?;
		let image = images.get(&v.image).ok_or("image")?;
		assert_eq!(image.texture_descriptor.format, TextureFormat::R16Float);
		for probe in probes[id].as_array().ok_or("density probes")? {
			let p = v3(&probe["position"])?.as_vec3();
			let expected = probe["value"].as_f64().ok_or("rho")? as f32;
			assert!((sample(image, v, p)? - expected).abs() < 2e-5);
		}
		assert_eq!(sample(image, v, Vec3::splat(100.))?, 0.);
	}
	Ok(())
}

fn host() -> TestResult<(App, EffectDefinition)> {
	let mut app = asset_app();
	let handle = load(&mut app)?;
	app.init_resource::<Geometry>()
		.init_resource::<CombinedRenderer>()
		.init_resource::<crate::parts::combined::cache::CacheInput>()
		.init_asset::<CombinedMaterial>()
		.init_asset::<ShaderBuffer>()
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(0.02)));
	app.add_systems(Update, (tick_vfx_instances, tick).chain()).add_systems(
		PostUpdate,
		(
			prepare.after(TransformSystems::Propagate),
			gate_vfx_instances.after(prepare),
			crate::parts::combined::render::compose.after(prepare),
		),
	);
	let recipe = crate::effects::combined::CombinedDust { assets: handle, options: default() };
	Ok((app, recipe.effect_definition()))
}
fn spawn(app: &mut App, def: &EffectDefinition, seed: u64, speed: f32) -> Entity {
	let root = spawn_vfx(
		&mut app.world_mut().commands(),
		def,
		VfxSpawn { seed: Some(seed), playback: speed, ..default() },
	);
	app.world_mut().flush();
	root
}
#[test]
fn shared_host_pause_speed_reset_cancel_completion_and_resource_reuse() -> TestResult {
	let (mut app, def) = host()?;
	let initial_images = app.world().resource::<Assets<Image>>().len();
	let initial_meshes = app.world().resource::<Assets<Mesh>>().len();
	let a = spawn(&mut app, &def, 9271, 1.);
	let b = spawn(&mut app, &def, 999, 0.5);
	for _ in 0..60 {
		app.update();
	}
	let sa = app.world().get::<crate::parts::combined::CombinedState>(a).ok_or("a")?;
	let tick_a = sa.simulation.tick;
	let count_a = sa.simulation.particles.len();
	assert!(count_a > 0);
	let sb = app.world().get::<crate::parts::combined::CombinedState>(b).ok_or("b")?;
	assert!((tick_a as i32 - 2 * sb.simulation.tick as i32).abs() <= 1);
	assert_eq!(app.world().resource::<Assets<CombinedMaterial>>().len(), 1);
	assert_eq!(app.world().resource::<Assets<ShaderBuffer>>().len(), 1);
	assert_eq!(app.world().resource::<Assets<Image>>().len(), initial_images + 1);
	for mut state in app
		.world_mut()
		.query::<&mut crate::parts::combined::CombinedState>()
		.iter_mut(app.world_mut())
	{
		let mut options = state.options;
		options.layers = CombinedLayers::Clouds;
		options.quality = CombinedQuality::Low;
		state.configure(options);
	}
	app.world_mut().resource_mut::<Time<Virtual>>().pause();
	for _ in 0..10 {
		app.update();
	}
	assert_eq!(
		app.world()
			.get::<crate::parts::combined::CombinedState>(a)
			.ok_or("paused a")?
			.simulation
			.tick,
		tick_a
	);
	app.world_mut().despawn(a);
	app.update();
	assert!(app.world().get::<crate::parts::combined::CombinedState>(b).is_some());
	app.world_mut().resource_mut::<Time<Virtual>>().unpause();
	let reset = spawn(&mut app, &def, 9271, 1.);
	for _ in 0..60 {
		app.update();
	}
	let sr = app.world().get::<crate::parts::combined::CombinedState>(reset).ok_or("reset")?;
	assert_eq!(sr.simulation.tick, tick_a);
	assert_eq!(sr.simulation.particles.len(), count_a);
	for _ in 0..470 {
		app.update();
	}
	assert_eq!(app.world_mut().query::<&VfxInstance>().iter(app.world()).count(), 0);
	assert_eq!(app.world().resource::<Assets<CombinedMaterial>>().len(), 0);
	assert_eq!(app.world().resource::<Assets<ShaderBuffer>>().len(), 0);
	assert_eq!(app.world().resource::<Assets<Image>>().len(), initial_images);
	assert_eq!(app.world().resource::<Assets<Mesh>>().len(), initial_meshes);
	assert_eq!(app.world().resource::<CombinedRenderer>().payload.references, 0);
	assert_eq!(app.world().resource::<CombinedRenderer>().payload.max_bin, 0);
	Ok(())
}
#[test]
fn bounded_events_and_spatial_candidates_never_silently_truncate() -> TestResult {
	let (mut app, def) = host()?;
	for seed in 0..7 {
		spawn(&mut app, &def, seed, 1.);
	}
	for _ in 0..5 {
		app.update();
	}
	assert_eq!(app.world_mut().query::<&VfxInstance>().iter(app.world()).count(), 4);
	assert_eq!(
		app.world_mut()
			.query::<&crate::parts::combined::CombinedLayerState>()
			.iter(app.world())
			.count(),
		12
	);
	let reference = app
		.world()
		.resource::<Assets<CombinedAsset>>()
		.iter()
		.next()
		.map(|(_, a)| a)
		.ok_or("asset")?;
	let volumes = app.world().resource::<Assets<StaticVolume>>();
	let v = reference
		.bodies
		.iter()
		.chain(reference.clouds.iter())
		.map(|h| volumes.get(h))
		.collect::<Option<Vec<_>>>()
		.ok_or("volumes")?;
	let v: [&StaticVolume; 7] = v.try_into().map_err(|_| "volume count")?;
	let same = Primitive {
		masked: false,
		inverse: Mat4::IDENTITY,
		coefficients: Vec4::ZERO,
		parameters: Vec4::ZERO,
		min: Vec3::splat(-1000.),
		max: Vec3::splat(1000.),
	};
	let mut payload = Payload::default();
	payload.build(&vec![same; 828], v, &reference.style, CombinedQuality::Default);
	assert_eq!(payload.max_bin, 828);
	assert_eq!(payload.references, 828 * 2048);
	assert!(payload.values.len() as u64 * 16 <= MAX_PAYLOAD_BYTES);
	Ok(())
}

#[test]
fn overflow_requests_preserve_existing_owners() -> TestResult {
	let (mut app, def) = host()?;
	let owners = (0..4).map(|seed| spawn(&mut app, &def, seed, 1.)).collect::<Vec<_>>();
	for _ in 0..5 {
		app.update();
	}
	app.world_mut().despawn(owners[0]);
	app.update();
	let replacement = spawn(&mut app, &def, 100, 1.);
	for _ in 0..5 {
		app.update();
	}
	let overflow = spawn(&mut app, &def, 101, 1.);
	for _ in 0..5 {
		app.update();
	}
	for owner in owners.into_iter().skip(1).chain([replacement]) {
		assert!(app.world().get::<crate::parts::combined::CombinedState>(owner).is_some());
	}
	assert!(app.world().get::<VfxInstance>(overflow).is_none());
	let mut config = app
		.world()
		.get::<crate::parts::combined::CombinedState>(replacement)
		.ok_or("replacement")?
		.simulation
		.config;
	config.dt = f64::NAN;
	assert!(config.validate().is_err());
	Ok(())
}

fn interactive_sim(continuous: bool) -> TestResult<Simulation> {
	let mut s = sim(CombinedScenario::Normal)?;
	s.interactive = Some(crate::parts::combined::interactive::Interactive::new(continuous));
	Ok(s)
}
fn advance_interaction(
	s: &mut Simulation,
	t: f64,
	scene: &crate::parts::combined::interaction::InteractionScene,
) {
	while s.time() + 1e-9 < t {
		s.advance_interactive(t, scene);
	}
}
#[test]
fn interaction_replay_partitions_backlog_and_no_snapback() -> TestResult {
	use crate::parts::combined::interaction::{InteractionScene, Scenario};
	let scene = InteractionScene { scenario: Scenario::Fast, ..default() };
	let mut a = interactive_sim(false)?;
	let mut b = interactive_sim(false)?;
	assert!(a.advance_interactive(3.6, &scene) > 3.);
	advance_interaction(&mut a, 3.6, &scene);
	for i in 1..=216 {
		b.advance_interactive(i as f64 / 60., &scene);
	}
	assert_eq!(a.tick, b.tick);
	for (x, y) in a
		.interactive
		.as_ref()
		.ok_or("a")?
		.puffs
		.iter()
		.flatten()
		.zip(b.interactive.as_ref().ok_or("b")?.puffs.iter().flatten())
	{
		near(x.position, y.position, 1e-12);
		near(x.velocity, y.velocity, 1e-12);
	}
	for (x, y) in a.particles.iter().zip(&b.particles) {
		assert_eq!(x.id, y.id);
		near(x.position, y.position, 1e-12);
	}
	let mut control = InteractionScene { scenario: Scenario::None, ..default() };
	let body = a.interactive.as_mut().ok_or("state")?.puffs[0].as_mut().ok_or("puff")?;
	let phase = body.phase;
	body.position.z += 10.;
	body.pose.position.z += 10.;
	body.previous.position.z += 10.;
	control.wind = DVec3::X * 0.3;
	advance_interaction(&mut a, 5., &control);
	let body = a.interactive.as_ref().ok_or("state")?.puffs[0].as_ref().ok_or("puff")?;
	assert!(body.position.z > 8.);
	assert!(body.phase >= phase);
	Ok(())
}
#[test]
fn displaced_body_material_points_drive_new_fine_emission() -> TestResult {
	use crate::parts::combined::interaction::{InteractionScene, Scenario};
	let scene = InteractionScene { scenario: Scenario::None, ..default() };
	let mut s = interactive_sim(false)?;
	advance_interaction(&mut s, 1., &scene);
	for p in s.interactive.as_mut().ok_or("state")?.puffs.iter_mut().flatten() {
		p.position.z += 3.;
		p.pose.position.z += 3.;
		p.previous.position.z += 3.;
	}
	let mut found = false;
	for _ in 0..120 {
		let birth_tick = s.tick;
		let t = s.time();
		s.advance_interactive(t + 1. / 120., &scene);
		if let Some(p) = s.particles.iter().find(|p| p.birth >= t - 1e-10 && p.position.z > 2.) {
			let puff = s
				.interactive
				.as_ref()
				.ok_or("state")?
				.puffs
				.iter()
				.flatten()
				.find(|b| b.id as usize == p.parent)
				.ok_or("source")?;
			let a = s.anchors[puff.pose.variant][(puff.id as usize + birth_tick as usize) % 3];
			let rest = Vec3::from_array(a.point) + Vec3::from_array(a.normal) * 0.12;
			let (point, velocity) = puff.material_point(rest);
			assert!((p.position - point).length() < 0.03);
			assert!((p.velocity - velocity).length() < 0.5);
			found = true;
			break;
		}
	}
	assert!(found, "expected a new birth on the displaced carrier");
	Ok(())
}
#[test]
fn interaction_wake_expiry_pools_and_finite_drain() -> TestResult {
	use crate::parts::combined::interaction::{InteractionScene, Scenario, MAX_WAKES};
	let scene = InteractionScene { scenario: Scenario::Stop, ..default() };
	let mut s = interactive_sim(false)?;
	advance_interaction(&mut s, 4., &scene);
	assert!(!s.interactive.as_ref().ok_or("state")?.wakes.is_empty());
	advance_interaction(&mut s, 5.2, &scene);
	assert!(s.interactive.as_ref().ok_or("state")?.wakes.is_empty());
	advance_interaction(&mut s, 14., &scene);
	assert!(s.interactive.as_ref().ok_or("state")?.puffs.iter().all(Option::is_none));
	assert!(s.particles.is_empty());
	let mut s = interactive_sim(true)?;
	s.config.max_clouds = 2;
	s.config.max_grains = 2;
	for i in 1..=400 {
		s.advance_interactive(i as f64 / 60., &scene);
		assert!(s.interactive.as_ref().ok_or("state")?.wakes.len() <= MAX_WAKES);
		assert!(s.interactive.as_ref().ok_or("state")?.puffs.iter().flatten().count() <= 15);
		assert!(s.particles.iter().filter(|p| p.kind == 0).count() <= 2);
		assert!(s.particles.iter().filter(|p| p.kind == 1).count() <= 2);
	}
	assert!(s.dropped[0] > 0 && s.dropped[1] > 0);
	Ok(())
}
#[test]
fn interactive_host_pause_occupancy_invalidation_admission_and_cleanup() -> TestResult {
	use crate::parts::combined::{cache::CacheInput, interaction::InteractionScene, CombinedState};
	let (mut app, mut def) = host()?;
	app.init_resource::<crate::parts::combined::interaction::InteractionScene>();
	for layer in &mut def.layers {
		if let EffectPart::Combined(p) = &mut layer.part {
			p.options.motion = CombinedMotion::InteractiveBrief;
		}
	}
	let a = spawn(&mut app, &def, 9271, 1.);
	for _ in 0..80 {
		app.update();
	}
	assert!(app
		.world()
		.get::<CombinedState>(a)
		.ok_or("owner")?
		.simulation
		.interactive
		.is_some());
	let overflow = spawn(&mut app, &def, 22, 1.);
	for _ in 0..5 {
		app.update();
	}
	assert!(app.world().get::<VfxInstance>(overflow).is_none());
	assert!(app.world().get::<CombinedState>(a).is_some());
	app.world_mut().resource_mut::<Time<Virtual>>().pause();
	app.update();
	let tick = app.world().get::<CombinedState>(a).ok_or("owner")?.simulation.tick;
	let generation = app.world().resource::<CacheInput>().0.as_ref().ok_or("cache")?.generation;
	let image_count = app.world().resource::<Assets<Image>>().len();
	let t = tick as f64 / 120.;
	app.world_mut()
		.resource_mut::<InteractionScene>()
		.place(t, DVec3::new(0., 0.7, 0.), true);
	for _ in 0..5 {
		app.update();
	}
	assert_eq!(app.world().get::<CombinedState>(a).ok_or("owner")?.simulation.tick, tick);
	assert!(
		app.world().resource::<CacheInput>().0.as_ref().ok_or("cache")?.generation > generation
	);
	assert_eq!(app.world().resource::<Assets<Image>>().len(), image_count);
	app.world_mut().resource_mut::<Time<Virtual>>().unpause();
	for _ in 0..220 {
		app.update();
	}
	assert!(
		app.world()
			.get::<VfxInstance>(a)
			.ok_or("interactive instance must outlive reference cutoff")?
			.age > 4.7
	);
	assert!(app
		.world()
		.get::<CombinedState>(a)
		.ok_or("owner")?
		.simulation
		.interactive
		.as_ref()
		.ok_or("state")?
		.puffs
		.iter()
		.any(Option::is_some));
	app.world_mut().despawn(a);
	for _ in 0..5 {
		app.update();
	}
	assert!(app.world().resource::<CacheInput>().0.is_none());
	assert_eq!(app.world().resource::<Assets<CombinedMaterial>>().len(), 0);
	assert_eq!(app.world().resource::<Assets<ShaderBuffer>>().len(), 0);
	Ok(())
}

#[test]
fn interaction_changes_large_motion_without_using_the_density_mask() -> TestResult {
	use crate::parts::combined::interaction::InteractionScene;
	let scene = InteractionScene::default();
	let control = InteractionScene { enabled: false, ..default() };
	let mut a = interactive_sim(false)?;
	let mut b = interactive_sim(false)?;
	advance_interaction(&mut a, 4.5, &scene);
	advance_interaction(&mut b, 4.5, &control);
	let mut displacement = 0.0f64;
	for (p, q) in a
		.interactive
		.as_ref()
		.ok_or("state")?
		.puffs
		.iter()
		.flatten()
		.zip(b.interactive.as_ref().ok_or("control")?.puffs.iter().flatten())
	{
		displacement = displacement.max((p.position - q.position).length());
	}
	assert!(
		displacement > 0.3,
		"physical carrier motion must differ, independent of rendering/masking: {displacement}"
	);
	eprintln!("Large-carrier interaction/control maximum displacement at4.5s: {displacement}m");
	Ok(())
}

#[test]
fn stopping_interactive_emission_is_instance_owned_and_drains() -> TestResult {
	use crate::parts::combined::interaction::{InteractionScene, Scenario};
	let scene = InteractionScene { scenario: Scenario::None, ..default() };
	let mut stopped = interactive_sim(true)?;
	let mut running = interactive_sim(true)?;
	advance_interaction(&mut stopped, 2., &scene);
	advance_interaction(&mut running, 2., &scene);
	let serial = stopped.interactive.as_ref().ok_or("state")?.serial;
	stopped.interactive.as_mut().ok_or("state")?.stop_at = Some(2.);
	advance_interaction(&mut stopped, 4., &scene);
	advance_interaction(&mut running, 4., &scene);
	assert_eq!(stopped.interactive.as_ref().ok_or("state")?.serial, serial);
	assert!(stopped.particles.iter().all(|p| p.birth <= 2. + 1e-9));
	assert!(running.interactive.as_ref().ok_or("independent state")?.serial > serial);
	advance_interaction(&mut stopped, 11., &scene);
	assert!(stopped.particles.is_empty());
	assert!(stopped.interactive.as_ref().ok_or("state")?.puffs.iter().all(Option::is_none));
	let mut reset = interactive_sim(true)?;
	advance_interaction(&mut reset, 4., &scene);
	assert_eq!(reset.tick, running.tick);
	assert_eq!(
		reset.interactive.as_ref().ok_or("reset state")?.serial,
		running.interactive.as_ref().ok_or("running state")?.serial
	);
	for (p, q) in reset
		.interactive
		.as_ref()
		.ok_or("reset")?
		.puffs
		.iter()
		.flatten()
		.zip(running.interactive.as_ref().ok_or("running")?.puffs.iter().flatten())
	{
		assert_eq!(p.position, q.position);
		assert_eq!(p.velocity, q.velocity);
	}
	Ok(())
}

#[test]
fn flow_guided_shear_changes_filled_map_and_emission_velocity() -> TestResult {
	use crate::parts::combined::{
		flow::{Flow, H, N},
		interaction::{InteractionScene, Scenario},
	};
	let mut s = interactive_sim(false)?;
	let mut field = Flow::default();
	for z in 0..N[2] {
		for y in 0..N[1] {
			for x in 0..=N[0] {
				field.velocity[0][x + (N[0] + 1) * (y + N[1] * z)] = (y as f64 + 0.5) * H;
			}
		}
	}
	s.interactive.as_mut().ok_or("state")?.flow = Some(field);
	let scene = InteractionScene { scenario: Scenario::None, ..default() };
	advance_interaction(&mut s, 0.1, &scene);
	let p = s
		.interactive
		.as_ref()
		.ok_or("state")?
		.puffs
		.iter()
		.flatten()
		.next()
		.ok_or("carrier")?;
	let a = p.affine;
	assert!(a.y_axis.x.abs() > 1e-5, "differential shear must change the mass map");
	assert!(a.determinant() > 0.);
	for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
		assert!((a * axis).length() >= 0.7 && (a * axis).length() <= 1.3);
	}
	let rest = Vec3::new(0.4, 0.7, -0.2);
	let model = p.model();
	let world = model.transform_point3(p.pose.coefficients.map(rest));
	let recovered = p.pose.coefficients.inverse(model.inverse().transform_point3(world));
	assert!((rest - recovered).length() < 1e-5);
	let (position, velocity) = p.material_point(rest);
	assert!((position - world.as_dvec3()).length() < 1e-8);
	assert!(velocity.is_finite());
	Ok(())
}

#[test]
fn flow_history_is_owned_deterministic_and_paused_with_host() -> TestResult {
	use crate::parts::combined::{
		flow::Flow,
		interaction::{InteractionScene, Scenario},
		CombinedState,
	};
	let scene = InteractionScene { scenario: Scenario::None, ..default() };
	let mut a = interactive_sim(false)?;
	let mut b = interactive_sim(false)?;
	a.interactive.as_mut().ok_or("state")?.flow = Some(Flow::new(scene.wind));
	b.interactive.as_mut().ok_or("state")?.flow = Some(Flow::new(scene.wind));
	advance_interaction(&mut a, 0.2, &scene);
	for k in 1..=24 {
		advance_interaction(&mut b, k as f64 / 120., &scene);
	}
	assert_eq!(
		a.interactive.as_ref().ok_or("state")?.flow.as_ref().ok_or("flow")?.velocity,
		b.interactive.as_ref().ok_or("state")?.flow.as_ref().ok_or("flow")?.velocity
	);
	let (mut app, mut def) = host()?;
	app.init_resource::<crate::parts::combined::interaction::InteractionScene>();
	for layer in &mut def.layers {
		if let EffectPart::Combined(p) = &mut layer.part {
			p.options.motion = CombinedMotion::FlowGuided;
		}
	}
	let root = spawn(&mut app, &def, 9271, 1.);
	for _ in 0..20 {
		app.update();
	}
	app.world_mut().resource_mut::<Time<Virtual>>().pause();
	app.update();
	let steps = app
		.world()
		.get::<CombinedState>(root)
		.ok_or("owner")?
		.simulation
		.interactive
		.as_ref()
		.ok_or("state")?
		.flow
		.as_ref()
		.ok_or("flow")?
		.steps;
	for _ in 0..4 {
		app.update();
	}
	assert_eq!(
		steps,
		app.world()
			.get::<CombinedState>(root)
			.ok_or("owner")?
			.simulation
			.interactive
			.as_ref()
			.ok_or("state")?
			.flow
			.as_ref()
			.ok_or("flow")?
			.steps
	);
	app.world_mut().despawn(root);
	for _ in 0..5 {
		app.update();
	}
	assert_eq!(app.world_mut().query::<&CombinedState>().iter(app.world()).count(), 0);
	Ok(())
}

#[test]
fn material_probe_transport_and_sampling_cost() -> TestResult {
	use crate::parts::combined::{
		flow::Flow,
		interaction::{InteractionScene, Scenario},
	};
	let scene = InteractionScene { scenario: Scenario::None, ..default() };
	let mut s = interactive_sim(false)?;
	advance_interaction(&mut s, 1.5, &scene);
	let field = Flow::new(DVec3::X);
	let puffs = &s.interactive.as_ref().ok_or("state")?.puffs;
	let start = std::time::Instant::now();
	let mut result = DVec3::ZERO;
	let mut count = 0;
	for _ in 0..1000 {
		for p in puffs.iter().flatten() {
			for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
				for sign in [-1., 1.] {
					let rest = Vec3::new(0.05, 0.66, 0.) + axis * sign * 0.45;
					let world = std::hint::black_box(
						p.model().transform_point3(p.pose.coefficients.map(rest)),
					);
					result += field.sample(world.as_dvec3());
					count += 1;
				}
			}
		}
	}
	assert!((result / count as f64 - DVec3::X).length() < 1e-9);
	eprintln!("complete material mapping plus {} trilinear probes: {} ms/repetition,1000 repetitions (CPU microbenchmark)",count/1000,start.elapsed().as_secs_f64());
	Ok(())
}

#[test]
fn clump_carry_motion_comparison_and_field_independence() -> TestResult {
	use crate::parts::combined::{
		flow::Flow,
		interaction::{InteractionScene, Scenario},
	};
	let mut states = Vec::new();
	for (carry, rate) in [(1., 4.), (0.15, 4.), (1., 0.)] {
		let scene = InteractionScene { clump_carry: carry, ..default() };
		let mut s = interactive_sim(false)?;
		let mut f = Flow::new(scene.wind);
		f.momentum_rate = rate;
		s.interactive.as_mut().ok_or("state")?.flow = Some(f);
		for tick in 1..=720 {
			advance_interaction(&mut s, tick as f64 / 120., &scene);
			if tick % 12 == 0 {
				for p in s.interactive.as_ref().ok_or("state")?.puffs.iter().flatten() {
					eprintln!(
						"CARRY,{carry},{rate},{},{},{},{},{},{},{}",
						s.time(),
						p.id,
						p.position.x,
						p.position.y,
						p.position.z,
						p.velocity.z,
						{
							let f = s
								.interactive
								.as_ref()
								.ok_or("state")?
								.flow
								.as_ref()
								.ok_or("field")?;
							let mut sum = 0.;
							for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
								for sign in [-1., 1.] {
									let rest = Vec3::new(0.05, 0.66, 0.) + axis * sign * 0.45;
									sum += f
										.sample(
											p.model()
												.transform_point3(p.pose.coefficients.map(rest))
												.as_dvec3(),
										)
										.z;
								}
							}
							sum / 6.
						}
					);
				}
			}
		}
		states.push(s);
	}
	for kind in 0..2 {
		let parts = states[1].particles.iter().filter(|p| p.kind == kind).collect::<Vec<_>>();
		assert!(parts.iter().any(|p| p.velocity.z > 0.02), "secondary following remains");
		eprintln!(
			"SECONDARY kind={kind},count={},mean_vz={}",
			parts.len(),
			parts.iter().map(|p| p.velocity.z).sum::<f64>() / parts.len() as f64
		);
	}
	let baseline = states[0].interactive.as_ref().ok_or("baseline")?;
	let tuned = states[1].interactive.as_ref().ok_or("tuned")?;
	assert_eq!(
		baseline.flow.as_ref().ok_or("flow")?.velocity,
		tuned.flow.as_ref().ok_or("flow")?.velocity
	);
	let a = baseline.puffs[0].as_ref().ok_or("puff")?;
	let b = tuned.puffs[0].as_ref().ok_or("puff")?;
	assert!(
		b.position.z < a.position.z * 0.7,
		"representative carrier must carry substantially less: {} vs {}",
		b.position.z,
		a.position.z
	);
	assert!(b.affine.determinant() > 0.);
	// No moving collider: carry tuning must not change normal transport at all.
	let mut a = interactive_sim(false)?;
	let mut b = interactive_sim(false)?;
	a.interactive.as_mut().ok_or("state")?.flow = Some(Flow::new(DVec3::X));
	b.interactive.as_mut().ok_or("state")?.flow = Some(Flow::new(DVec3::X));
	let control = InteractionScene { scenario: Scenario::None, clump_carry: 1., ..default() };
	let tuned = InteractionScene { clump_carry: 0.15, ..control.clone() };
	advance_interaction(&mut a, 1., &control);
	advance_interaction(&mut b, 1., &tuned);
	for (p, q) in a
		.interactive
		.as_ref()
		.ok_or("state")?
		.puffs
		.iter()
		.flatten()
		.zip(b.interactive.as_ref().ok_or("state")?.puffs.iter().flatten())
	{
		assert_eq!(p.position, q.position);
		assert_eq!(p.affine, q.affine);
	}
	Ok(())
}

#[test]
fn choreography_typed_host_no_opaque_pause_limits_and_cleanup() -> TestResult {
	use crate::{
		effects::combined_choreography::CombinedLanding,
		layers::combined_choreography::CombinedChoreographyLayer,
		parts::{
			cartoon_flow::{CartoonFlow, CartoonFlowPlugin, Surface},
			cartoon_landing::LandingMaterial,
			combined_choreography::OpticalChoreography,
		},
		spawn::Effect,
	};
	let (mut app, _) = host()?;
	app.init_resource::<crate::parts::combined::interaction::InteractionScene>()
		.init_asset::<LandingMaterial>();
	CartoonFlowPlugin.build(&mut app);
	let handle: Handle<CombinedAsset> =
		app.world().resource::<AssetServer>().load("vfx/dust/combined.combined.json");
	let meshes = app
		.world()
		.resource::<Assets<CombinedAsset>>()
		.get(&handle)
		.ok_or("asset")?
		.meshes
		.clone();
	let recipe = CombinedLanding {
		layer: CombinedChoreographyLayer {
			part: CombinedChoreographyPart {
				assets: handle,
				meshes,
				landing: true,
				contact: false,
				options: default(),
			},
		},
	};
	let initial_meshes = app.world().resource::<Assets<Mesh>>().len();
	let initial_images = app.world().resource::<Assets<Image>>().len();
	app.world_mut().commands().spawn_scene(Effect {
		recipe: recipe.clone(),
		spawn: VfxSpawn { seed: Some(13), playback: 2., ..default() },
	});
	app.world_mut().flush();
	for _ in 0..15 {
		app.update();
	}
	assert_eq!(app.world_mut().query::<&OpticalChoreography>().iter(app.world()).count(), 1);
	assert_eq!(app.world_mut().query::<&Surface>().iter(app.world()).count(), 0);
	assert!(app.world_mut().query::<&CartoonFlow>().single(app.world())?.tick > 40);
	let tick = app.world_mut().query::<&CartoonFlow>().single(app.world())?.tick;
	let age = app.world_mut().query::<&VfxInstance>().single(app.world())?.age;
	assert_eq!(tick, (age as f64 * 120. + 1e-6).floor() as u32);
	let generation = app
		.world()
		.resource::<crate::parts::combined::cache::CacheInput>()
		.0
		.as_ref()
		.ok_or("cache")?
		.generation;
	app.world_mut().resource_mut::<Time<Virtual>>().pause();
	for _ in 0..4 {
		app.update();
	}
	assert_eq!(app.world_mut().query::<&CartoonFlow>().single(app.world())?.tick, tick);
	assert_eq!(
		app.world()
			.resource::<crate::parts::combined::cache::CacheInput>()
			.0
			.as_ref()
			.ok_or("cache")?
			.generation,
		generation
	);
	app.world_mut().resource_mut::<Time<Virtual>>().unpause();
	for _ in 0..85 {
		app.update();
	}
	assert!(app.world_mut().query::<&VfxInstance>().iter(app.world()).next().is_none());
	for _ in 0..6 {
		app.update();
	}
	assert_eq!(app.world().resource::<Assets<Mesh>>().len(), initial_meshes);
	assert_eq!(app.world().resource::<Assets<Image>>().len(), initial_images);
	for seed in 0..5 {
		app.world_mut().commands().spawn_scene(Effect {
			recipe: recipe.clone(),
			spawn: VfxSpawn { seed: Some(seed), ..default() },
		});
	}
	app.world_mut().flush();
	for _ in 0..5 {
		app.update();
	}
	assert_eq!(app.world_mut().query::<&VfxInstance>().iter(app.world()).count(), 4);
	let roots = app
		.world_mut()
		.query_filtered::<Entity, With<VfxInstance>>()
		.iter(app.world())
		.collect::<Vec<_>>();
	for root in roots {
		app.world_mut().despawn(root);
	}
	for _ in 0..6 {
		app.update();
	}
	assert_eq!(app.world_mut().query::<&OpticalChoreography>().iter(app.world()).count(), 0);
	assert_eq!(app.world().resource::<CombinedRenderer>().primitives, 0);
	Ok(())
}

#[test]
fn choreography_wave_terminal_clock_releases_owner_and_optical_assets() -> TestResult {
	use crate::{
		effects::combined_choreography::CombinedWaveContact,
		layers::combined_choreography::CombinedChoreographyLayer,
		parts::{
			cartoon_flow::{CartoonFlow, CartoonFlowPlugin},
			cartoon_landing::LandingMaterial,
			combined_choreography::OpticalChoreography,
		},
	};
	let (mut app, _) = host()?;
	app.init_resource::<crate::parts::combined::interaction::InteractionScene>()
		.init_asset::<LandingMaterial>();
	CartoonFlowPlugin.build(&mut app);
	let assets: Handle<CombinedAsset> =
		app.world().resource::<AssetServer>().load("vfx/dust/combined.combined.json");
	let meshes = app
		.world()
		.resource::<Assets<CombinedAsset>>()
		.get(&assets)
		.ok_or("asset")?
		.meshes
		.clone();
	let recipe = CombinedWaveContact {
		layer: CombinedChoreographyLayer {
			part: CombinedChoreographyPart {
				assets,
				meshes,
				landing: false,
				contact: true,
				options: default(),
			},
		},
	};
	let root = spawn(&mut app, &recipe.effect_definition(), 13, 1.);
	for _ in 0..4 {
		app.update();
	}
	{
		let mut q = app.world_mut().query::<&mut CartoonFlow>();
		let mut c = q.single_mut(app.world_mut())?;
		assert!(c.ready);
		c.tick = 682;
		c.bodies.fill(None);
	}
	{
		let mut q = app.world_mut().query::<&mut OpticalChoreography>();
		q.single_mut(app.world_mut())?.particles.clear();
	}
	app.world_mut().get_mut::<VfxInstance>(root).ok_or("root")?.age = 5.7;
	for _ in 0..8 {
		app.update();
	}
	assert!(app.world().get::<VfxInstance>(root).is_none());
	assert_eq!(app.world_mut().query::<&CartoonFlow>().iter(app.world()).count(), 0);
	assert_eq!(app.world_mut().query::<&OpticalChoreography>().iter(app.world()).count(), 0);
	assert_eq!(app.world().resource::<CombinedRenderer>().primitives, 0);
	Ok(())
}

#[test]
fn fixed_regions_isolate_optics_inputs_admission_and_cleanup() -> TestResult {
	use crate::parts::{
		cartoon_flow::{CartoonFlow, CartoonFlowPlugin},
		cartoon_landing::LandingMaterial,
		combined::cache::CacheInput,
	};
	use crate::{
		CartoonContact, CombinedWaveContact, ContactInputs, ContactRegionBudget, Effect,
		WorldContactSnapshot,
	};
	let (mut app, _) = host()?;
	app.init_resource::<crate::parts::combined::interaction::InteractionScene>()
		.init_asset::<LandingMaterial>();
	CartoonFlowPlugin.build(&mut app);
	app.world_mut().resource_mut::<ContactRegionBudget>().maximum = 2;
	let recipe = CombinedWaveContact::load(app.world().resource::<AssetServer>());
	let initial_images = app.world().resource::<Assets<Image>>().len();
	let initial_materials = app
		.world()
		.resource::<Assets<crate::parts::combined::render::CombinedMaterial>>()
		.len();
	let initial_buffers =
		app.world().resource::<Assets<bevy::render::storage::ShaderBuffer>>().len();
	let initial_meshes = app.world().resource::<Assets<Mesh>>().len();
	let mut roots = Vec::new();
	for p in [Vec3::new(20., 0., -15.), Vec3::new(-12., 8., 18.)] {
		let root = app
			.world_mut()
			.commands()
			.spawn_scene(Effect {
				recipe: recipe.clone(),
				spawn: VfxSpawn {
					transform: Transform::from_translation(p)
						.with_rotation(Quat::from_rotation_y(0.64577)),
					seed: Some(13),
					..default()
				},
			})
			.id();
		let mut input = ContactInputs::default();
		input.submit(WorldContactSnapshot {
			age: 0.,
			colliders: vec![],
			wind: bevy::math::DVec3::ZERO,
			discontinuous: true,
		})?;
		app.world_mut().commands().entity(root).insert(input);
		roots.push(root);
	}
	app.world_mut().flush();
	for _ in 0..8 {
		app.update();
	}
	assert_eq!(
		app.world_mut()
			.query::<&CartoonFlow>()
			.iter(app.world())
			.filter(|s| s.ready && s.region.is_some())
			.count(),
		2
	);
	assert_eq!(app.world().resource::<CacheInput>().1.len(), 2);
	assert_ne!(
		app.world().resource::<CacheInput>().1[0].field,
		app.world().resource::<CacheInput>().1[1].field
	);
	assert!(app.world().resource::<CacheInput>().0.is_none());
	let generations = app
		.world()
		.resource::<CacheInput>()
		.1
		.iter()
		.map(|i| i.generation)
		.collect::<Vec<_>>();
	app.world_mut().resource_mut::<Time<Virtual>>().pause();
	for _ in 0..3 {
		app.update();
	}
	assert_eq!(
		generations,
		app.world()
			.resource::<CacheInput>()
			.1
			.iter()
			.map(|i| i.generation)
			.collect::<Vec<_>>()
	);
	// Explicit cap rejection does not evict either owner.
	let overflow = app
		.world_mut()
		.commands()
		.spawn_scene(Effect { recipe: recipe.clone(), spawn: default() })
		.id();
	app.world_mut().flush();
	for _ in 0..4 {
		app.update();
	}
	assert!(app.world().get_entity(overflow).is_err());
	assert!(app.world().get_entity(roots[0]).is_ok());
	app.world_mut().despawn(roots[0]);
	app.world_mut().resource_mut::<Time<Virtual>>().unpause();
	for _ in 0..5 {
		app.update();
	}
	assert_eq!(app.world().resource::<CacheInput>().1.len(), 1);
	assert!(app.world().get::<VfxInstance>(roots[1]).is_some_and(|i| i.age > 0.1));
	// Root motion is explicitly unsupported, including a change before transform propagation.
	app.world_mut()
		.get_mut::<Transform>(roots[1])
		.ok_or("root transform")?
		.translation
		.x += 1.;
	for _ in 0..8 {
		app.update();
	}
	assert!(app.world().get_entity(roots[1]).is_err());
	assert!(app.world().resource::<CacheInput>().1.is_empty());
	assert_eq!(app.world().resource::<Assets<Image>>().len(), initial_images);
	assert_eq!(app.world().resource::<Assets<Mesh>>().len(), initial_meshes);
	assert_eq!(
		app.world()
			.resource::<Assets<crate::parts::combined::render::CombinedMaterial>>()
			.len(),
		initial_materials
	);
	assert_eq!(
		app.world().resource::<Assets<bevy::render::storage::ShaderBuffer>>().len(),
		initial_buffers
	);
	// Transform.scale must not be silently discarded by the generic host.
	let cartoon = CartoonContact::wave(app.world().resource::<AssetServer>());
	let invalid = app
		.world_mut()
		.commands()
		.spawn_scene(Effect {
			recipe: cartoon,
			spawn: VfxSpawn {
				transform: Transform::from_scale(Vec3::new(1., 2., 1.)),
				..default()
			},
		})
		.id();
	app.world_mut().flush();
	for _ in 0..5 {
		app.update();
	}
	assert!(app.world().get_entity(invalid).is_err());
	Ok(())
}
