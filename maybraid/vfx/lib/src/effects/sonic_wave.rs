//! Mouth-emitted vortex rings. Presentation only — no travel simulation.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectKind, ScaleBounds, DEFAULT_SCALE_MIN};
use crate::drive::{VfxDrive, VfxWaveStates};
use crate::field_material::{FieldKind, FieldMaterial};
use crate::fields::{FieldMaterialSlot, VfxField, VfxSonicRing, VfxSustainLight};
use crate::membership::VfxMemberOf;
use crate::names::SONIC_WAVE;
use crate::palette::FieldPalette;
use crate::seed;
use crate::spawn::{VfxInstance, VfxLayerLife, VfxPendingLayers, VfxSpawn};

pub const MAX_SCALE: f32 = 8.0;
const ORIGIN_LIGHT_PEAK: f32 = 18_000.0;

/// Compiled definition. Meshes are `[vortex ring]`.
pub fn definition(ring: Handle<Mesh>) -> EffectDefinition {
	EffectDefinition::new(SONIC_WAVE, [])
		.with_kind(EffectKind::SonicWave)
		.with_duration(VfxDrive::sonic_wave().total_secs())
		.with_scale_bounds(ScaleBounds { min: DEFAULT_SCALE_MIN, max: MAX_SCALE })
		.with_meshes([ring])
}

pub fn spawn(commands: &mut Commands, definition: &EffectDefinition, spawn: VfxSpawn) -> Entity {
	let drive = spawn.drive.resolved_sonic_wave();
	let duration = drive.total_secs();
	let seed = spawn.resolved_seed();
	let palette = spawn
		.palette
		.map(|palette| palette.as_field())
		.unwrap_or_else(FieldPalette::shock)
		.with_tint(spawn.tint);
	let ring = definition.meshes.first().cloned().unwrap_or_default();

	let root = commands
		.spawn((
			Name::new("vfx-sonic_wave"),
			Transform::from_translation(spawn.transform.translation),
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
			VfxWaveStates::default(),
			NotShadowCaster,
		))
		.id();

	for index in 0..drive.count {
		let material = FieldMaterial::new(
			FieldKind::ShockRing,
			duration,
			seed::unit(seed::stream(seed, seed::LAYER_WAVE, index)),
			&palette,
			spawn.tint_or_white(),
			spawn.clamped_intensity(),
		);
		commands.spawn((
			Name::new(format!("vfx-sonic_wave-ring-{index}")),
			ChildOf(root),
			VfxMemberOf(root),
			Mesh3d(ring.clone()),
			Transform::from_scale(Vec3::ZERO),
			Visibility::Hidden,
			VfxField { kind: FieldKind::ShockRing, playback: spawn.clamped_playback() },
			VfxSonicRing { index },
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

	commands.spawn((
		Name::new("vfx-sonic_wave-origin-light"),
		ChildOf(root),
		VfxMemberOf(root),
		Transform::IDENTITY,
		PointLight {
			color: crate::palette::ExplosionPalette::color(palette.flash),
			intensity: 0.0,
			range: (drive.radius * 8.0).max(4.0),
			shadow_maps_enabled: false,
			..default()
		},
		VfxSustainLight {
			peak: ORIGIN_LIGHT_PEAK * spawn.clamped_intensity(),
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

/// Independent drive preset. [#1119](https://github.com/ramate-io/maybraid/issues/1119)
/// writes [`VfxWaveStates`] from `#1147` `WavefrontPresentation` fields.
///
/// ```ignore
/// let handle = commands.spawn_vfx(
///     &library.sonic_wave,
///     VfxSpawn {
///         transform: Transform::from_translation(mouth),
///         drive: VfxDrive::sonic_wave()
///             .with_size(wave.range)
///             .with_radius(wave.radius)
///             .with_thickness(wave.thickness)
///             .with_speed(wave.speed)
///             .with_count(3),
///         seed: Some(seed),
///         ..Default::default()
///     },
/// );
/// // Each Updated: SonicWaveState::follow(origin, direction, position, …)
/// commands.cancel_vfx(handle);
/// ```
pub fn attack_drive_example() -> VfxDrive {
	VfxDrive::sonic_wave()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::drive::{SonicWaveState, SONIC_WAVE_RADIUS, SONIC_WAVE_RANGE, SONIC_WAVE_SPEED};

	#[test]
	fn definition_matches_mouth_emitted_defaults() {
		let def = definition(Handle::default());
		assert_eq!(def.name, SONIC_WAVE);
		assert_eq!(def.kind, EffectKind::SonicWave);
		let drive = VfxDrive::sonic_wave();
		assert!((drive.size - SONIC_WAVE_RANGE).abs() < 1e-4);
		assert!((drive.speed - SONIC_WAVE_SPEED).abs() < 1e-4);
		assert!((drive.radius - SONIC_WAVE_RADIUS).abs() < 1e-4);
		assert_eq!(drive.count, 3);
		assert!((drive.reach).abs() < 1e-4);
		assert!((def.duration() - drive.total_secs()).abs() < 1e-4);
		assert_eq!(attack_drive_example().count, 3);
	}

	#[test]
	fn repeated_spawns_keep_independent_seeds() -> Result<(), String> {
		let a = VfxSpawn { seed: Some(8), drive: VfxDrive::sonic_wave(), ..default() }.resolved();
		let b = VfxSpawn { seed: Some(9), drive: VfxDrive::sonic_wave(), ..default() }.resolved();
		if a.seed == b.seed {
			return Err("seeds collided".into());
		}
		Ok(())
	}

	#[test]
	fn render_pose_does_not_grow_with_travel() -> Result<(), String> {
		let early =
			SonicWaveState::at_time(Vec3::ZERO, Vec3::Z, 22.0, 0.2, 0.0, 0.55, 0.14, 0, 20.0, None)
				.ok_or("early")?;
		let late =
			SonicWaveState::at_time(Vec3::ZERO, Vec3::Z, 22.0, 0.9, 0.0, 0.55, 0.14, 0, 20.0, None)
				.ok_or("late")?;
		if (early.ring_transform().scale.x - late.ring_transform().scale.x).abs() >= 1e-4 {
			return Err("ring scale changed".into());
		}
		Ok(())
	}
}
