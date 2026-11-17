//! Headless stills: braidman spiritual spawn-in, dissolve, and ooze in two tints.
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
use combat_vfx::{
	CombatVfxPlugin, SpiritTint, DEFAULT_OOZE_TINT, DEFAULT_SPIRIT_TINT, SPIRIT_SPAWN_REVEAL,
};
use damage::{DamagePlugin, Health};
use game_commands::ui::GameCommandDrawerVisible;
use maybraid_vfx::{
	EffectRecipe, ExplosionPalette, SpawnVfxExt, VfxInstance, VfxLibrary, VfxSpawn, VfxPlugin,
	OOZE_BURST, SPIRIT_DISSOLVE, SPIRIT_SPAWN,
};
use player::Npc;

const TIMEOUT: Duration = Duration::from_secs(180);
const ASSEMBLY_SETTLE: Duration = Duration::from_secs(8);
const AGE_SLACK: f32 = 0.35;

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
		.insert_resource(ClearColor(Color::srgb(0.07, 0.08, 0.11)))
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
	SpiritSpawn { at: f32 },
	SpiritDissolve { at: f32 },
	Ooze { at: f32 },
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
	spawn_fired: bool,
	dissolve_fired: bool,
	ooze_fired: bool,
}

const ROSE_SPIRIT_TINT: Color = Color::srgb(1.0, 0.22, 0.55);

impl CapturePlan {
	fn bootstrap(dir: &Path) -> Self {
		let sequences = [("cyan", DEFAULT_SPIRIT_TINT), ("magenta", ROSE_SPIRIT_TINT)];
		let mut shots = Vec::new();
		for (label, tint) in sequences {
			for at in [0.5, 1.5, 2.5, 3.5] {
				shots.push(Shot {
					file: dir.join(format!("spirit_{label}_spawn_{at:.1}s.png")),
					kind: ShotKind::SpiritSpawn { at },
					tint,
				});
			}
			shots.push(Shot {
				file: dir.join(format!("spirit_{label}_dissolve_mid.png")),
				kind: ShotKind::SpiritDissolve { at: 1.4 },
				tint,
			});
			shots.push(Shot {
				file: dir.join(format!("spirit_{label}_ooze_mid.png")),
				kind: ShotKind::Ooze { at: 0.08 },
				tint,
			});
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
		base_color: Color::srgb(0.16, 0.17, 0.19),
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
	let focus = subject.translation() + Vec3::new(0.0, 0.5, 0.0);
	let offset = Vec3::new(0.62, 0.42, 2.15);
	orient_camera(&mut transform, &mut controller, focus + offset, focus);
}

fn named_age(instances: &Query<(Entity, &VfxInstance)>, name: &str) -> Option<f32> {
	instances
		.iter()
		.filter(|(_, instance)| instance.name == name && instance.armed)
		.map(|(_, instance)| instance.age)
		.next()
}

fn age_reached(age: Option<f32>, at: f32) -> bool {
	age.is_some_and(|age| age >= at)
}

fn despawn_vfx(commands: &mut Commands, instances: &Query<(Entity, &VfxInstance)>) {
	for (entity, _) in instances {
		commands.entity(entity).try_despawn();
	}
}

fn set_character_visible(commands: &mut Commands, roots: &Query<Entity, With<PlaygroundCharacter>>, visible: bool) {
	if let Ok(root) = roots.single() {
		commands.entity(root).insert(if visible {
			Visibility::Inherited
		} else {
			Visibility::Hidden
		});
	}
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
		let prev = plan.shots[plan.index].kind;
		plan.index += 1;
		plan.requested = false;
		plan.hold_frames = 0;
		plan.phase_started = time.elapsed();
		if plan.index >= plan.shots.len() {
			return;
		}
		let next = plan.shots[plan.index].kind;
		let switching_kind = std::mem::discriminant(&prev) != std::mem::discriminant(&next);
		if switching_kind || matches!(next, ShotKind::SpiritSpawn { at: 0.5 }) {
			despawn_vfx(&mut commands, &instances);
			plan.spawn_fired = false;
			plan.dissolve_fired = false;
			plan.ooze_fired = false;
			set_character_visible(&mut commands, &roots, true);
		}
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
	let Ok((body, transform)) = subject.single() else {
		return;
	};
	let library = library.as_ref().expect("library");
	let origin = transform.translation() + Vec3::new(0.0, 0.4, 0.0);
	let ready = match shot.kind {
		ShotKind::SpiritSpawn { at } => {
			if !plan.spawn_fired {
				commands.entity(body).insert(SpiritTint(shot.tint));
				set_character_visible(&mut commands, &roots, false);
				commands.spawn_vfx(
					&library.spirit_spawn.effect_definition(),
					VfxSpawn {
						transform: Transform::from_translation(origin),
						scale: 1.0,
						intensity: combat_vfx::spirit_intensity(),
						palette: Some(ExplosionPalette::spirit_burst(shot.tint)),
						..default()
					},
				);
				plan.spawn_fired = true;
				plan.phase_started = time.elapsed();
				return;
			}
			if named_age(&instances, SPIRIT_SPAWN).is_some_and(|age| age >= SPIRIT_SPAWN_REVEAL) {
				set_character_visible(&mut commands, &roots, true);
			}
			age_reached(named_age(&instances, SPIRIT_SPAWN), at)
				|| time.elapsed().saturating_sub(plan.phase_started)
					>= Duration::from_secs_f32(at + AGE_SLACK)
		}
		ShotKind::SpiritDissolve { at } => {
			if !plan.dissolve_fired {
				despawn_vfx(&mut commands, &instances);
				commands.entity(body).insert(SpiritTint(shot.tint));
				set_character_visible(&mut commands, &roots, false);
				commands.spawn_vfx(
					&library.spirit_dissolve.effect_definition(),
					VfxSpawn {
						transform: Transform::from_translation(origin),
						scale: 1.0,
						intensity: combat_vfx::spirit_intensity(),
						palette: Some(ExplosionPalette::spirit_burst(shot.tint)),
						..default()
					},
				);
				plan.dissolve_fired = true;
				plan.phase_started = time.elapsed();
				return;
			}
			age_reached(named_age(&instances, SPIRIT_DISSOLVE), at)
				|| time.elapsed().saturating_sub(plan.phase_started)
					>= Duration::from_secs_f32(at + AGE_SLACK)
		}
		ShotKind::Ooze { at } => {
			if !plan.ooze_fired {
				despawn_vfx(&mut commands, &instances);
				set_character_visible(&mut commands, &roots, true);
				let hit = origin + Vec3::new(0.10, 0.06, 0.08);
				commands.spawn_vfx(
					&library.ooze_burst.effect_definition(),
					VfxSpawn {
						transform: Transform::from_translation(hit)
							.with_rotation(Quat::from_rotation_arc(Vec3::Z, Vec3::new(0.92, 0.18, 0.22).normalize())),
						scale: 1.0,
						intensity: combat_vfx::ooze_intensity(),
						palette: Some(ExplosionPalette::combat_ooze(DEFAULT_OOZE_TINT)),
						..default()
					},
				);
				plan.ooze_fired = true;
				plan.phase_started = time.elapsed();
				return;
			}
			{
				let age = named_age(&instances, OOZE_BURST);
				age.is_some_and(|age| age >= at.max(0.06))
					|| (age.is_some_and(|age| age > 0.04)
						&& time.elapsed().saturating_sub(plan.phase_started)
							>= Duration::from_secs_f32(at + 0.16))
			}
		}
	};

	if !ready || plan.requested {
		return;
	}
	let hold = match shot.kind {
		ShotKind::Ooze { .. } => 1,
		_ => 2,
	};
	if plan.hold_frames < hold {
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
