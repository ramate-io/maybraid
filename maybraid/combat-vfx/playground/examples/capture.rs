//! Headless stills of combat ooze burst and spirit dissolve.
//!
//! ```bash
//! COMBAT_VFX_CAPTURE_DIR=/tmp/combat-vfx \
//!   VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!   WGPU_BACKEND=vk \
//!   xvfb-run -a cargo run -p combat-vfx-playground --example capture
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use combat_vfx::{CombatVfxPlugin, SPIRIT_DISSOLVE_SECS};
use damage::{DamageApplied, Health};
use maybraid_vfx::{EffectRecipe, ExplosionPalette, SpawnVfxExt, VfxLibrary, VfxSpawn, VfxPlugin};
use player::Npc;

fn capture_dir() -> PathBuf {
	std::env::var_os("COMBAT_VFX_CAPTURE_DIR")
		.map(PathBuf::from)
		.unwrap_or_else(|| Path::new("/tmp/combat-vfx").to_path_buf())
}

fn main() {
	let dir = capture_dir();
	let _ = std::fs::create_dir_all(&dir);
	App::new()
		.add_plugins(
			DefaultPlugins
				.set(WindowPlugin {
					primary_window: Some(Window {
						title: "Combat VFX capture".into(),
						resolution: (960u32, 720u32).into(),
						..default()
					}),
					..default()
				})
				.set(AssetPlugin {
					file_path: Path::new(env!("CARGO_MANIFEST_DIR"))
						.join("../../assets")
						.to_string_lossy()
						.into(),
					..default()
				}),
		)
		.add_plugins((VfxPlugin, CombatVfxPlugin))
		.insert_resource(ClearColor(Color::srgb(0.06, 0.07, 0.10)))
		.insert_resource(CaptureDir(dir))
		.init_resource::<CaptureShots>()
		.add_message::<DamageApplied>()
		.add_systems(Startup, setup)
		.add_systems(Update, (capture_frames, advance_shots).chain())
		.run();
}

#[derive(Resource)]
struct CaptureDir(PathBuf);

#[derive(Resource, Default)]
struct CaptureShots {
	phase: CapturePhase,
	requested: bool,
	shown_at: Option<Duration>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum CapturePhase {
	#[default]
	OozeBurst,
	SpiritMid,
	SpiritDone,
	Finished,
}

impl CapturePhase {
	fn file_stem(self) -> &'static str {
		match self {
			Self::OozeBurst => "ooze_burst_mid",
			Self::SpiritMid => "spirit_dissolve_mid",
			Self::SpiritDone => "spirit_dissolve_complete",
			Self::Finished => "finished",
		}
	}

	fn next(self) -> Self {
		match self {
			Self::OozeBurst => Self::SpiritMid,
			Self::SpiritMid => Self::SpiritDone,
			Self::SpiritDone | Self::Finished => Self::Finished,
		}
	}
}

#[derive(Component)]
struct CaptureCamera;

fn setup(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>, mut materials: ResMut<Assets<StandardMaterial>>) {
	let ground = meshes.add(Plane3d::default().mesh().size(8.0, 8.0));
	let mat = materials.add(StandardMaterial {
		base_color: Color::srgb(0.18, 0.19, 0.22),
		..default()
	});
	commands.spawn((Mesh3d(ground), MeshMaterial3d(mat), Transform::IDENTITY));
	commands.spawn((
		CaptureCamera,
		Camera3d::default(),
		Transform::from_xyz(0.0, 1.25, 2.8).looking_at(Vec3::new(0.0, 0.85, 0.0), Vec3::Y),
	));
	commands.spawn((
		DirectionalLight::default(),
		Transform::from_xyz(4.0, 6.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
	));
	commands.spawn((
		Npc,
		Health::from_max(40.0),
		Transform::from_xyz(0.0, 0.9, 0.0),
		GlobalTransform::IDENTITY,
	));
}

fn capture_frames(
	time: Res<Time>,
	mut commands: Commands,
	mut shots: ResMut<CaptureShots>,
	dir: Res<CaptureDir>,
	library: Option<Res<VfxLibrary>>,
	capturing: Query<(), With<Capturing>>,
	camera: Query<Entity, With<CaptureCamera>>,
) {
	if shots.phase == CapturePhase::Finished || !capturing.is_empty() {
		return;
	}
	let Some(library) = library else {
		return;
	};
	let ready = match shots.phase {
		CapturePhase::OozeBurst => {
			if !shots.requested {
				commands.write_message(DamageApplied {
					target: Entity::PLACEHOLDER,
					source: None,
					amount: 18.0,
					applied: 18.0,
					remaining: 0.0,
					point: Vec3::new(0.0, 1.0, 0.0),
				});
				shots.requested = true;
				shots.shown_at = Some(time.elapsed());
			}
			shots.shown_at.is_some_and(|started| time.elapsed().saturating_sub(started) >= Duration::from_millis(120))
		}
		CapturePhase::SpiritMid => {
			if !shots.requested {
				let palette = ExplosionPalette::celestial();
				commands.spawn_vfx(
					&library.spirit_dissolve.effect_definition(),
					VfxSpawn {
						transform: Transform::from_translation(Vec3::new(0.0, 0.9, 0.0)),
						palette: Some(palette),
						..default()
					},
				);
				shots.requested = true;
				shots.shown_at = Some(time.elapsed());
			}
			shots.shown_at.is_some_and(|started| {
				time.elapsed().saturating_sub(started) >= Duration::from_secs_f32(SPIRIT_DISSOLVE_SECS * 0.45)
			})
		}
		CapturePhase::SpiritDone => {
			if !shots.requested {
				shots.requested = true;
				shots.shown_at = Some(time.elapsed());
			}
			shots.shown_at.is_some_and(|started| {
				time.elapsed().saturating_sub(started) >= Duration::from_secs_f32(SPIRIT_DISSOLVE_SECS + 0.35)
			})
		}
		CapturePhase::Finished => false,
	};
	if !ready {
		return;
	}
	let Ok(camera) = camera.single() else {
		return;
	};
	let path = dir.0.join(format!("{}.png", shots.phase.file_stem()));
	commands
		.spawn(Screenshot::primary_window())
		.observe(save_to_disk(path.clone()));
	commands.entity(camera).insert(Capturing);
	info!(path = %path.display(), "requested combat vfx capture");
}

fn advance_shots(
	mut commands: Commands,
	mut shots: ResMut<CaptureShots>,
	capturing: Query<Entity, With<Capturing>>,
) {
	if capturing.is_empty() || !shots.requested {
		return;
	}
	for entity in &capturing {
		commands.entity(entity).remove::<Capturing>();
	}
	shots.phase = shots.phase.next();
	shots.requested = false;
	shots.shown_at = None;
	if shots.phase == CapturePhase::Finished {
		commands.write_message(AppExit::Success);
	}
}
