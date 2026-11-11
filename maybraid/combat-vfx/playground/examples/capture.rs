//! Headless stills: braidman spiritual spawn-in and dissolve in two tint palettes.
//!
//! ```bash
//! COMBAT_VFX_CAPTURE_DIR=/tmp/combat-vfx \
//!   VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
//!   WGPU_BACKEND=vk \
//!   WINIT_UNIX_BACKEND=x11 WAYLAND_DISPLAY= \
//!   xvfb-run -a cargo run -p combat-vfx-playground --example capture
//! ```

use std::path::{Path, PathBuf};
use std::time::Duration;

use bevy::audio::AudioPlugin;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use character_rigs::Side;
use characters::CharacterRig;
use characters_playground::{
	camera::orient_camera, AnimationMode, CameraController, CharacterConfig, CharacterSpecies,
	CharactersPlaygroundPlugin, PlaygroundCharacter,
};
use combat_vfx::{CombatVfxPlugin, SpiritTint, DEFAULT_SPIRIT_TINT, SPIRIT_DISSOLVE_SECS, SPIRIT_SPAWN_SECS};
use maybraid_vfx::SPIRIT_SPAWN;
use damage::{DamagePlugin, Health};
use game_commands::ui::GameCommandDrawerVisible;
use maybraid_vfx::{EffectRecipe, ExplosionPalette, SpawnVfxExt, VfxInstance, VfxLibrary, VfxSpawn, VfxPlugin};
use player::Npc;

const TIMEOUT: Duration = Duration::from_secs(120);
const ASSEMBLY_SETTLE: Duration = Duration::from_secs(8);

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
						resolution: (1280u32, 960u32).into(),
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
				})
				.disable::<AudioPlugin>(),
		)
		.add_plugins((DamagePlugin, VfxPlugin, CombatVfxPlugin, CharactersPlaygroundPlugin))
		.insert_resource(ClearColor(Color::srgb(0.14, 0.16, 0.20)))
		.insert_resource(CaptureDir(dir))
		.init_resource::<CapturePlan>()
		.insert_resource(CharacterConfig {
			species: CharacterSpecies::Braidman,
			animation: AnimationMode::Still,
			side: Side::Right,
			transform: Transform::from_xyz(0.0, 0.92, 0.0),
		})
		.add_systems(Startup, (hide_command_ui, add_ground))
		.add_systems(Update, (tag_combat_subject, frame_capture_camera, advance_capture).chain())
		.run();
}

#[derive(Resource)]
struct CaptureDir(PathBuf);

#[derive(Clone)]
struct Shot {
	file: PathBuf,
	kind: ShotKind,
	tint: Color,
}

#[derive(Clone, Copy, Debug)]
enum ShotKind {
	SpiritSpawnStart,
	SpiritSpawnMid,
	SpiritSpawnComplete,
	SpiritSettle,
	SpiritMid,
	SpiritComplete,
}

#[derive(Resource, Default)]
struct CapturePlan {
	shots: Vec<Shot>,
	index: usize,
	scene_ready: bool,
	requested: bool,
	hold_frames: u8,
	started: Duration,
	phase_started: Duration,
	hit_fired: bool,
	spawn_in_fired: bool,
	spawn_revealed: bool,
	spirit_spawned: bool,
	character_hidden: bool,
}

const ROSE_SPIRIT_TINT: Color = Color::srgb(1.0, 0.22, 0.55);

fn spirit_palette(tint: Color) -> ExplosionPalette {
	ExplosionPalette::spirit_burst(tint)
}

impl CapturePlan {
	fn bootstrap(dir: &Path) -> Self {
		let sequences = [
			("cyan", DEFAULT_SPIRIT_TINT),
			("magenta", ROSE_SPIRIT_TINT),
		];
		let mut shots = Vec::new();
		for (label, tint) in sequences {
			for (stem, kind) in [
				("spawn_start", ShotKind::SpiritSpawnStart),
				("spawn_mid", ShotKind::SpiritSpawnMid),
				("spawn_complete", ShotKind::SpiritSpawnComplete),
				("settle", ShotKind::SpiritSettle),
				("dissolve_mid", ShotKind::SpiritMid),
				("dissolve_complete", ShotKind::SpiritComplete),
			] {
				shots.push(Shot {
					file: dir.join(format!("spirit_{label}_{stem}.png")),
					kind,
					tint,
				});
			}
		}
		Self { shots, ..default() }
	}
}

fn hide_command_ui(mut visible: ResMut<GameCommandDrawerVisible>) {
	visible.0 = false;
}

fn add_ground(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let mesh = meshes.add(Plane3d::default().mesh().size(12.0, 12.0));
	let material = materials.add(StandardMaterial {
		base_color: Color::srgb(0.22, 0.23, 0.26),
		perceptual_roughness: 0.92,
		..default()
	});
	commands.spawn((Mesh3d(mesh), MeshMaterial3d(material), Transform::IDENTITY));
}

fn tag_combat_subject(
	mut commands: Commands,
	pending: Query<Entity, (With<PlaygroundCharacter>, Without<Health>)>,
) {
	for entity in &pending {
		commands
			.entity(entity)
			.insert((Npc, Health::from_max(48.0), CombatSubject, SpiritTint::default()));
	}
}

#[derive(Component)]
struct CombatSubject;

fn character_ready(has_library: bool, has_subject: bool, rig_count: usize, mesh_count: usize) -> bool {
	has_library && has_subject && rig_count > 0 && mesh_count > 6
}

/// Tighten framing on the torso so impact particles fill the still.
fn frame_capture_camera(
	plan: Res<CapturePlan>,
	subject: Query<&GlobalTransform, With<CombatSubject>>,
	mut cameras: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	if plan.index >= plan.shots.len() || plan.shots.is_empty() {
		return;
	}
	let Ok(subject) = subject.single() else {
		return;
	};
	let Ok((mut transform, mut controller)) = cameras.single_mut() else {
		return;
	};
	let focus = subject.translation() + Vec3::new(0.0, 0.45, 0.0);
	let offset = Vec3::new(0.4, 0.22, 1.55);
	orient_camera(&mut transform, &mut controller, focus + offset, focus);
}

fn active_spirit(instances: &Query<(Entity, &VfxInstance)>) -> Option<(Entity, f32)> {
	instances
		.iter()
		.filter(|(_, instance)| instance.name.contains("dissolve") && instance.armed)
		.map(|(entity, instance)| (entity, instance.age))
		.next()
}

fn active_spirit_spawn(instances: &Query<(Entity, &VfxInstance)>) -> Option<(Entity, f32)> {
	instances
		.iter()
		.filter(|(_, instance)| instance.name == SPIRIT_SPAWN && instance.armed)
		.map(|(entity, instance)| (entity, instance.age))
		.next()
}

fn spirit_spawn_age_in_window(
	instances: &Query<(Entity, &VfxInstance)>,
	min_age: f32,
	max_age: f32,
) -> bool {
	active_spirit_spawn(instances).is_some_and(|(_, age)| age >= min_age && age <= max_age)
}

fn advance_capture(
	time: Res<Time>,
	mut commands: Commands,
	mut plan: ResMut<CapturePlan>,
	dir: Res<CaptureDir>,
	library: Option<Res<VfxLibrary>>,
	meshes: Query<(), With<Mesh3d>>,
	rigs: Query<(), With<CharacterRig>>,
	subject: Query<(Entity, &GlobalTransform), With<CombatSubject>>,
	roots: Query<Entity, With<PlaygroundCharacter>>,
	instances: Query<(Entity, &VfxInstance)>,
	names: Query<&Name>,
	capturing: Query<(), With<Capturing>>,
	mut exit: MessageWriter<AppExit>,
) {
	if plan.shots.is_empty() {
		*plan = CapturePlan::bootstrap(&dir.0);
		plan.started = time.elapsed();
		return;
	}
	if plan.index >= plan.shots.len() {
		for root in &roots {
			commands.entity(root).try_despawn();
		}
		exit.write(AppExit::Success);
		return;
	}
	if time.elapsed().saturating_sub(plan.started) > TIMEOUT {
		error!("capture timed out on {}", plan.shots[plan.index].file.display());
		exit.write(AppExit::error());
		return;
	}
	if !capturing.is_empty() {
		return;
	}

	if plan.scene_ready && plan.requested && plan.shots[plan.index].file.is_file() {
		info!("captured {}", plan.shots[plan.index].file.display());
		plan.index += 1;
		plan.requested = false;
		plan.hold_frames = 0;
		let prev = plan.shots.get(plan.index.saturating_sub(1));
		if matches!(prev.map(|s| s.kind), Some(ShotKind::SpiritSpawnComplete)) {
			plan.spawn_in_fired = false;
			plan.spawn_revealed = false;
			plan.character_hidden = false;
			if let Ok(root) = roots.single() {
				commands.entity(root).insert(Visibility::Inherited);
			}
		}
		if matches!(
			plan.shots.get(plan.index).map(|s| s.kind),
			Some(ShotKind::SpiritSpawnStart)
		) {
			plan.spawn_in_fired = false;
			plan.spawn_revealed = false;
			plan.spirit_spawned = false;
			plan.character_hidden = false;
			plan.hit_fired = false;
			if let Ok(root) = roots.single() {
				commands.entity(root).insert(Visibility::Inherited);
			}
		}
		plan.spirit_spawned = false;
		plan.phase_started = time.elapsed();
		return;
	}

	if !plan.scene_ready {
		if !character_ready(
			library.is_some(),
			!subject.is_empty(),
			rigs.iter().len(),
			meshes.iter().count(),
		) {
			return;
		}
		if time.elapsed().saturating_sub(plan.started) < ASSEMBLY_SETTLE {
			return;
		}
		plan.scene_ready = true;
		plan.phase_started = time.elapsed();
	}

	let shot = plan.shots[plan.index].clone();
	let ready = match shot.kind {
		ShotKind::SpiritSpawnStart => {
			if !plan.spawn_in_fired {
				let Ok((body, transform)) = subject.single() else {
					return;
				};
				commands.entity(body).insert(SpiritTint(shot.tint));
				if let Ok(root) = roots.single() {
					commands.entity(root).insert(Visibility::Hidden);
				}
				commands.spawn_vfx(
					&library.as_ref().expect("library").spirit_spawn.effect_definition(),
					VfxSpawn {
						transform: Transform::from_translation(
							transform.translation() + Vec3::new(0.0, 0.35, 0.0),
						),
						scale: 1.35,
						intensity: 1.65,
						palette: Some(spirit_palette(shot.tint)),
						..default()
					},
				);
				plan.spawn_in_fired = true;
				plan.phase_started = time.elapsed();
				return;
			}
			plan.spawn_in_fired
				&& (spirit_spawn_age_in_window(&instances, 0.06, 0.32)
					|| time.elapsed().saturating_sub(plan.phase_started)
						>= Duration::from_millis(320))
		}
		ShotKind::SpiritSpawnMid => {
			plan.spawn_in_fired
				&& !plan.spawn_revealed
				&& (spirit_spawn_age_in_window(&instances, 0.45, 1.25)
					|| active_spirit_spawn(&instances)
						.is_some_and(|(_, age)| age >= 0.55)
						&& time.elapsed().saturating_sub(plan.phase_started)
							>= Duration::from_millis(180))
		}
		ShotKind::SpiritSpawnComplete => {
			if !plan.spawn_revealed {
				if let Ok(root) = roots.single() {
					commands.entity(root).insert(Visibility::Inherited);
				}
				plan.spawn_revealed = true;
				plan.phase_started = time.elapsed();
			}
			let wisps_done = instances
				.iter()
				.all(|(_, i)| i.name != SPIRIT_SPAWN)
				|| active_spirit_spawn(&instances)
					.is_some_and(|(_, age)| age >= SPIRIT_SPAWN_SECS * 0.85)
				|| time.elapsed().saturating_sub(plan.phase_started) >= Duration::from_millis(1400);
			plan.spawn_revealed
				&& wisps_done
				&& time.elapsed().saturating_sub(plan.phase_started) >= Duration::from_millis(200)
		}
		ShotKind::SpiritSettle => {
			// Clear prior spirit instances so the dissolve reads cleanly.
			for (entity, _) in &instances {
				commands.entity(entity).try_despawn();
			}
			time.elapsed().saturating_sub(plan.phase_started) >= Duration::from_millis(200)
		}
		ShotKind::SpiritMid => {
			if !plan.spirit_spawned {
				let Ok((body, transform)) = subject.single() else {
					return;
				};
				commands.entity(body).insert(SpiritTint(shot.tint));
				commands.spawn_vfx(
					&library.as_ref().expect("library").spirit_dissolve.effect_definition(),
					VfxSpawn {
						transform: Transform::from_translation(
							transform.translation() + Vec3::new(0.0, 0.35, 0.0),
						),
						scale: 1.35,
						intensity: 1.65,
						palette: Some(spirit_palette(shot.tint)),
						..default()
					},
				);
				plan.spirit_spawned = true;
				plan.phase_started = time.elapsed();
				return;
			}
			active_spirit(&instances)
				.is_some_and(|(_, age)| age >= SPIRIT_DISSOLVE_SECS * 0.38)
		}
		ShotKind::SpiritComplete => {
			if !plan.character_hidden {
				if let Ok(root) = roots.single() {
					commands.entity(root).insert(Visibility::Hidden);
					plan.character_hidden = true;
					plan.phase_started = time.elapsed();
				}
			}
			let wisps_done = instances.is_empty()
				|| active_spirit(&instances)
					.is_some_and(|(_, age)| age >= SPIRIT_DISSOLVE_SECS * 0.92);
			plan.character_hidden
				&& wisps_done
				&& time.elapsed().saturating_sub(plan.phase_started) >= Duration::from_millis(180)
		}
	};

	if !ready || plan.requested {
		return;
	}
	if plan.hold_frames < 2 {
		plan.hold_frames += 1;
		return;
	}

	commands
		.spawn(Screenshot::primary_window())
		.observe(save_to_disk(shot.file.clone()));
	plan.requested = true;
	let instance_snapshot: Vec<_> = instances
		.iter()
		.map(|(_, instance)| format!("{} armed={} age={:.3}", instance.name, instance.armed, instance.age))
		.collect();
	let vfx_layers = names
		.iter()
		.filter(|name| name.as_str().starts_with("vfx-layer-"))
		.map(|name| name.as_str().to_string())
		.collect::<Vec<_>>();
	info!(
		path = %shot.file.display(),
		?shot.kind,
		vfx_layers = ?vfx_layers,
		instances = ?instance_snapshot,
		"requested combat vfx capture"
	);
}
