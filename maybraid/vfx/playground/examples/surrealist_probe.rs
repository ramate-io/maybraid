//! Fixed-step native visual captures, not a performance measurement.
//! `cargo run -p vfx-playground --example surrealist_probe -- /tmp/surrealist-views`
use bevy::prelude::*;
use bevy::window::PresentMode;
use std::{path::PathBuf, time::Instant};
use vfx_playground::{PlaygroundCommand, VfxPlaygroundPlugin};

#[derive(Resource)]
struct Probe {
	output: PathBuf,
	stage: usize,
	start: Instant,
	shot: bool,
	initialized: bool,
}
const VIEWS: [(&str, Vec3); 11] = [
	("baseline", Vec3::new(2.4, 1.6, 3.2)),
	("opening-outside", Vec3::new(2.4, 1.6, 3.2)),
	("opening-side", Vec3::new(3.2, 1.8, -2.4)),
	("opening-inside", Vec3::new(0.8, 0.6, 0.6)),
	("peak-outside", Vec3::new(2.4, 1.6, 3.2)),
	("peak-side", Vec3::new(3.2, 1.8, -2.4)),
	("peak-inside", Vec3::new(0.8, 0.6, 0.6)),
	("peak-close", Vec3::new(1.6, 0.8, 1.8)),
	("tail-outside", Vec3::new(2.4, 1.6, 3.2)),
	("tail-side", Vec3::new(3.2, 1.8, -2.4)),
	("tail-inside", Vec3::new(0.8, 0.6, 0.6)),
];
fn main() {
	let output = std::env::args_os()
		.nth(1)
		.map(PathBuf::from)
		.unwrap_or_else(|| PathBuf::from("/tmp/volume-probe"));
	if let Err(error) = std::fs::create_dir_all(&output) {
		eprintln!("{error}");
		return;
	}
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						resolution: (960, 540).into(),
						present_mode: PresentMode::AutoNoVsync,
						..default()
					}),
					..default()
				})
				.set(AssetPlugin {
					file_path: format!("{}/../../assets", env!("CARGO_MANIFEST_DIR")),
					..default()
				}),
		)
		.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
			std::time::Duration::from_secs_f64(1.0 / 120.0),
		))
		.insert_resource(bevy::winit::WinitSettings::continuous())
		.insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.11)))
		.insert_resource(Probe {
			output,
			stage: 0,
			start: Instant::now(),
			shot: false,
			initialized: false,
		})
		.add_plugins(VfxPlaygroundPlugin)
		.insert_resource(game_commands::ui::GameCommandDrawerVisible(false))
		.add_systems(Startup, occluder)
		.add_systems(Update, probe)
		.run();
}
fn occluder(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	commands.spawn((
		Mesh3d(meshes.add(Cuboid::new(0.3, 2.5, 0.3))),
		MeshMaterial3d(
			materials
				.add(StandardMaterial { base_color: Color::srgb(0.15, 0.35, 0.65), ..default() }),
		),
		Transform::from_xyz(0.1, 1.25, 0.),
	));
}
fn probe(
	mut state: ResMut<Probe>,
	clock: Res<Time<Virtual>>,
	mut commands: Commands,
	mut exit: MessageWriter<AppExit>,
	instances: Query<&maybraid_vfx::VfxInstance>,
) {
	if state.stage >= VIEWS.len() {
		return;
	}
	let mut console = String::new();
	if !state.initialized {
		state.initialized = true;
		state.start = Instant::now();
		let (_, p) = VIEWS[state.stage];
		PlaygroundCommand::Cancel.react(&mut commands, &mut console);
		PlaygroundCommand::View { x: p.x, y: p.y, z: p.z }.react(&mut commands, &mut console);
		if state.stage > 0 {
			PlaygroundCommand::Show {
				effect: "surrealist-dust".into(),
				scale: 1.,
				intensity: 1.,
				distance: 0.,
				seed: Some(13),
				playback: 1.,
			}
			.react(&mut commands, &mut console);
			PlaygroundCommand::Freeze {
				age: if state.stage < 4 {
					0.2
				} else if state.stage < 8 {
					1.0
				} else {
					3.8
				},
			}
			.react(&mut commands, &mut console);
		}
	}

	if state.stage > 0 && !clock.is_paused() {
		state.start = Instant::now();
		return;
	}
	let elapsed = state.start.elapsed().as_secs_f32();
	if elapsed > 6.0 && !state.shot {
		state.shot = true;
		let info = format!("ages={:?}\n", instances.iter().map(|v| v.age).collect::<Vec<_>>());
		if let Err(e) =
			std::fs::write(state.output.join(format!("{}.txt", VIEWS[state.stage].0)), info)
		{
			eprintln!("{e}");
			exit.write(AppExit::error());
			return;
		}
		PlaygroundCommand::Screenshot {
			path: state.output.join(format!("{}.png", VIEWS[state.stage].0)),
		}
		.react(&mut commands, &mut console);
	}
	if elapsed > 8.0 {
		println!("captured {}", VIEWS[state.stage].0);
		state.stage += 1;
		state.initialized = false;
		state.shot = false;
		if state.stage == VIEWS.len() {
			exit.write(AppExit::Success);
		}
	}
}
