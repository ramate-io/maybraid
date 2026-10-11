//! Propagating sonic-wave presentation. No hit geometry or damage.

use bevy::light::NotShadowCaster;
use bevy::prelude::*;

use crate::composition::{EffectDefinition, EffectKind, ScaleBounds, DEFAULT_SCALE_MIN};
use crate::drive::{VfxDrive, SONIC_WAVE_STAGGER};
use crate::field_material::{FieldKind, FieldMaterial};
use crate::fields::{FieldMaterialSlot, VfxField, VfxSustainLight, VfxWavefront};
use crate::membership::VfxMemberOf;
use crate::names::SONIC_WAVE;
use crate::palette::FieldPalette;
use crate::seed;
use crate::spawn::{VfxInstance, VfxLayerLife, VfxPendingLayers, VfxSpawn};

pub const MAX_SCALE: f32 = 8.0;
const ORIGIN_LIGHT_PEAK: f32 = 28_000.0;

/// Compiled definition. Meshes are `[ring]`.
pub fn definition(ring: Handle<Mesh>) -> EffectDefinition {
	EffectDefinition::new(SONIC_WAVE, [])
		.with_kind(EffectKind::SonicWave)
		.with_duration(VfxDrive::sonic_wave().wave_total_secs(SONIC_WAVE_STAGGER))
		.with_scale_bounds(ScaleBounds { min: DEFAULT_SCALE_MIN, max: MAX_SCALE })
		.with_meshes([ring])
}

pub fn spawn(commands: &mut Commands, definition: &EffectDefinition, spawn: VfxSpawn) -> Entity {
	let drive = spawn.drive.resolved_sonic_wave();
	let duration = drive.wave_total_secs(SONIC_WAVE_STAGGER);
	let seed = spawn.resolved_seed();
	let palette = spawn
		.palette
		.map(|palette| palette.as_field())
		.unwrap_or_else(FieldPalette::shock)
		.with_tint(spawn.tint);
	let ring = definition.meshes.first().cloned().unwrap_or_default();
	let scale = spawn.scale;

	let root = commands
		.spawn((
			Name::new("vfx-sonic_wave"),
			Transform {
				translation: spawn.transform.translation,
				rotation: spawn.transform.rotation,
				scale: Vec3::splat(scale),
			},
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

	for index in 0..drive.count {
		let delay = index as f32 * SONIC_WAVE_STAGGER;
		let mut material = FieldMaterial::new(
			FieldKind::ShockRing,
			duration,
			seed::unit(seed::stream(seed, seed::LAYER_WAVE, index)),
			&palette,
			spawn.tint_or_white(),
			spawn.clamped_intensity(),
		);
		material.set_spread(drive.reach);
		commands.spawn((
			Name::new(format!("vfx-sonic_wave-ring-{index}")),
			ChildOf(root),
			VfxMemberOf(root),
			Mesh3d(ring.clone()),
			Transform::from_scale(Vec3::new(0.3, 1.0, 0.3)),
			Visibility::Inherited,
			VfxField { kind: FieldKind::ShockRing, playback: spawn.clamped_playback() },
			VfxWavefront { speed: drive.speed, range: drive.size, delay },
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
		Transform::from_xyz(0.0, 0.6, 0.0),
		PointLight {
			color: crate::palette::ExplosionPalette::color(palette.flash),
			intensity: 0.0,
			range: (drive.size * 0.45 * scale).max(6.0),
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
/// should prefer [`crate::attack_drive`] when a `#1147` hazard is a wavefront.
/// One hazard is one ring; a three-wave Bray burst is three hazards or
/// `drive.with_count(3)` on a single spawn.
///
/// ```ignore
/// let handle = commands.spawn_vfx(
///     &library.sonic_wave,
///     VfxSpawn {
///         transform: muzzle_or_head_socket,
///         drive: VfxDrive::sonic_wave()
///             .with_size(28.0)   // range meters
///             .with_reach(2.2)   // spread radians
///             .with_speed(22.0)  // match authoritative wavefront
///             .with_count(3),
///         seed: Some(seed),
///         ..Default::default()
///     },
/// );
/// commands.cancel_vfx(handle);
/// ```
pub fn attack_drive_example() -> VfxDrive {
	VfxDrive::sonic_wave()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::drive::{SONIC_WAVE_RANGE, SONIC_WAVE_SPEED};

	#[test]
	fn definition_matches_b04_travel_defaults() {
		let def = definition(Handle::default());
		assert_eq!(def.name, SONIC_WAVE);
		assert_eq!(def.kind, EffectKind::SonicWave);
		let drive = VfxDrive::sonic_wave();
		assert!((drive.size - SONIC_WAVE_RANGE).abs() < 1e-4);
		assert!((drive.speed - SONIC_WAVE_SPEED).abs() < 1e-4);
		assert_eq!(drive.count, 3);
		assert!((def.duration() - drive.wave_total_secs(SONIC_WAVE_STAGGER)).abs() < 1e-4);
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
}
