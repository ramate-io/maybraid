use crate::{
	composition::{CartoonWavePart, EffectDefinition, EffectLayer, EffectPart},
	lifecycle::{gate_vfx_instances, tick_vfx_instances},
	parts::{
		cartoon_landing::LandingMaterial,
		cartoon_wave::{tick, Body, Geometry},
	},
	spawn::{spawn_vfx, VfxInstance, VfxSpawn},
};
use bevy::{asset::AssetPlugin, prelude::*, time::TimeUpdateStrategy};
use std::{sync::Arc, time::Duration};
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(crate) fn rest() -> TestResult<[Arc<[Vec3]>; 3]> {
	let mut result = Vec::new();
	for name in ["Shoulder", "Column", "Swept"] {
		let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
			.join(format!("../../assets/vfx/dust/bodies/{name}.glb"));
		let bytes = std::fs::read(path)?;
		let u32_at = |i| -> TestResult<u32> {
			Ok(u32::from_le_bytes(bytes.get(i..i + 4).ok_or("GLB range")?.try_into()?))
		};
		let json_len = u32_at(12)? as usize;
		let doc: serde_json::Value =
			serde_json::from_slice(bytes.get(20..20 + json_len).ok_or("json range")?)?;
		let ai = doc["meshes"][0]["primitives"][0]["attributes"]["POSITION"]
			.as_u64()
			.ok_or("position accessor")? as usize;
		let a = &doc["accessors"][ai];
		let v = &doc["bufferViews"][a["bufferView"].as_u64().ok_or("view")? as usize];
		let count = a["count"].as_u64().ok_or("count")? as usize;
		let offset = 28
			+ json_len
			+ v["byteOffset"].as_u64().unwrap_or(0) as usize
			+ a["byteOffset"].as_u64().unwrap_or(0) as usize;
		let stride = v["byteStride"].as_u64().unwrap_or(12) as usize;
		let mut points = Vec::new();
		for i in 0..count {
			let mut xyz = [0.; 3];
			for (j, x) in xyz.iter_mut().enumerate() {
				*x = f32::from_bits(u32_at(offset + i * stride + j * 4)?);
			}
			points.push(Vec3::from_array(xyz));
		}
		result.push(Arc::from(points));
	}
	result.try_into().map_err(|_| "mesh array".into())
}
fn fixture(step: f64) -> TestResult<(App, EffectDefinition)> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, AssetPlugin::default()))
		.init_asset::<Mesh>()
		.init_asset::<LandingMaterial>()
		.init_resource::<Geometry>()
		.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(step)))
		.add_systems(Update, (tick_vfx_instances, tick).chain())
		.add_systems(PostUpdate, gate_vfx_instances);
	let points = rest()?;
	let handles = points.map(|p| {
		let mut mesh = Mesh::new(
			bevy::mesh::PrimitiveTopology::TriangleList,
			bevy::asset::RenderAssetUsages::all(),
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			p.iter().map(|p| p.to_array()).collect::<Vec<_>>(),
		);
		app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh)
	});
	let part = CartoonWavePart { meshes: handles };
	Ok((
		app,
		EffectDefinition::new(
			"cartoon-wave",
			[EffectLayer {
				part: EffectPart::CartoonWave(part),
				delay: 0.,
				transform: Transform::IDENTITY,
				scale: 1.,
			}],
		),
	))
}
fn spawn(app: &mut App, def: &EffectDefinition, speed: f32) -> Entity {
	let root = spawn_vfx(
		&mut app.world_mut().commands(),
		def,
		VfxSpawn { seed: Some(13), playback: speed, ..default() },
	);
	app.world_mut().flush();
	root
}
#[test]
fn readiness_pause_speed_reset_independence_cancel_cleanup() -> TestResult {
	let (mut app, def) = fixture(0.01)?;
	let a = spawn(&mut app, &def, 1.);
	let b = spawn(&mut app, &def, 2.);
	assert!(!app.world().get::<VfxInstance>(a).ok_or("a")?.armed);
	for _ in 0..25 {
		app.update();
	}
	let age = app.world().get::<VfxInstance>(a).ok_or("a")?.age;
	assert!((app.world().get::<VfxInstance>(b).ok_or("b")?.age - 2. * age).abs() < 0.00001);
	assert_eq!(app.world_mut().query::<&Body>().iter(app.world()).count(), 30);
	assert_eq!(app.world().resource::<Geometry>().slots.len(), 3);
	let handles = app
		.world_mut()
		.query::<&Body>()
		.iter(app.world())
		.map(|p| p.material.id())
		.collect::<std::collections::HashSet<_>>();
	assert_eq!(handles.len(), 30);
	app.world_mut().resource_mut::<Time<Virtual>>().pause();
	let poses = app
		.world_mut()
		.query::<(&Body, &Transform)>()
		.iter(app.world())
		.map(|(_, t)| *t)
		.collect::<Vec<_>>();
	for _ in 0..10 {
		app.update();
	}
	assert_eq!(age, app.world().get::<VfxInstance>(a).ok_or("paused a")?.age);
	assert_eq!(
		poses,
		app.world_mut()
			.query::<(&Body, &Transform)>()
			.iter(app.world())
			.map(|(_, t)| *t)
			.collect::<Vec<_>>()
	);
	app.world_mut().despawn(a);
	app.update();
	assert_eq!(app.world_mut().query::<&Body>().iter(app.world()).count(), 15);
	app.world_mut().resource_mut::<Time<Virtual>>().unpause();
	let reset = spawn(&mut app, &def, 1.);
	for _ in 0..25 {
		app.update();
	}
	assert!((age - app.world().get::<VfxInstance>(reset).ok_or("reset")?.age).abs() < 0.011);
	for _ in 0..410 {
		app.update();
	}
	assert_eq!(app.world_mut().query::<&Body>().iter(app.world()).count(), 0);
	assert_eq!(app.world_mut().query::<&VfxInstance>().iter(app.world()).count(), 0);
	for _ in 0..5 {
		app.update();
	}
	assert_eq!(app.world().resource::<Assets<LandingMaterial>>().len(), 0);
	assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 3);
	Ok(())
}
#[test]
fn bounded_admission_and_landing_coexistence() -> TestResult {
	let (mut app, def) = fixture(0.01)?;
	app.add_systems(Update, crate::parts::cartoon_landing::tick.after(tick_vfx_instances));
	let EffectPart::CartoonWave(part) = &def.layers[0].part else {
		return Err("wave part".into());
	};
	let landing = EffectDefinition::new(
		"cartoon-landing",
		[EffectLayer {
			part: EffectPart::CartoonLanding(crate::composition::CartoonLandingPart {
				meshes: part.meshes.clone(),
			}),
			delay: 0.,
			transform: Transform::IDENTITY,
			scale: 1.,
		}],
	);
	let land = spawn(&mut app, &landing, 1.);
	for _ in 0..20 {
		spawn(&mut app, &def, 1.);
	}
	for _ in 0..3 {
		app.update();
	}
	assert_eq!(app.world_mut().query::<&Body>().iter(app.world()).count(), 240);
	assert_eq!(app.world_mut().query::<&VfxInstance>().iter(app.world()).count(), 17);
	assert!(app.world().get::<VfxInstance>(land).ok_or("landing preserved")?.armed);
	assert_eq!(app.world().resource::<Assets<Mesh>>().len(), 3);
	Ok(())
}

#[test]
fn waits_for_all_meshes_and_reuses_asset_server_handles() -> TestResult {
	let (mut app, def) = fixture(0.01)?;
	let EffectPart::CartoonWave(part) = &def.layers[0].part else {
		return Err("wave part".into());
	};
	let handle = part.meshes[1].clone();
	let mesh = app
		.world_mut()
		.resource_mut::<Assets<Mesh>>()
		.remove(handle.id())
		.ok_or("mesh")?;
	let root = spawn(&mut app, &def, 1.);
	for _ in 0..30 {
		app.update();
	}
	let instance = app.world().get::<VfxInstance>(root).ok_or("waiting root")?;
	assert!(!instance.armed);
	assert_eq!(instance.age, 0.);
	assert_eq!(app.world_mut().query::<&Body>().iter(app.world()).count(), 0);
	app.world_mut().resource_mut::<Assets<Mesh>>().insert(handle.id(), mesh)?;
	app.update();
	app.update();
	assert!(app.world().get::<VfxInstance>(root).ok_or("ready root")?.armed);
	assert!(app.world().get::<VfxInstance>(root).ok_or("ready age")?.age <= 0.011);
	assert_eq!(app.world_mut().query::<&Body>().iter(app.world()).count(), 15);
	let server = app.world().resource::<AssetServer>();
	let wave = CartoonWavePart::load(server);
	let landing = crate::composition::CartoonLandingPart::load(server);
	assert_eq!(wave.meshes, landing.meshes);
	app.world_mut().despawn(root);
	for _ in 0..6 {
		app.update();
	}
	assert_eq!(app.world().resource::<Assets<LandingMaterial>>().len(), 0);
	Ok(())
}
