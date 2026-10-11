//! Heavenly laser column. Presentation only — no hazard or damage.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectKind, ScaleBounds, DEFAULT_SCALE_MIN};
use crate::drive::VfxDrive;
use crate::field_material::{FieldKind, FieldMaterial};
use crate::fields::{column_pose, FieldMaterialSlot, VfxField, VfxSustainLight};
use crate::membership::VfxMemberOf;
use crate::names::GOD_BEAM;
use crate::palette::FieldPalette;
use crate::seed;
use crate::spawn::{VfxInstance, VfxLayerLife, VfxPendingLayers, VfxSpawn};

/// Spatial multiplier cap. World meters live on [`VfxDrive::size`] / [`VfxDrive::reach`].
pub const MAX_SCALE: f32 = 8.0;

const CORE_WIDTH: f32 = 0.42;
const SHEATH_WIDTH: f32 = 1.0;
const CONTACT_WIDTH: f32 = 1.35;
const GROUND_LIGHT_PEAK: f32 = 90_000.0;
const SHAFT_LIGHT_PEAK: f32 = 36_000.0;

/// Compiled definition. Meshes are `[column, disk]`.
pub fn definition(column: Handle<Mesh>, disk: Handle<Mesh>) -> EffectDefinition {
	EffectDefinition::new(GOD_BEAM, [])
		.with_kind(EffectKind::GodBeam)
		.with_duration(VfxDrive::god_beam().total_secs())
		.with_scale_bounds(ScaleBounds { min: DEFAULT_SCALE_MIN, max: MAX_SCALE })
		.with_meshes([column, disk])
}

pub fn spawn(commands: &mut Commands, definition: &EffectDefinition, spawn: VfxSpawn) -> Entity {
	let drive = spawn.drive.resolved_god_beam();
	let duration = drive.total_secs();
	let (root_transform, height) =
		column_pose(spawn.transform.translation, spawn.transform.rotation, &drive);
	let width = drive.size * spawn.scale;
	let height = height * spawn.scale;
	let seed = spawn.resolved_seed();
	let palette = spawn
		.palette
		.map(|palette| palette.as_field())
		.unwrap_or_else(FieldPalette::heaven)
		.with_tint(spawn.tint);
	let column = definition.meshes.first().cloned().unwrap_or_default();
	let disk = definition.meshes.get(1).cloned().unwrap_or_default();

	let root = commands
		.spawn((
			Name::new("vfx-god_beam"),
			root_transform,
			Visibility::Visible,
			VfxInstance {
				name: definition.name.clone(),
				age: 0.0,
				duration,
				playback: spawn.clamped_playback(),
				seed,
				armed: true,
			},
			VfxPendingLayers { layers: Vec::new(), spawn: spawn.clone() },
			NotShadowCaster,
		))
		.id();

	spawn_field(
		commands,
		root,
		FieldPart {
			name: "core",
			mesh: column.clone(),
			kind: FieldKind::ColumnCore,
			transform: Transform::from_scale(Vec3::new(
				width * CORE_WIDTH,
				height,
				width * CORE_WIDTH,
			)),
			layer: seed::LAYER_COLUMN,
			index: 0,
		},
		duration,
		seed,
		&palette,
		&spawn,
	);
	spawn_field(
		commands,
		root,
		FieldPart {
			name: "sheath",
			mesh: column,
			kind: FieldKind::ColumnSheath,
			transform: Transform::from_scale(Vec3::new(
				width * SHEATH_WIDTH,
				height,
				width * SHEATH_WIDTH,
			)),
			layer: seed::LAYER_COLUMN,
			index: 1,
		},
		duration,
		seed,
		&palette,
		&spawn,
	);
	spawn_field(
		commands,
		root,
		FieldPart {
			name: "contact",
			mesh: disk,
			kind: FieldKind::GroundContact,
			transform: Transform::from_scale(Vec3::new(
				width * CONTACT_WIDTH,
				1.0,
				width * CONTACT_WIDTH,
			)),
			layer: seed::LAYER_CONTACT,
			index: 0,
		},
		duration,
		seed,
		&palette,
		&spawn,
	);

	let ground_peak = GROUND_LIGHT_PEAK * spawn.clamped_intensity();
	commands.spawn((
		Name::new("vfx-god_beam-ground-light"),
		ChildOf(root),
		VfxMemberOf(root),
		Transform::from_xyz(0.0, 0.4, 0.0),
		PointLight {
			color: crate::palette::ExplosionPalette::color(palette.flash),
			intensity: 0.0,
			range: (width * 3.2).max(8.0),
			shadow_maps_enabled: false,
			..default()
		},
		VfxSustainLight { peak: ground_peak, playback: spawn.clamped_playback() },
		VfxLayerLife {
			age: 0.0,
			duration,
			playback: spawn.clamped_playback(),
			waiting_for_emitter: false,
		},
	));
	commands.spawn((
		Name::new("vfx-god_beam-shaft-light"),
		ChildOf(root),
		VfxMemberOf(root),
		Transform::from_xyz(0.0, height * 0.32, 0.0),
		PointLight {
			color: crate::palette::ExplosionPalette::color(palette.hot),
			intensity: 0.0,
			range: (width * 2.4 + height * 0.12).max(12.0),
			shadow_maps_enabled: false,
			..default()
		},
		VfxSustainLight {
			peak: SHAFT_LIGHT_PEAK * spawn.clamped_intensity(),
			playback: spawn.clamped_playback(),
		},
		VfxLayerLife {
			age: 0.0,
			duration,
			playback: spawn.clamped_playback(),
			waiting_for_emitter: false,
		},
	));

	root
}

struct FieldPart {
	name: &'static str,
	mesh: Handle<Mesh>,
	kind: FieldKind,
	transform: Transform,
	layer: u32,
	index: u32,
}

fn spawn_field(
	commands: &mut Commands,
	parent: Entity,
	part: FieldPart,
	duration: f32,
	seed: u64,
	palette: &FieldPalette,
	spawn: &VfxSpawn,
) {
	let material = FieldMaterial::new(
		part.kind,
		duration,
		seed::unit(seed::stream(seed, part.layer, part.index)),
		palette,
		spawn.tint_or_white(),
		spawn.clamped_intensity(),
	);
	commands.spawn((
		Name::new(format!("vfx-god_beam-{}", part.name)),
		ChildOf(parent),
		VfxMemberOf(parent),
		Mesh3d(part.mesh),
		part.transform,
		Visibility::Inherited,
		VfxField { kind: part.kind, playback: spawn.clamped_playback() },
		VfxLayerLife {
			age: 0.0,
			duration,
			playback: spawn.clamped_playback(),
			waiting_for_emitter: false,
		},
		NotShadowCaster,
		FieldMaterialSlot(material),
	));
}

/// Independent drive preset. [#1119](https://github.com/ramate-io/maybraid/issues/1119)
/// should prefer [`crate::attack_drive`] when a `#1147` hazard is a cylinder.
///
/// ```ignore
/// let handle = commands.spawn_vfx(
///     &library.god_beam,
///     VfxSpawn {
///         transform: Transform::from_translation(ground),
///         drive: VfxDrive::god_beam()
///             .with_size(7.0) // diameter meters
///             .with_lifetime(10.0)
///             .with_far_point(sky),
///         seed: Some(seed),
///         ..Default::default()
///     },
/// );
/// // On sleep / death / interrupt when not using AttackPresentationBinding:
/// commands.cancel_vfx(handle);
/// ```
pub fn attack_drive_example() -> VfxDrive {
	VfxDrive::god_beam()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::drive::{
		FieldEnvelope, GOD_BEAM_ACTIVE, GOD_BEAM_FADE_IN, GOD_BEAM_FADE_OUT, GOD_BEAM_WIDTH,
	};

	#[test]
	fn definition_is_seven_by_ten() {
		let def = definition(Handle::default(), Handle::default());
		assert_eq!(def.name, GOD_BEAM);
		assert_eq!(def.kind, EffectKind::GodBeam);
		assert!((def.duration() - VfxDrive::god_beam().total_secs()).abs() < 1e-4);
		assert!((VfxDrive::god_beam().size - GOD_BEAM_WIDTH).abs() < 1e-4);
		assert!((VfxDrive::god_beam().active_secs() - GOD_BEAM_ACTIVE).abs() < 1e-4);
		assert_eq!(attack_drive_example().size, GOD_BEAM_WIDTH);
	}

	#[test]
	fn two_instances_keep_independent_seeds_and_timing() -> Result<(), String> {
		let a = VfxSpawn { seed: Some(3), drive: VfxDrive::god_beam(), ..default() }.resolved();
		let b =
			VfxSpawn { seed: Some(4), drive: VfxDrive::god_beam().with_lifetime(4.0), ..default() }
				.resolved();
		if a.seed == b.seed {
			return Err("seeds collided".into());
		}
		if (a.drive.total_secs() - b.drive.total_secs()).abs() < 1.0 {
			return Err("lifetimes should differ".into());
		}
		Ok(())
	}

	#[test]
	fn envelope_covers_ten_second_active_phase() {
		let drive = VfxDrive::god_beam();
		let held = FieldEnvelope::sample(GOD_BEAM_FADE_IN + 5.0, &drive);
		let leaving = FieldEnvelope::sample(
			GOD_BEAM_FADE_IN + GOD_BEAM_ACTIVE + GOD_BEAM_FADE_OUT * 0.5,
			&drive,
		);
		assert!((held.alpha - 1.0).abs() < 1e-4);
		assert!(leaving.alpha > 0.2 && leaving.alpha < 0.8);
	}
}
