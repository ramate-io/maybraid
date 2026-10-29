//! Render a High → Mid → Low → High cycle, then finish an old Mid asset late.
//!
//! The late-asset screenshot should contain only the red High sphere. The
//! final Mid revisit should show the green square again. Before the completion
//! fix, a green square leaks into the late-asset screenshot beside the sphere.
//!
//! Run `cargo run -p lod --example retirement_capture`; captures default to
//! `/tmp`, or set `LOD_RETIREMENT_CAPTURE_DIR` to an existing output directory.
//! The 40-second sequence leaves time for render pipelines and readbacks.

use bevy::audio::AudioPlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use bevy::world_serialization::{WorldAsset, WorldAssetRoot};
use lod::scene::host::{hide_lod_tree, show_lod_tree, LodLevelRoot};
use lod::{LodRefreshCorePlugin, LodSceneLevel};
#[derive(Resource)]
struct Roots {
	high: Entity,
	mid: Entity,
	low: Entity,
	delayed: Handle<WorldAsset>,
}
#[derive(Resource, Default)]
struct Tick(u32);

enum Step {
	Photograph(&'static str),
	ShowMid,
	ShowLow,
	ShowHigh,
	CompleteMidAsset,
	Finish,
}

const STEPS: &[(f64, Step)] = &[
	(6.0, Step::Photograph("high-before")),
	(8.0, Step::ShowMid),
	(12.0, Step::Photograph("mid")),
	(14.0, Step::ShowLow),
	(18.0, Step::Photograph("low")),
	(20.0, Step::ShowHigh),
	(24.0, Step::Photograph("high-return")),
	(26.0, Step::CompleteMidAsset),
	(32.0, Step::Photograph("late-mid-world-asset")),
	(34.0, Step::ShowMid),
	(38.0, Step::Photograph("late-mid-revealed")),
	(40.0, Step::Finish),
];

fn main() {
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window { resolution: (640, 480).into(), ..default() }),
					..default()
				})
				.disable::<AudioPlugin>(),
		)
		.add_plugins(LodRefreshCorePlugin)
		.insert_resource(ClearColor(Color::BLACK))
		.init_resource::<Tick>()
		.add_systems(Startup, setup)
		.add_systems(Update, cycle)
		.run();
}
fn setup(
	mut c: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut mats: ResMut<Assets<StandardMaterial>>,
	worlds: ResMut<Assets<WorldAsset>>,
) {
	c.spawn((
		Camera3d::default(),
		Transform::from_xyz(0.0, 0.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
	));
	let high = c
		.spawn((LodLevelRoot(LodSceneLevel::High), Transform::default(), Visibility::Inherited))
		.id();
	let mid = c
		.spawn((LodLevelRoot(LodSceneLevel::Medium), Transform::default(), Visibility::Hidden))
		.id();
	let low = c
		.spawn((LodLevelRoot(LodSceneLevel::Low), Transform::default(), Visibility::Hidden))
		.id();
	let red = mats.add(StandardMaterial {
		base_color: Color::srgb(1.0, 0.0, 0.0),
		unlit: true,
		..default()
	});
	let green = mats.add(StandardMaterial {
		base_color: Color::srgb(0.0, 1.0, 0.0),
		unlit: true,
		..default()
	});
	let blue = mats.add(StandardMaterial {
		base_color: Color::srgb(0.0, 0.0, 1.0),
		unlit: true,
		..default()
	});
	c.spawn((
		Mesh3d(meshes.add(Sphere::new(0.75))),
		MeshMaterial3d(red),
		Transform::default(),
		ChildOf(high),
	));
	c.spawn((
		Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 0.3))),
		MeshMaterial3d(green),
		Transform::from_xyz(0.0, 0.0, -0.25),
		ChildOf(mid),
	));
	c.spawn((
		Mesh3d(meshes.add(Cuboid::new(2.8, 2.8, 0.3))),
		MeshMaterial3d(blue),
		Transform::from_xyz(0.0, 0.0, -0.5),
		ChildOf(low),
	));
	let delayed = worlds.reserve_handle();
	c.spawn((
		WorldAssetRoot(delayed.clone()),
		Transform::default(),
		Visibility::Inherited,
		ChildOf(mid),
	));
	c.insert_resource(Roots { high, mid, low, delayed });
}
fn shot(c: &mut Commands, name: &str) {
	c.spawn(Screenshot::primary_window()).observe(save_to_disk(
		std::path::Path::new(
			&std::env::var("LOD_RETIREMENT_CAPTURE_DIR").unwrap_or_else(|_| "/tmp".into()),
		)
		.join(format!("wall-retirement-{name}.png")),
	));
}
fn cycle(
	mut c: Commands,
	roots: Res<Roots>,
	mut tick: ResMut<Tick>,
	time: Res<Time<Real>>,
	mut exit: MessageWriter<AppExit>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut mats: ResMut<Assets<StandardMaterial>>,
	mut worlds: ResMut<Assets<WorldAsset>>,
) {
	let Some((at, step)) = STEPS.get(tick.0 as usize) else { return };
	if time.elapsed_secs_f64() < *at {
		return;
	}
	tick.0 += 1;
	match step {
		Step::Photograph(name) => shot(&mut c, name),
		Step::ShowMid => {
			hide_lod_tree(&mut c, roots.high);
			show_lod_tree(&mut c, roots.mid);
		}
		Step::ShowLow => {
			hide_lod_tree(&mut c, roots.mid);
			show_lod_tree(&mut c, roots.low);
		}
		Step::ShowHigh => {
			hide_lod_tree(&mut c, roots.low);
			show_lod_tree(&mut c, roots.high);
		}
		Step::CompleteMidAsset => {
			let mat = mats.add(StandardMaterial {
				base_color: Color::srgb(0.0, 1.0, 0.0),
				unlit: true,
				..default()
			});
			let mut world = World::new();
			world.spawn((
				Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 0.3))),
				MeshMaterial3d(mat),
				Transform::from_xyz(0.0, 0.0, -0.25),
				Visibility::Inherited,
			));
			assert!(worlds.insert(roots.delayed.id(), WorldAsset::new(world)).is_ok());
		}
		Step::Finish => {
			exit.write(AppExit::Success);
		}
	}
}
