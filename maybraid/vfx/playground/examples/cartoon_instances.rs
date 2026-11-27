//! Native transformed-instance, independent-clock and cleanup evidence; no game systems.
use bevy::{prelude::*, transform::TransformSystems};
use maybraid_vfx::{parts::mesh_clip::ClipMaterial, VfxInstance};
use std::path::PathBuf;
use vfx_playground::{PlaygroundCommand, VfxPlaygroundPlugin};
#[derive(Component)]
struct Placed;
#[derive(Resource)]
struct Probe {
	stage: u32,
	frames: u32,
	dir: PathBuf,
	other: Entity,
	natural_seen: bool,
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
	let dir = PathBuf::from(std::env::args().nth(1).ok_or("output required")?);
	std::fs::create_dir_all(&dir)?;
	let mut app = App::new();
	app.add_plugins(DefaultPlugins.set(AssetPlugin {
		file_path: concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets").into(),
		..default()
	}))
	.add_plugins(VfxPlaygroundPlugin);
	let other = app.world_mut().spawn(Name::new("unrelated sentinel")).id();
	app.insert_resource(Probe { stage: 0, frames: 0, dir, other, natural_seen: false })
		.add_systems(Startup, start)
		.add_systems(Update, probe)
		.add_systems(PostUpdate, place.before(TransformSystems::Propagate))
		.run();
	Ok(())
}
fn start(mut commands: Commands) {
	for playback in [0.8, 1.2] {
		PlaygroundCommand::Show {
			effect: "cartoon-dust".into(),
			scale: 1.,
			intensity: 1.,
			distance: 0.,
			seed: Some(13),
			playback,
		}
		.react(&mut commands, &mut String::new());
	}
	PlaygroundCommand::Noloop.react(&mut commands, &mut String::new());
}
fn place(
	mut commands: Commands,
	mut roots: Query<(Entity, &VfxInstance, &mut Transform), Without<Placed>>,
	mut camera: Query<&mut Transform, (With<Camera3d>, Without<VfxInstance>)>,
) {
	for (e, i, mut t) in &mut roots {
		let sign = if i.playback < 1. { -1. } else { 1. };
		*t = Transform::from_xyz(sign * 1.6, 0., sign * 0.5)
			.with_rotation(Quat::from_rotation_y(sign * 0.5))
			.with_scale(Vec3::splat(0.7));
		commands.entity(e).insert(Placed);
	}
	for mut t in &mut camera {
		*t = Transform::from_xyz(6., 4., 8.).looking_at(Vec3::new(0.7, 0.6, 0.), Vec3::Y);
	}
}
fn probe(
	mut commands: Commands,
	mut p: ResMut<Probe>,
	roots: Query<(Entity, &VfxInstance, &GlobalTransform)>,
	mut time: ResMut<Time<Virtual>>,
	materials: Res<Assets<ClipMaterial>>,
	images: Res<Assets<Image>>,
	clips: Res<Assets<maybraid_vfx::parts::mesh_clip::MeshClip>>,
	meshes: Res<Assets<Mesh>>,
	mut exit: MessageWriter<AppExit>,
) {
	if p.stage == 0 {
		if roots.iter().count() == 2
			&& roots.iter().all(|(_, i, _)| i.armed)
			&& roots.iter().any(|(_, i, _)| i.age >= 0.3)
		{
			time.pause();
			p.stage = 1;
			p.frames = 0;
		}
		return;
	}
	if p.stage >= 5 {
		p.frames += 1;
		if p.frames >= 30 {
			exit.write(AppExit::Success);
		}
		return;
	}
	if p.stage == 4 {
		p.natural_seen |= roots.iter().any(|(_, i, _)| i.armed);
		if !p.natural_seen || !roots.is_empty() {
			return;
		}
	}
	p.frames += 1;
	if p.frames < 20 {
		return;
	}
	p.frames = 0;
	let state = format!(
		"stage={} instances={} materials={} images={} clips={} meshes={} ages={:?} transforms={:?}\n",
		p.stage,
		roots.iter().count(),
		materials.len(),
		images.len(),
        clips.len(), meshes.len(),
		roots.iter().map(|(_, i, _)| i.age).collect::<Vec<_>>(),
		roots.iter().map(|(_, _, t)| t.to_matrix().to_cols_array()).collect::<Vec<_>>()
	);
	if let Err(e) = std::fs::write(p.dir.join(format!("state-{}.txt", p.stage)), state) {
		eprintln!("{e}");
		exit.write(AppExit::error());
		return;
	}
	PlaygroundCommand::Screenshot { path: p.dir.join(format!("stage-{}.png", p.stage)) }
		.react(&mut commands, &mut String::new());
	match p.stage {
		1 => {
			if let Some((e, _, _)) = roots.iter().next() {
				commands.entity(e).try_despawn();
			}
		}
		2 => {
			for (e, _, _) in &roots {
				commands.entity(e).try_despawn();
			}
		}
		3 => {
			if !roots.is_empty() {
				exit.write(AppExit::error());
				return;
			}
			commands.entity(p.other).insert(Name::new("unrelated sentinel survived"));
			time.unpause();
			PlaygroundCommand::Show {
				effect: "cartoon-dust".into(),
				scale: 1.,
				intensity: 1.,
				distance: 0.,
				seed: Some(13),
				playback: 1.,
			}
			.react(&mut commands, &mut String::new());
			PlaygroundCommand::Noloop.react(&mut commands, &mut String::new());
		}
		4 => {
			if !materials.is_empty() {
				exit.write(AppExit::error());
				return;
			}
		}
		_ => {
			exit.write(AppExit::Success);
		}
	}
	p.stage += 1;
}
