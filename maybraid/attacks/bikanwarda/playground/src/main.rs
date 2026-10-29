//! Repeatable Parade / God Beam / Bray preview.
//!
//! Gameplay is `bikanwarda-attacks` + `attacks`. Presentation is placeholders
//! plus existing `fiery_explosion`. Self / `maybraid-world` are not loaded.
//!
//! `bikanwarda-attacks-playground --capture` walks each move, then cancels
//! God Beam so xvfb can GPU-readback stills.

use std::path::PathBuf;

use attacks::{
	AttackEnded, AttackId, AttackInstance, AttackPlaceholderPresentationPlugin, AttackStage,
	AttackUser, AttacksPlugin, BeginAttack, CancelAttacks,
};
use avian3d::prelude::{Collider, PhysicsPlugins, Sensor};
use bevy::app::AppExit;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Capturing, Screenshot};
use bikanwarda_attacks::{
	BikanwardaAttackParams, BikanwardaAttacksPlugin, BikanwardaEyeSocket, BikanwardaHeadSocket,
	BikanwardaMouthSocket, ParadeExplosionCue, BRAY, GOD_BEAM, PARADE,
};
use combat_targeting::{CombatContact, CombatTargeting};
use damage::{apply_hits, DamageApplied, Died, Health};
use lod_avian::PhysicsInteractionLayer;
use maybraid_vfx::{SpawnVfxExt, VfxLibrary, VfxPlugin, VfxSpawn};

fn main() {
	let capture = std::env::args().any(|arg| arg == "--capture");
	App::new()
		.add_plugins(DefaultPlugins.set(WindowPlugin {
			primary_window: Some(Window {
				title: "Bikanwarda Attacks".into(),
				resolution: (1280, 720).into(),
				decorations: !capture,
				resizable: false,
				..default()
			}),
			..default()
		}))
		.insert_resource(ClearColor(Color::srgb(0.06, 0.07, 0.09)))
		.insert_resource(if capture { PreviewMode::Capture } else { PreviewMode::Loop })
		.insert_resource(BikanwardaAttackParams::PLACEHOLDER)
		.insert_resource(PreviewStats::default())
		.insert_resource(CaptureIo::from_env())
		.add_plugins(PhysicsPlugins::default())
		.add_plugins(AttacksPlugin)
		.add_plugins(BikanwardaAttacksPlugin { placeholders: true })
		.add_plugins(AttackPlaceholderPresentationPlugin)
		.add_plugins(VfxPlugin)
		.add_message::<DamageApplied>()
		.add_message::<Died>()
		.add_systems(PostUpdate, apply_hits)
		.add_systems(Startup, setup)
		.add_systems(
			Update,
			(
				retrigger,
				capture_sequence,
				present_fiery_explosions,
				tint_target,
				update_hud,
				track_hits,
			),
		)
		.run();
}

#[derive(Resource, Clone, Copy, PartialEq, Eq)]
enum PreviewMode {
	Loop,
	Capture,
}

#[derive(Resource)]
struct CaptureIo {
	dir: PathBuf,
}

impl CaptureIo {
	fn from_env() -> Self {
		Self {
			dir: std::env::var("BIKANWARDA_PREVIEW_DIR")
				.map(PathBuf::from)
				.unwrap_or_else(|_| PathBuf::from("maybraid/attacks/bikanwarda/preview")),
		}
	}
}

#[derive(Resource, Default)]
struct PreviewStats {
	hits: u32,
	cancelled: bool,
}

#[derive(Resource)]
struct CapturePlan {
	steps: Vec<CaptureStep>,
	index: usize,
	wait: f32,
	awaiting: Option<PathBuf>,
	requested: bool,
}

#[derive(Clone, Copy)]
enum CaptureStep {
	Wait(f32),
	Aim(CaptureCamera),
	Cast(AttackId),
	Shot(&'static str),
	Cancel,
	Exit,
}

#[derive(Clone, Copy)]
enum CaptureCamera {
	Parade,
	Beam,
	Bray,
}

#[derive(Component)]
struct PreviewCaster {
	next: AttackId,
	wait: f32,
}

#[derive(Component)]
struct PreviewTarget(Entity);

#[derive(Component)]
struct PreviewSocket(Entity);

#[derive(Component)]
struct PreviewHud;

#[derive(Component)]
struct TargetBody;

#[derive(Component)]
struct PreviewCamera;

fn setup(
	mut commands: Commands,
	mode: Res<PreviewMode>,
	io: Res<CaptureIo>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let _ = std::fs::create_dir_all(&io.dir);
	commands.spawn((
		PreviewCamera,
		Camera3d::default(),
		AmbientLight { brightness: 380.0, ..default() },
		camera_for(CaptureCamera::Parade),
	));
	commands.spawn((
		DirectionalLight { illuminance: 9_000.0, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.85, 0.55, 0.0)),
	));
	commands.spawn((
		Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(40.0)))),
		MeshMaterial3d(materials.add(Color::srgb(0.14, 0.16, 0.18))),
		Transform::from_xyz(0.0, 0.0, 0.0),
	));

	let body = materials.add(Color::srgb(0.32, 0.34, 0.38));
	let accent = materials.add(Color::srgb(0.82, 0.52, 0.36));
	let owner = commands
		.spawn((
			Name::new("bikanwarda-stand-in"),
			AttackUser::default(),
			PreviewCaster {
				next: PARADE,
				wait: if *mode == PreviewMode::Capture { 1000.0 } else { 0.5 },
			},
			Mesh3d(meshes.add(Cuboid::new(5.5, 12.0, 8.0))),
			MeshMaterial3d(body.clone()),
			Transform::from_xyz(0.0, 6.0, 12.0),
		))
		.id();
	commands.spawn((
		Name::new("head"),
		BikanwardaHeadSocket,
		ChildOf(owner),
		Mesh3d(meshes.add(Cuboid::new(3.2, 2.6, 3.6))),
		MeshMaterial3d(body),
		Transform::from_xyz(0.0, 7.4, -2.6),
	));
	let mouth = commands
		.spawn((
			Name::new("mouth"),
			BikanwardaMouthSocket,
			ChildOf(owner),
			Mesh3d(meshes.add(Sphere::new(0.38))),
			MeshMaterial3d(accent.clone()),
			Transform::from_xyz(0.0, 6.6, -4.4),
		))
		.id();
	for x in [-0.55, 0.55] {
		commands.spawn((
			Name::new("eye"),
			BikanwardaEyeSocket,
			ChildOf(owner),
			Mesh3d(meshes.add(Sphere::new(0.22))),
			MeshMaterial3d(accent.clone()),
			Transform::from_xyz(x, 7.5, -4.2),
		));
	}

	let target = commands
		.spawn((
			Name::new("target"),
			TargetBody,
			Health::from_max(800.0),
			Mesh3d(meshes.add(Capsule3d::new(0.45, 1.3))),
			MeshMaterial3d(materials.add(Color::srgb(0.55, 0.72, 0.84))),
			Transform::from_xyz(0.0, 1.1, -8.0),
			Collider::capsule(0.45, 1.3),
			Sensor,
			PhysicsInteractionLayer::animated_layers(),
		))
		.id();
	let mut targeting = CombatTargeting::default();
	targeting.upsert_contact(CombatContact {
		subject: target,
		position: Vec3::new(0.0, 0.0, -8.0),
		movement_vector: Vec3::ZERO,
		visible_point: Vec3::new(0.0, 0.0, -8.0),
		visible_head: Some(Vec3::new(0.0, 1.8, -8.0)),
		last_spotted_at: 0.0,
	});
	targeting.rebalance(0.0);
	commands
		.entity(owner)
		.insert((PreviewTarget(target), PreviewSocket(mouth), targeting));

	if *mode != PreviewMode::Capture {
		commands.spawn((
			PreviewHud,
			Text::new(hud_text("idle", 800.0, 0, false)),
			TextFont { font_size: FontSize::Px(24.0), ..default() },
			TextColor(Color::srgb(0.95, 0.93, 0.86)),
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(16.0),
				left: Val::Px(16.0),
				..default()
			},
		));
	}

	if *mode == PreviewMode::Capture {
		commands.insert_resource(CapturePlan {
			steps: vec![
				CaptureStep::Wait(3.2),
				CaptureStep::Aim(CaptureCamera::Parade),
				CaptureStep::Cast(PARADE),
				CaptureStep::Wait(0.95),
				CaptureStep::Shot("parade-windup.png"),
				CaptureStep::Wait(1.2),
				CaptureStep::Shot("parade-execute.png"),
				CaptureStep::Cancel,
				CaptureStep::Wait(0.7),
				CaptureStep::Aim(CaptureCamera::Beam),
				CaptureStep::Cast(GOD_BEAM),
				CaptureStep::Wait(1.0),
				CaptureStep::Shot("god-beam-windup.png"),
				CaptureStep::Wait(1.0),
				CaptureStep::Shot("god-beam-execute.png"),
				CaptureStep::Cancel,
				CaptureStep::Wait(0.7),
				CaptureStep::Aim(CaptureCamera::Bray),
				CaptureStep::Cast(BRAY),
				CaptureStep::Wait(0.55),
				CaptureStep::Shot("bray-windup.png"),
				CaptureStep::Wait(1.15),
				CaptureStep::Shot("bray-execute.png"),
				CaptureStep::Cancel,
				CaptureStep::Wait(0.8),
				CaptureStep::Shot("cancelled.png"),
				CaptureStep::Exit,
			],
			index: 0,
			wait: 0.0,
			awaiting: None,
			requested: false,
		});
	}
}

fn camera_for(kind: CaptureCamera) -> Transform {
	match kind {
		CaptureCamera::Parade => {
			Transform::from_xyz(28.0, 12.0, 4.0).looking_at(Vec3::new(0.0, 3.0, 2.0), Vec3::Y)
		}
		CaptureCamera::Beam => {
			Transform::from_xyz(24.0, 11.0, 10.0).looking_at(Vec3::new(0.0, 5.0, 0.0), Vec3::Y)
		}
		CaptureCamera::Bray => {
			Transform::from_xyz(16.0, 12.4, 8.0).looking_at(Vec3::new(0.0, 12.4, 4.0), Vec3::Y)
		}
	}
}

fn retrigger(
	mode: Res<PreviewMode>,
	time: Res<Time>,
	mut casters: Query<(Entity, &mut PreviewCaster, &PreviewTarget, &PreviewSocket, &AttackUser)>,
	mut begins: MessageWriter<BeginAttack>,
) {
	if *mode == PreviewMode::Capture {
		return;
	}
	let dt = time.delta_secs();
	for (owner, mut caster, target, socket, user) in &mut casters {
		caster.wait -= dt;
		if caster.wait > 0.0 || !user.ready(caster.next) {
			continue;
		}
		begins.write(
			BeginAttack::new(owner, caster.next)
				.with_target(target.0)
				.with_emission_socket(socket.0),
		);
		caster.next = match caster.next {
			id if id == PARADE => GOD_BEAM,
			id if id == GOD_BEAM => BRAY,
			_ => PARADE,
		};
		caster.wait = 1.4;
	}
}

fn present_fiery_explosions(
	mut cues: MessageReader<ParadeExplosionCue>,
	library: Option<Res<VfxLibrary>>,
	mut commands: Commands,
) {
	let Some(library) = library else {
		return;
	};
	for cue in cues.read() {
		commands.spawn_vfx(
			&library.fiery_explosion,
			VfxSpawn {
				transform: Transform::from_translation(cue.position),
				scale: (cue.radius / 1.2).clamp(1.0, 8.0),
				intensity: 1.0,
				playback: 1.0,
				..default()
			},
		);
	}
}

#[allow(clippy::too_many_arguments)]
fn capture_sequence(
	mode: Res<PreviewMode>,
	time: Res<Time>,
	io: Res<CaptureIo>,
	mut cameras: Query<&mut Transform, With<PreviewCamera>>,
	casters: Query<(Entity, &PreviewTarget, &PreviewSocket), With<PreviewCaster>>,
	mut plan: Option<ResMut<CapturePlan>>,
	mut begins: MessageWriter<BeginAttack>,
	mut cancels: MessageWriter<CancelAttacks>,
	mut commands: Commands,
	mut exit: MessageWriter<AppExit>,
	capturing: Query<(), With<Capturing>>,
) {
	if *mode != PreviewMode::Capture {
		return;
	}
	let Some(plan) = plan.as_mut() else {
		return;
	};
	if let Some(path) = plan.awaiting.clone() {
		if !capturing.is_empty() {
			return;
		}
		if plan.requested && path.is_file() {
			plan.awaiting = None;
			plan.requested = false;
			plan.index += 1;
		} else if !plan.requested {
			let _ = std::fs::remove_file(&path);
			request_shot(&mut commands, path);
			plan.requested = true;
			return;
		} else {
			return;
		}
	}
	let Some(step) = plan.steps.get(plan.index).copied() else {
		exit.write(AppExit::Success);
		return;
	};
	match step {
		CaptureStep::Wait(secs) => {
			plan.wait += time.delta_secs();
			if plan.wait >= secs {
				plan.wait = 0.0;
				plan.index += 1;
			}
		}
		CaptureStep::Aim(camera) => {
			if let Ok(mut transform) = cameras.single_mut() {
				*transform = camera_for(camera);
			}
			plan.index += 1;
		}
		CaptureStep::Cast(id) => {
			if let Ok((owner, target, socket)) = casters.single() {
				begins.write(
					BeginAttack::new(owner, id)
						.with_target(target.0)
						.with_emission_socket(socket.0),
				);
			}
			plan.index += 1;
		}
		CaptureStep::Shot(name) => {
			plan.awaiting = Some(io.dir.join(name));
		}
		CaptureStep::Cancel => {
			if let Ok((owner, _, _)) = casters.single() {
				cancels.write(CancelAttacks::all(owner));
			}
			plan.index += 1;
		}
		CaptureStep::Exit => {
			if capturing.is_empty() {
				exit.write(AppExit::Success);
			}
		}
	}
}

fn request_shot(commands: &mut Commands, path: impl Into<PathBuf>) {
	commands.spawn(Screenshot::primary_window()).observe(save_to_disk(path.into()));
}

fn track_hits(
	mut applied: MessageReader<DamageApplied>,
	mut ended: MessageReader<AttackEnded>,
	mut stats: ResMut<PreviewStats>,
) {
	stats.hits += applied.read().count() as u32;
	if ended.read().any(|event| event.reason == attacks::AttackEndReason::Cancelled) {
		stats.cancelled = true;
	}
}

fn tint_target(
	health: Query<&Health, With<TargetBody>>,
	targets: Query<&MeshMaterial3d<StandardMaterial>, With<TargetBody>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let Ok(pool) = health.single() else {
		return;
	};
	let Ok(handle) = targets.single() else {
		return;
	};
	let Some(mut material) = materials.get_mut(&handle.0) else {
		return;
	};
	let hurt = 1.0 - pool.fraction();
	material.base_color = Color::srgb(0.55 + 0.4 * hurt, 0.72 - 0.5 * hurt, 0.84 - 0.6 * hurt);
}

fn update_hud(
	stats: Res<PreviewStats>,
	instances: Query<&AttackInstance>,
	health: Query<&Health, With<TargetBody>>,
	mut hud: Query<&mut Text, With<PreviewHud>>,
) {
	let Ok(mut text) = hud.single_mut() else {
		return;
	};
	let stage = match instances.iter().next().map(|instance| instance.stage) {
		Some(AttackStage::Windup) => "windup",
		Some(AttackStage::Execute) => "execute",
		Some(AttackStage::Recover) => "recover",
		None => "idle",
	};
	let hp = health.single().map(|pool| pool.current).unwrap_or(0.0);
	text.0 = hud_text(stage, hp, stats.hits, stats.cancelled);
}

fn hud_text(stage: &str, hp: f32, hits: u32, cancelled: bool) -> String {
	format!(
		"Bikanwarda attacks\nstage: {stage}\ntarget HP: {hp:.0} / 800\nhits: {hits}{}",
		if cancelled { "\ncancelled" } else { "" }
	)
}
