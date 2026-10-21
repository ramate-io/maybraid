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
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use bevy::transform::TransformSystems;
use combat_hud::CombatHudPlugin;
use damage::{DamageApplied, DamagePlugin, DamageSystems, Health};
use fisticuffs_user::{
	draw_fist_sensors, resolve_fist_hits, sync_fist_sensors, AttackSpec, AttackTiming,
	FisticuffsHitSensor, FisticuffsSensorDebug, FisticuffsUser, JabAim, PLACEHOLDER_FIST_RADIUS,
	PLACEHOLDER_FOREARM_LENGTH, PLACEHOLDER_JAB_DAMAGE, Side, JAB_STARTUP_END,
};
use lod_avian::PhysicsInteractionLayer;
use player::Player;

fn assets_path() -> PathBuf {
	Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets")
}

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
				.set(AssetPlugin { file_path: assets_path().to_string_lossy().into(), ..default() })
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
		.add_systems(Update, (sync_fist_sensors, paint_sensor_volume, draw_fist_sensors))
		.add_systems(
			PostUpdate,
			resolve_fist_hits
				.after(TransformSystems::Propagate)
				.before(DamageSystems::Apply),
		)
		.add_systems(Update, (replay_hit_marker, capture_scene, advance_after_shot).chain())
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

#[derive(Component)]
struct VisibleFistVolume;

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

	fn warmup(self) -> Duration {
		match self {
			Self::SensorVolume => Duration::from_millis(1800),
			Self::HitMarker => Duration::from_millis(280),
			Self::Done => Duration::ZERO,
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
		Transform::from_xyz(2.6, 1.55, 0.15).looking_at(Vec3::new(0.12, 1.15, -0.95), Vec3::Y),
	));
	commands.spawn((
		DirectionalLight { illuminance: 2_200.0, shadow_maps_enabled: false, ..default() },
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
		Mesh3d(meshes.add(Capsule3d::new(0.22, 1.0))),
		MeshMaterial3d(materials.add(StandardMaterial {
			base_color: Color::srgba(0.35, 0.55, 0.78, 0.42),
			alpha_mode: AlphaMode::Blend,
			..default()
		})),
		RigidBody::Kinematic,
		Collider::capsule(0.45, 1.1),
		PhysicsInteractionLayer::animated_layers(),
	));
}

fn paint_sensor_volume(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	sensors: Query<Entity, (With<FisticuffsHitSensor>, Without<VisibleFistVolume>)>,
) {
	for entity in &sensors {
		commands.entity(entity).insert((
			VisibleFistVolume,
			Mesh3d(meshes.add(Capsule3d::new(
				PLACEHOLDER_FIST_RADIUS + 0.04,
				PLACEHOLDER_FOREARM_LENGTH,
			))),
			MeshMaterial3d(materials.add(StandardMaterial {
				base_color: Color::srgb(1.0, 0.45, 0.12),
				emissive: LinearRgba::rgb(4.0, 1.2, 0.2),
				unlit: true,
				..default()
			})),
		));
	}
}

fn replay_hit_marker(
	shots: Res<CaptureShots>,
	player: Query<Entity, With<Player>>,
	dummy: Query<(Entity, &Health, &Transform), With<DummyTarget>>,
	mut applied: MessageWriter<DamageApplied>,
	mut replayed: Local<bool>,
) {
	if shots.scene != CaptureScene::HitMarker || *replayed {
		return;
	}
	let Ok(source) = player.single() else {
		return;
	};
	let Ok((target, health, transform)) = dummy.single() else {
		return;
	};
	applied.write(DamageApplied {
		target,
		source: Some(source),
		amount: PLACEHOLDER_JAB_DAMAGE,
		applied: PLACEHOLDER_JAB_DAMAGE,
		remaining: health.current,
		point: transform.translation + Vec3::Y * 0.35,
	});
	*replayed = true;
}

fn capture_scene(
	mut commands: Commands,
	time: Res<Time>,
	dir: Res<CaptureDir>,
	mut shots: ResMut<CaptureShots>,
	sensors: Query<(), With<FisticuffsHitSensor>>,
	dummy: Query<&Health, With<DummyTarget>>,
	names: Query<&Name>,
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
	let warmed = time.elapsed() > shown + shots.scene.warmup();
	let marker_present = names.iter().any(|name| name.as_str() == "hit-marker");
	let ready = match shots.scene {
		CaptureScene::SensorVolume => warmed && !sensors.is_empty(),
		CaptureScene::HitMarker => {
			let hurt = dummy.iter().any(|health| health.current < 100.0 - 0.5);
			warmed && (hurt || marker_present)
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
