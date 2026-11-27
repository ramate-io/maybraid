use crate::{
	effects::cartoon_dust::CartoonDust,
	parts::mesh_clip::{ClipMaterial, MeshClipManifest},
	EffectPart, EffectRecipe,
};
use bevy::prelude::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn manifest() -> Result<MeshClipManifest, Box<dyn std::error::Error>> {
	Ok(MeshClipManifest::from_json(include_bytes!(
		"../../../../../assets/vfx/dust/cartoon_landing.mesh_clip.json"
	))?)
}
#[test]
fn matches_authored_landing_state_probes_and_opaque_contract() -> TestResult {
	let m = manifest()?;
	let cases: serde_json::Value = serde_json::from_str(include_str!("reference.json"))?;
	for case in cases.as_array().ok_or("cases")? {
		let time = case["time"].as_f64().ok_or("time")? as f32;
		for s in case["states"].as_array().ok_or("states")? {
			let seed = s["seed"].as_u64().ok_or("seed")? as u32;
			let (index, t) =
				m.tracks.iter().enumerate().find(|(_, t)| t.seed == seed).ok_or("track")?;
			let p = m.sample(index, time).ok_or("live pose")?;
			for k in 0..3 {
				assert!(
					(p.translation[k] as f64 - s["position"][k].as_f64().ok_or("position")?).abs()
						< 0.0002
				);
				assert!(
					(p.scale[k] as f64 - s["scale"][k].as_f64().ok_or("scale")?).abs() < 0.0002
				);
			}
			assert!(
				(p.cutoff as f64 - s["dissolve_threshold"].as_f64().ok_or("cutoff")?).abs() < 0.001
			);
			let yaw = s["yaw"].as_f64().ok_or("yaw")? as f32;
			assert!(
				Quat::from_array(p.rotation).angle_between(Quat::from_rotation_y(-yaw)) < 0.001
			);
			assert_eq!(
				Material::alpha_mode(&ClipMaterial::new(t.seed, &m.surface)),
				AlphaMode::Mask(0.5)
			);
		}
	}
	let def = CartoonDust.effect_definition();
	assert_eq!(def.name, "cartoon-dust");
	assert_eq!(def.layers.len(), 1);
	let EffectPart::MeshClip(part) = &def.layers[0].part else {
		return Err("mesh clip missing".into());
	};
	assert_eq!(part.path, CartoonDust::CLIP);
	Ok(())
}
#[test]
fn finite_positive_poses_and_independent_stateless_clocks() -> TestResult {
	let m = manifest()?;
	let mut elapsed = 0.;
	for _ in 0..60 {
		elapsed += 0.005;
	}
	for i in 0..m.tracks.len() {
		let a = m.sample(i, 0.3);
		let b = m.sample(i, elapsed);
		assert_eq!(a.is_some(), b.is_some());
		if let (Some(a), Some(b)) = (a, b) {
			assert!((a.transform().translation - b.transform().translation).length() < 1e-5);
		}
		for j in 0..1200 {
			if let Some(p) = m.sample(i, j as f32 / 1000.) {
				assert!(p.transform().to_matrix().is_finite());
				assert!(p.transform().scale.min_element() > 0.);
			}
		}
		assert!(m.sample(i, 1.2).is_none());
		assert!(m.sample(i, m.tracks[i].birth - 0.001).is_none());
	}
	let a = m.sample(2, 0.3);
	let _other = m.sample(2, 0.6);
	assert_eq!(a.map(|p| p.translation), m.sample(2, 0.3).map(|p| p.translation));
	Ok(())
}
#[test]
fn rejects_invalid_paths_timing_and_pose_data() -> TestResult {
	let m = manifest()?;
	assert!(m.sample(0, f32::NAN).is_none());
	let mut overflow = m.clone();
	overflow.duration = f32::MAX;
	overflow.fps = f32::MAX;
	assert!(overflow.validate().is_err());
	let mut bad = m.clone();
	bad.tracks[0].mesh = "../escape.glb#Mesh0/Primitive0".into();
	assert!(bad.validate().is_err());
	let mut bad = m.clone();
	bad.tracks[0].frames.pop();
	assert!(bad.validate().is_err());
	let mut bad = m.clone();
	bad.fps = f32::INFINITY;
	assert!(bad.validate().is_err());
	let mut bad = m;
	let p = bad
		.tracks
		.iter_mut()
		.flat_map(|t| t.frames.iter_mut())
		.find_map(|p| p.as_mut())
		.ok_or("pose")?;
	p.scale[0] = 0.;
	assert!(bad.validate().is_err());
	Ok(())
}

#[test]
fn instance_palette_gain_defaults_tint_and_clamped_intensity() -> TestResult {
	use crate::{parts::mesh_clip::ClipAppearance, VfxSpawn};
	let m = manifest()?;
	for spawn in [
		VfxSpawn::default(),
		VfxSpawn { tint: Some(Color::srgba(0.5, 0.25, 0.75, 0.0)), ..default() },
		VfxSpawn { intensity: 0.01, ..default() },
		VfxSpawn { intensity: 2., ..default() },
		VfxSpawn { intensity: 100., ..default() },
	] {
		let appearance = ClipAppearance::from_spawn(&spawn);
		let material = ClipMaterial::with_appearance(13, &m.surface, appearance);
		let tint = spawn.tint_or_white().to_linear();
		for (authored, actual) in
			m.surface.colors.iter().zip([material.dark, material.mid, material.bright])
		{
			let expected = Vec3::from_array(*authored)
				* Vec3::new(tint.red, tint.green, tint.blue)
				* spawn.clamped_intensity();
			assert!((actual.truncate() - expected).abs().max_element() < 1e-6);
			assert_eq!(actual.w, 1.);
		}
		assert_eq!(material.thresholds, Vec4::new(0.04, 0.62, 0., 0.));
		assert_eq!(material.seed.x, 13);
		assert_eq!(material.cutoff.x, -0.02);
	}
	let baseline = ClipMaterial::with_appearance(
		13,
		&m.surface,
		ClipAppearance::from_spawn(&VfxSpawn::default()),
	);
	assert_eq!(baseline.dark, Vec4::new(0.34, 0.19, 0.085, 1.));
	assert_eq!(baseline.mid, Vec4::new(0.58, 0.37, 0.17, 1.));
	assert_eq!(baseline.bright, Vec4::new(0.84, 0.59, 0.31, 1.));
	assert!(<ClipMaterial as Material>::enable_prepass());
	assert!(!<ClipMaterial as Material>::enable_shadows());
	Ok(())
}
#[test]
fn two_players_share_clip_but_capture_separate_appearance() -> TestResult {
	use crate::{
		composition::MeshClipPart,
		parts::mesh_clip::{MeshClip, MeshClipPlayer},
		VfxSpawn,
	};
	let mut world = World::new();
	let root_a = world.spawn_empty().id();
	let root_b = world.spawn_empty().id();
	let part = MeshClipPart { path: CartoonDust::CLIP.into() };
	let a = part.spawn(&mut world.commands(), root_a, Transform::IDENTITY, &VfxSpawn::default());
	let b = part.spawn(
		&mut world.commands(),
		root_b,
		Transform::IDENTITY,
		&VfxSpawn { tint: Some(Color::linear_rgba(0.5, 1., 0.25, 0.)), intensity: 2., ..default() },
	);
	world.flush();
	let mut clips = Assets::<MeshClip>::default();
	let clip = clips.add(MeshClip { manifest: manifest()?, meshes: vec![] });
	world.get_mut::<MeshClipPlayer>(a).ok_or("a")?.clip = Some(clip.clone());
	world.get_mut::<MeshClipPlayer>(b).ok_or("b")?.clip = Some(clip.clone());
	let pa = world.get::<MeshClipPlayer>(a).ok_or("a")?;
	let pb = world.get::<MeshClipPlayer>(b).ok_or("b")?;
	assert_eq!(pa.clip, pb.clip);
	let asset = clips.get(&clip).ok_or("clip")?;
	let mut materials = Assets::<ClipMaterial>::default();
	let ma =
		materials.add(ClipMaterial::with_appearance(13, &asset.manifest.surface, pa.appearance));
	let mb =
		materials.add(ClipMaterial::with_appearance(13, &asset.manifest.surface, pb.appearance));
	assert_ne!(ma, mb);
	let before = materials.get(&mb).ok_or("mb")?.mid;
	assert_eq!(before, Vec4::new(0.58, 0.74, 0.085, 1.));
	materials.get_mut(&ma).ok_or("ma")?.mid = Vec4::ZERO;
	assert_eq!(materials.get(&mb).ok_or("mb")?.mid, before);
	assert_eq!(asset.manifest.surface.colors[1], [0.58, 0.37, 0.17]);
	Ok(())
}
