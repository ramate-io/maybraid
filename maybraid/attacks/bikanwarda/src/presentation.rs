//! Placeholder sockets and God Beam / Bray body cues.
//!
//! No final GLB is loaded here. Documented socket names live in
//! [`crate::ASSETS_README`]. VFX lanes replace these meshes; gameplay stays
//! on [`attacks::AttackPresentationEvent`].

use attacks::{AttackInstance, AttackStage, AttackSystems};
use bevy::prelude::*;

use crate::{BikanwardaAttackParams, EyeGlowCue, BRAY};

/// Mouth / wave emission. Encounter scripts pass this as `BeginAttack::emission_socket`.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BikanwardaMouthSocket;

/// Eye locator for God Beam glow. Missing sockets fall back to owner + offset.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BikanwardaEyeSocket;

/// Head locator for the Bray flick. Missing sockets skip the pose drive.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BikanwardaHeadSocket;

#[derive(Component, Clone, Copy, Debug)]
struct EyeGlowVisual {
	instance: attacks::AttackInstanceId,
}

#[derive(Component, Clone, Copy, Debug, Default)]
struct HeadRestPose(Quat);

pub fn placeholder_presentation_plugin(app: &mut App) {
	app.add_systems(
		Update,
		(present_eye_glow, drive_head_flick, restore_head_on_end)
			.chain()
			.after(AttackSystems::Advance),
	);
}

fn present_eye_glow(
	mut cues: MessageReader<EyeGlowCue>,
	eyes: Query<(Entity, &ChildOf), With<BikanwardaEyeSocket>>,
	existing: Query<(Entity, &EyeGlowVisual)>,
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	for cue in cues.read() {
		if !cue.active {
			for (entity, visual) in &existing {
				if visual.instance == cue.instance {
					commands.entity(entity).try_despawn();
				}
			}
			continue;
		}
		if existing.iter().any(|(_, visual)| visual.instance == cue.instance) {
			continue;
		}
		let mut spawned = 0;
		for (socket, parent) in &eyes {
			if parent.parent() != cue.owner {
				continue;
			}
			spawn_glow(&mut commands, &mut meshes, &mut materials, cue, socket, Vec3::ZERO);
			spawned += 1;
		}
		if spawned == 0 {
			spawn_glow(
				&mut commands,
				&mut meshes,
				&mut materials,
				cue,
				cue.owner,
				Vec3::new(-0.35, 1.6, -0.4),
			);
			spawn_glow(
				&mut commands,
				&mut meshes,
				&mut materials,
				cue,
				cue.owner,
				Vec3::new(0.35, 1.6, -0.4),
			);
		}
	}
}

fn spawn_glow(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	cue: &EyeGlowCue,
	parent: Entity,
	local: Vec3,
) {
	let mesh = meshes.add(Sphere::new(0.18));
	let material = materials.add(StandardMaterial {
		base_color: Color::srgb(1.0, 0.35, 0.12),
		emissive: LinearRgba::rgb(8.0, 1.6, 0.3),
		unlit: true,
		..default()
	});
	commands.spawn((
		Name::new("bikanwarda-eye-glow"),
		EyeGlowVisual { instance: cue.instance },
		ChildOf(parent),
		Transform::from_translation(local),
		Mesh3d(mesh),
		MeshMaterial3d(material),
		PointLight {
			color: Color::srgb(1.0, 0.45, 0.15),
			intensity: 40_000.0,
			range: 8.0,
			shadow_maps_enabled: false,
			..default()
		},
	));
}

fn drive_head_flick(
	params: Res<BikanwardaAttackParams>,
	instances: Query<&AttackInstance>,
	mut heads: Query<
		(Entity, &ChildOf, &mut Transform, Option<&HeadRestPose>),
		With<BikanwardaHeadSocket>,
	>,
	mut commands: Commands,
) {
	for (entity, parent, mut transform, rest) in &mut heads {
		let Some(instance) = instances.iter().find(|instance| {
			instance.owner == parent.parent()
				&& instance.definition == BRAY
				&& instance.stage == AttackStage::Windup
		}) else {
			continue;
		};
		if rest.is_none() {
			commands.entity(entity).insert(HeadRestPose(transform.rotation));
		}
		let rest_rotation = rest.map(|pose| pose.0).unwrap_or(transform.rotation);
		let t = (instance.stage_age / params.bray.windup.max(1e-3)).clamp(0.0, 1.0);
		let envelope = (t * std::f32::consts::PI).sin();
		transform.rotation =
			rest_rotation * Quat::from_rotation_x(-params.bray.head_flick_pitch * envelope);
	}
}

fn restore_head_on_end(
	instances: Query<&AttackInstance>,
	mut heads: Query<(Entity, &ChildOf, &mut Transform, &HeadRestPose), With<BikanwardaHeadSocket>>,
	mut commands: Commands,
) {
	for (entity, parent, mut transform, rest) in &mut heads {
		let live = instances.iter().any(|instance| {
			instance.owner == parent.parent()
				&& instance.definition == BRAY
				&& instance.stage == AttackStage::Windup
		});
		if live {
			continue;
		}
		transform.rotation = rest.0;
		commands.entity(entity).remove::<HeadRestPose>();
	}
}
