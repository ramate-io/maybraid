//! Native shared MeshClipPart + VolumeFlipbookPart occlusion comparisons, not a benchmark.
use bevy::{
	core_pipeline::prepass::DepthPrepass,
	prelude::*,
	render::view::screenshot::{save_to_disk, Screenshot},
};
use maybraid_vfx::{
	effects::cartoon_dust::CartoonDust,
	parts::mesh_clip::{ClipBody, ClipMaterial, MeshClip},
	spawn_vfx, EffectDefinition, EffectLayer, EffectRecipe, PackagedVolumeFlipbook, VfxInstance,
	VfxPlugin, VfxSpawn, VolumeFlipbookMaterial,
};
use std::{
	path::PathBuf,
	time::{Duration, Instant},
};
#[derive(Clone, Copy)]
struct Case {
	name: &'static str,
	age: f32,
	mesh: bool,
	volume: bool,
	z: f32,
	empty: bool,
}
const CASES: [Case; 10] = [
	Case { name: "mesh-peak", age: 0.3, mesh: true, volume: false, z: -2., empty: false },
	Case { name: "volume-behind", age: 0.3, mesh: false, volume: true, z: -2., empty: false },
	Case { name: "behind", age: 0.3, mesh: true, volume: true, z: -2., empty: false },
	Case { name: "volume-front", age: 0.3, mesh: false, volume: true, z: 2., empty: false },
	Case { name: "front", age: 0.3, mesh: true, volume: true, z: 2., empty: false },
	Case { name: "mesh-partial", age: 0.75, mesh: true, volume: false, z: -2., empty: false },
	Case { name: "partial-behind", age: 0.75, mesh: true, volume: true, z: -2., empty: false },
	Case { name: "partial-overlap", age: 0.75, mesh: true, volume: true, z: 0.15, empty: false },
	Case { name: "volume-overlap", age: 0.75, mesh: false, volume: true, z: 0.15, empty: false },
	Case { name: "dissolved-overlap", age: 0.75, mesh: true, volume: true, z: 0.15, empty: true },
];
#[derive(Component)]
struct MeshRoot;
#[derive(Component)]
struct VolumeRoot;
#[derive(Resource)]
struct Probe {
	out: PathBuf,
	index: usize,
	spawned: bool,
	hold: Option<Instant>,
	shot: bool,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
	let out = PathBuf::from(std::env::args().nth(1).ok_or("output directory required")?);
	std::fs::create_dir_all(&out)?;
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(AssetPlugin {
					file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").into(),
					..default()
				})
				.set(WindowPlugin {
					primary_window: Some(Window { resolution: (960, 540).into(), ..default() }),
					..default()
				}),
		)
		.add_plugins(VfxPlugin)
		.insert_resource(ClearColor(Color::srgb(0.04, 0.08, 0.14)))
		.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
			1. / 120.,
		)))
		.insert_resource(Probe { out, index: 0, spawned: false, hold: None, shot: false })
		.add_systems(Startup, camera)
		.add_systems(Update, probe)
		.add_systems(PostUpdate, fixture_state.before(bevy::transform::TransformSystems::Propagate))
		.run();
	Ok(())
}
fn camera(mut commands: Commands) {
	commands.spawn((
		Camera3d::default(),
		DepthPrepass,
		Transform::from_xyz(0., 0., 5.).looking_at(Vec3::ZERO, Vec3::Y),
	));
}
fn fixture_state(
	p: Res<Probe>,
	assets: Res<Assets<PackagedVolumeFlipbook>>,
	mut volumes: ResMut<Assets<VolumeFlipbookMaterial>>,
	mut surfaces: ResMut<Assets<ClipMaterial>>,
	clips: Res<Assets<MeshClip>>,
	mut bodies: Query<(&ClipBody, &MeshMaterial3d<ClipMaterial>, &mut Transform, &mut Visibility)>,
) {
	// Inspection only: exact matched poses, independent of freeze-command tick overshoot.
	if p.hold.is_some() {
		if let Some((_, clip)) = clips.iter().next() {
			for (body, material, mut transform, mut visible) in &mut bodies {
				if let Some(pose) =
					clip.manifest.sample(body.track, CASES[p.index.min(CASES.len() - 1)].age)
				{
					*transform = pose.transform();
					*visible = Visibility::Inherited;
					if let Some(mut m) = surfaces.get_mut(&material.0) {
						m.cutoff.x = pose.cutoff;
					}
				} else {
					*visible = Visibility::Hidden;
				}
			}
		}
	}
	if let Some((_, asset)) = assets.iter().next() {
		for (_, material) in volumes.iter_mut() {
			material.set_age(asset, 0.75);
		}
	}
	if CASES[p.index.min(CASES.len() - 1)].empty {
		for (_, m) in surfaces.iter_mut() {
			m.cutoff.x = 1.1;
		}
	}
}
fn probe(
	mut commands: Commands,
	mut p: ResMut<Probe>,
	mut time: ResMut<Time<Virtual>>,
	roots: Query<(Entity, &VfxInstance)>,
	mut mesh_roots: Query<&mut Visibility, With<MeshRoot>>,
	mut volume_roots: Query<&mut Visibility, (With<VolumeRoot>, Without<MeshRoot>)>,
	mut exit: MessageWriter<AppExit>,
) {
	if p.index >= CASES.len() {
		exit.write(AppExit::Success);
		return;
	}
	let case = CASES[p.index];
	if !p.spawned {
		time.unpause();
		let root = spawn_vfx(
			&mut commands,
			&CartoonDust.effect_definition(),
			VfxSpawn { seed: Some(13), ..default() },
		);
		commands.entity(root).insert((
			MeshRoot,
			Transform::from_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
		));
		let def = EffectDefinition::new(
			"fixture",
			[EffectLayer::volume_flipbook_path(
				"vfx/fixtures/volume_puffs/puffs.volume_flipbook.json",
			)],
		);
		let root = spawn_vfx(&mut commands, &def, VfxSpawn::default());
		commands.entity(root).insert((
			VolumeRoot,
			Transform::from_xyz(0., 0., case.z).with_scale(Vec3::splat(0.75)),
		));
		p.spawned = true;
		return;
	}
	if !time.is_paused() {
		if roots.iter().count() == 2 && roots.iter().all(|(_, i)| i.armed && i.age >= case.age) {
			time.pause();
		} else {
			return;
		}
	}
	for mut v in &mut mesh_roots {
		*v = if case.mesh { Visibility::Inherited } else { Visibility::Hidden };
	}
	for mut v in &mut volume_roots {
		*v = if case.volume { Visibility::Inherited } else { Visibility::Hidden };
	}
	let held = p.hold.get_or_insert_with(Instant::now).elapsed().as_secs_f32();
	if held > 2. && !p.shot {
		commands
			.spawn(Screenshot::primary_window())
			.observe(save_to_disk(p.out.join(format!("{}.png", case.name))));
		p.shot = true;
	}
	if held > 3. {
		for (root, _) in &roots {
			commands.entity(root).try_despawn();
		}
		p.index += 1;
		p.spawned = false;
		p.shot = false;
		p.hold = None;
	}
}
