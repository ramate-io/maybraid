//! Capture opening, peak and breakup phases with a fixed inspection clock.
use bevy::{prelude::*, transform::TransformSystems};
use maybraid_vfx::VfxInstance;
use std::{
	path::PathBuf,
	time::{Duration, Instant},
};
use vfx_playground::{PlaygroundCommand, VfxPlaygroundPlugin};
#[derive(Resource)]
struct Probe {
	output: PathBuf,
	index: usize,
	begun: bool,
	hold: Option<Instant>,
	frames: u32,
}
const AGES: [f32; 3] = [0.08, 0.30, 0.72];
fn main() -> Result<(), Box<dyn std::error::Error>> {
	let output = PathBuf::from(std::env::args().nth(1).ok_or("output required")?);
	std::fs::create_dir_all(&output)?;
	let mut app = App::new();
	app.add_plugins(
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
	.add_plugins(VfxPlaygroundPlugin)
	.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(Duration::from_secs_f64(
		1. / 120.,
	)))
	.insert_resource(game_commands::ui::GameCommandDrawerVisible(false))
	.insert_resource(Probe { output, index: 0, begun: false, hold: None, frames: 0 })
	.add_systems(PostUpdate, camera.before(TransformSystems::Propagate))
	.add_systems(Last, capture);
	app.run();
	Ok(())
}
fn camera(p: Res<Probe>, mut q: Query<&mut Transform, With<Camera3d>>) {
	let pos = match p.index % 3 {
		0 => Vec3::new(2.4, 1.6, 3.2),
		1 => Vec3::new(1.3, 0.75, 1.5),
		_ => Vec3::new(0.1, 3., 0.2),
	};
	for mut t in &mut q {
		*t = Transform::from_translation(pos).looking_at(Vec3::new(0., 0.12, 0.), Vec3::Y);
	}
}
fn capture(
	mut p: ResMut<Probe>,
	time: Res<Time<Virtual>>,
	mut commands: Commands,
	roots: Query<&VfxInstance>,
	mut exit: MessageWriter<AppExit>,
) {
	if p.index >= 9 {
		p.frames += 1;
		if p.frames > 30 {
			exit.write(AppExit::Success);
		}
		return;
	}
	let mut console = String::new();
	if !p.begun {
		PlaygroundCommand::Show {
			effect: "cartoon-dust".into(),
			scale: 1.,
			intensity: 1.,
			distance: 0.,
			seed: Some(13),
			playback: 1.,
		}
		.react(&mut commands, &mut console);
		PlaygroundCommand::Freeze { age: AGES[p.index / 3] }.react(&mut commands, &mut console);
		p.begun = true;
		return;
	}
	if !time.is_paused() {
		return;
	}
	let hold = p.hold.get_or_insert_with(Instant::now).elapsed().as_secs_f32();
	if hold > 2. && p.frames == 0 {
		PlaygroundCommand::Screenshot { path: p.output.join(format!("view-{}.png", p.index)) }
			.react(&mut commands, &mut console);
		if let Err(e) = std::fs::write(
			p.output.join(format!("view-{}.txt", p.index)),
			format!("root ages={:?}", roots.iter().map(|i| i.age).collect::<Vec<_>>()),
		) {
			eprintln!("{e}");
		}
		p.frames = 1;
	}
	if hold > 3. {
		PlaygroundCommand::Cancel.react(&mut commands, &mut console);
		p.index += 1;
		p.begun = false;
		p.hold = None;
		p.frames = 0;
	}
}
