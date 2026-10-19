//! Headless stills of the fist sensor volume and a combat-hud hit marker.
//!
//! ```bash
//! FISTICUFFS_JAB_CAPTURE_DIR=/tmp/fisticuffs-jab \
//!   VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!   WGPU_BACKEND=vk \
//!   xvfb-run -a cargo run -p menu-playground --example fisticuffs_jab_hit_capture
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use avian3d::prelude::*;
use bevy::audio::AudioPlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{Capturing, Screenshot, save_to_disk};
use bevy::transform::TransformSystems;
use combat_hud::CombatHudPlugin;
use damage::{DamageApplied, DamagePlugin, DamageSystems, Health};
use fisticuffs_user::{
	AttackSpec, AttackTiming, FisticuffsHitSensor, FisticuffsSensorDebug, FisticuffsUser,
	JAB_STARTUP_END, JabAim, PLACEHOLDER_JAB_DAMAGE, Side, draw_fist_sensors, resolve_fist_hits,
	sync_fist_sensors,
};
use lod_avian::PhysicsInteractionLayer;
use player::Player;

fn shot_dir() -> PathBuf {
	std::env::var_os("FISTICUFFS_JAB_CAPTURE_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| Path::new("/tmp/fisticuffs-jab").to_path_buf())
}

fn main() {
	let dir = shot_dir();
	let _ = std::fs::create_dir_all(&dir);
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Fisticuffs jab hit capture".into(),
						resolution: (960, 720).into(),
						..default()
					}),
					..default()
				})
				.disable::<AudioPlugin>(),
		)
		.add_plugins(PhysicsPlugins::default())
		.add_plugins(DamagePlugin)
		.add_plugins(CombatHudPlugin {
			hit_markers: true,
			directional_damage: false,
			player_vitals: false,
			lubikan_charge: false,
			lubikan_point_feedback: false,
			score: false,
		})
		.insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.11)))
		.insert_resource(FisticuffsSensorDebug(true))
		.insert_resource(CaptureDir(dir))
		.init_resource::<CaptureShots>()
		.add_systems(Startup, setup)
		.add_systems(Update, (sync_fist_sensors, draw_fist_sensors))
		.add_systems(
			PostUpdate,
			resolve_fist_hits
				.after(TransformSystems::Propagate)
				.before(DamageSystems::Apply),
		)
		.add_systems(Update, (capture_scene, advance_after_shot).chain())
		.run();
}

#[derive(Resource)]
struct CaptureDir(PathBuf);

#[derive(Resource, Default)]
struct CaptureShots {
	scene: CaptureScene,
	requested: bool,
	shown_at: Option<Duration>,
}

#[derive(Component)]
struct DummyTarget;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CaptureScene {
	#[default]
	SensorVolume,
	HitMarker,
	Done,
}

impl CaptureScene {
	fn next(self) -> Self {
		match self {
			Self::SensorVolume => Self::HitMarker,
			Self::HitMarker | Self::Done => Self::Done,
		}
	}

	fn file_stem(self) -> &'static str {
		match self {
			Self::SensorVolume => "fist_sensor_volume",
			Self::HitMarker => "hit_marker_on_dummy",
			Self::Done => "done",
		}
	}
}

fn setup(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	commands.spawn((
		Camera3d::default(),
		Transform::from_xyz(3.4, 1.7, 1.1).looking_at(Vec3::new(0.15, 0.7, -1.05), Vec3::Y),
	));
	commands.spawn((
		DirectionalLight { illuminance: 2_200.0, shadows_enabled: false, ..default() },
		Transform::from_xyz(4.0, 8.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
	));
	commands.spawn((
		Mesh3d(meshes.add(Plane3d::default().mesh().size(8.0, 8.0))),
		MeshMaterial3d(materials.add(Color::srgb(0.14, 0.15, 0.17))),
		Transform::from_xyz(0.0, 0.0, -0.6),
	));

	let mut user = FisticuffsUser::default();
	user.state.request(AttackSpec::jab(Side::Left));
	user.state.latch_aim(JabAim::level());
	user.state.tick(AttackTiming::JAB.duration_secs * JAB_STARTUP_END + 1e-4);
	commands.spawn((
		Name::new("fisticuffs-attacker"),
		Player,
		user,
		Health::from_max(100.0),
		Transform::from_translation(Vec3::new(0.0, 0.9, 0.0)),
		Mesh3d(meshes.add(Capsule3d::new(0.28, 1.0))),
		MeshMaterial3d(materials.add(Color::srgb(0.72, 0.62, 0.48))),
	));

	commands.spawn((
		Name::new("dummy-target"),
		DummyTarget,
		Health::from_max(100.0),
		Transform::from_translation(Vec3::new(0.16, 0.9, -1.28)),
		Mesh3d(meshes.add(Capsule3d::new(0.32, 1.0))),
		MeshMaterial3d(materials.add(Color::srgb(0.35, 0.55, 0.78))),
		RigidBody::Kinematic,
		Collider::capsule(0.4, 1.0),
		PhysicsInteractionLayer::animated_layers(),
	));
}

fn capture_scene(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<CaptureDir>,
	mut shots: ResMut<CaptureShots>,
	sensors: Query<(), With<FisticuffsHitSensor>>,
	dummy: Query<&Health, With<DummyTarget>>,
	applied: MessageReader<DamageApplied>,
) {
	if shots.scene == CaptureScene::Done || shots.requested {
		return;
	}
	if shots.shown_at.is_none() {
		shots.shown_at = Some(time.elapsed());
	}
	let Some(shown) = shots.shown_at else {
		return;
	};
	let warmed = time.elapsed() > shown + Duration::from_millis(400);
	let ready = match shots.scene {
		CaptureScene::SensorVolume => warmed && !sensors.is_empty(),
		CaptureScene::HitMarker => {
			let hurt = dummy.iter().any(|health| health.current < 100.0 - 0.5);
			warmed && (hurt || !applied.is_empty())
		}
		CaptureScene::Done => return,
	};
	if !ready && time.elapsed() <= shown + Duration::from_secs(6) {
		return;
	}
	let path = dir.0.join(format!("{}.png", shots.scene.file_stem()));
	info!(
		path = %path.display(),
		scene = ?shots.scene,
		sensors = sensors.iter().len(),
		dummy = ?dummy.iter().map(|health| health.current).collect::<Vec<_>>(),
		placeholder = PLACEHOLDER_JAB_DAMAGE,
		"capturing fisticuffs jab still"
	);
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path));
	shots.requested = true;
}

fn advance_after_shot(
	time: Res<Time>,
	dir: Res<CaptureDir>,
	mut shots: ResMut<CaptureShots>,
	capturing: Query<(), With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
	mut ready_at: Local<Option<Duration>>,
) {
	if shots.scene == CaptureScene::Done {
		exit.write(AppExit::Success);
		return;
	}
	if !shots.requested {
		if shots.shown_at.is_some_and(|at| time.elapsed() > at + Duration::from_secs(10)) {
			exit.write(AppExit::Success);
		}
		return;
	}
	let path = dir.0.join(format!("{}.png", shots.scene.file_stem()));
	if path.is_file() && capturing.is_empty() && ready_at.is_none() {
		*ready_at = Some(time.elapsed());
	}
	if !(ready_at.is_some_and(|at| time.elapsed() > at + Duration::from_millis(200))
		|| shots.shown_at.is_some_and(|at| time.elapsed() > at + Duration::from_secs(8)))
	{
		return;
	}
	shots.requested = false;
	shots.shown_at = None;
	*ready_at = None;
	shots.scene = shots.scene.next();
	if shots.scene == CaptureScene::Done {
		exit.write(AppExit::Success);
	}
}
