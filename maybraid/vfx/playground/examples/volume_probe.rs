//! Repeatable software/hardware frame-time probe. Screenshots use the playground console command.
//! `cargo run -p vfx-playground --example volume_probe -- /tmp/volume-probe`
use bevy::prelude::*;
use bevy::window::PresentMode;
use std::{path::PathBuf, time::Instant};
use vfx_playground::{PlaygroundCommand, VfxPlaygroundPlugin};

#[derive(Resource)]
struct Probe {
	output: PathBuf,
	stage: usize,
	start: Instant,
	samples: Vec<f64>,
	shot: bool,
	initialized: bool,
	armed_frames: u32,
	stepped: bool,
}
const VIEWS: [(&str, Vec3); 5] = [
	("baseline", Vec3::new(2.4, 1.6, 3.2)),
	("outside", Vec3::new(2.4, 1.6, 3.2)),
	("side", Vec3::new(3.2, 1.8, -2.4)),
	("above", Vec3::new(0.1, 4.8, 0.2)),
	("inside", Vec3::new(0.8, 1.0, 0.6)),
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
		.insert_resource(bevy::winit::WinitSettings::continuous())
		.insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.11)))
		.insert_resource(Probe {
			output,
			stage: 0,
			start: Instant::now(),
			samples: Vec::new(),
			shot: false,
			initialized: false,
			armed_frames: 0,
			stepped: false,
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
	time: Res<Time<Real>>,
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
				effect: "volume-demo".into(),
				scale: 1.,
				intensity: 1.,
				distance: 0.,
				seed: Some(7),
				playback: 1.,
			}
			.react(&mut commands, &mut console);
			PlaygroundCommand::Pause.react(&mut commands, &mut console);
		}
	}
	if state.stage > 0 && !state.stepped && instances.iter().any(|v| v.armed) {
		state.armed_frames += 1;
		if state.armed_frames >= 3 {
			PlaygroundCommand::Step { seconds: 0.75 }.react(&mut commands, &mut console);
			state.stepped = true;
		}
	}
	let elapsed = state.start.elapsed().as_secs_f32();
	if (4.0..6.0).contains(&elapsed) {
		state.samples.push(time.delta_secs_f64() * 1000.);
	}
	if elapsed > 6.0 && !state.shot {
		state.shot = true;
		PlaygroundCommand::Screenshot {
			path: state.output.join(format!("{}.png", VIEWS[state.stage].0)),
		}
		.react(&mut commands, &mut console);
	}
	if elapsed > 8.0 {
		state.samples.sort_by(f64::total_cmp);
		let n = state.samples.len();
		if n > 0 {
			println!(
				"{}: frames={} median_ms={:.3} p95_ms={:.3}",
				VIEWS[state.stage].0,
				n,
				state.samples[n / 2],
				state.samples[(n * 95 / 100).min(n - 1)]
			);
		}
		state.samples.clear();
		state.stage += 1;
		state.initialized = false;
		state.armed_frames = 0;
		state.stepped = false;
		state.shot = false;
		if state.stage == VIEWS.len() {
			exit.write(AppExit::Success);
		}
	}
}
