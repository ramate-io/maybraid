//! [`DamageApplied`] → oriented ooze burst at the hit point.

use bevy::prelude::*;
use damage::{DamageApplied, DamageSystems, Health};

use crate::CombatVfxSystems;
use maybraid_vfx::{
	ExplosionPalette, EffectRecipe, SpawnVfxExt, VfxLibrary, VfxSpawn,
};
use player::{Npc, Player};

/// Stylized fluid color for impact splatter. Attach to [`Player`] / [`Npc`] bodies to override.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct CombatOozeTint(pub Color);

pub const DEFAULT_OOZE_TINT: Color = Color::srgb(0.42, 0.06, 0.12);

impl Default for CombatOozeTint {
	fn default() -> Self {
		Self(DEFAULT_OOZE_TINT)
	}
}

pub(crate) fn configure_ooze_tints(app: &mut App) {
	app.add_systems(
		PostUpdate,
		ensure_default_ooze_tints
			.after(DamageSystems::Apply)
			.before(CombatVfxSystems::Impact),
	);
}

fn ensure_default_ooze_tints(
	mut commands: Commands,
	bodies: Query<Entity, (With<Health>, Or<(With<Player>, With<Npc>)>, Without<CombatOozeTint>)>,
) {
	for entity in &bodies {
		commands.entity(entity).insert(CombatOozeTint::default());
	}
}

pub(crate) fn spawn_ooze_on_damage(
	mut applied: MessageReader<DamageApplied>,
	library: Option<Res<VfxLibrary>>,
	mut commands: Commands,
	transforms: Query<&GlobalTransform>,
	tints: Query<&CombatOozeTint>,
) {
	let Some(library) = library else {
		return;
	};
	for hit in applied.read() {
		if hit.is_zero_effect() || !hit.point.is_finite() {
			continue;
		}
		let direction = impact_direction(hit, &transforms);
		let strength = (hit.applied / 12.0).clamp(0.35, 2.0);
		let tint = tints.get(hit.target).copied().unwrap_or_default().0;
		let mut palette = ExplosionPalette::maybraid_ooze().with_tint(Some(tint));
		palette.spark = palette.fire_hot;
		let transform = Transform::from_translation(hit.point)
			.with_rotation(Quat::from_rotation_arc(Vec3::Z, direction.normalize_or_zero()));
		commands.spawn_vfx(
			&library.ooze_burst.effect_definition(),
			VfxSpawn {
				transform,
				scale: strength.sqrt().clamp(0.55, 1.6),
				intensity: strength.clamp(0.5, 1.8),
				playback: 1.0,
				tint: Some(tint),
				palette: Some(palette),
				seed: Some(
					hit.target.to_bits()
						^ (hit.point.x.to_bits() as u64)
							.rotate_left(7)
							^ (hit.point.z.to_bits() as u64),
				),
				..default()
			},
		);
	}
}

fn impact_direction(hit: &DamageApplied, transforms: &Query<&GlobalTransform>) -> Vec3 {
	if let Some(source) = hit.source {
		if let Ok(source_transform) = transforms.get(source) {
			let incoming = hit.point - source_transform.translation();
			if incoming.length_squared() > 1e-4 {
				return incoming.normalize();
			}
		}
	}
	Vec3::Z
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_vfx::{
		Dust, DustLayer, EffectDefinition, FieryExplosion, FlipbookPart, LobeKind, MeshPart,
		OozeBurst, ParticlePart, ParticleShade, SpiritDissolve, VfxFlipbooks, VfxInstance, VfxLibrary,
	};

	fn stub_library() -> VfxLibrary {
		let particle = ParticlePart {
			name: "stub".into(),
			effect: Default::default(),
			images: Vec::new(),
			count: 1.0,
			capacity: 1,
			max_lifetime: 0.5,
			shade: ParticleShade::Ooze,
		};
		let blank_mesh = MeshPart {
			name: "stub".into(),
			mesh: Handle::default(),
			kind: LobeKind::Flash,
			lobes: Vec::new(),
			duration: 0.0,
		};
		VfxLibrary {
			dust: Dust::new(DustLayer::new(FlipbookPart::default())),
			flipbooks: VfxFlipbooks {
				fire: maybraid_vfx::FlipbookAsset {
					image: Handle::default(),
					grid: UVec2::ONE,
					frame_count: 1,
					playback: 1.0,
				},
				smoke: maybraid_vfx::FlipbookAsset {
					image: Handle::default(),
					grid: UVec2::ONE,
					frame_count: 1,
					playback: 1.0,
				},
				spark: Handle::default(),
				ooze: Handle::default(),
			},
			flash: EffectDefinition::new("flash", []),
			fireball: EffectDefinition::new("fireball", []),
			smoke: EffectDefinition::new("smoke", []),
			sparks: EffectDefinition::new("sparks", []),
			ooze_burst: OozeBurst::new(particle.clone(), particle.clone()),
			spirit_dissolve: SpiritDissolve::new(particle.clone()),
			fiery_explosion: FieryExplosion::new(
				blank_mesh.clone(),
				blank_mesh.clone(),
				particle.clone(),
				blank_mesh,
				particle.clone(),
				particle,
			),
		}
	}

	#[test]
	fn damage_spawns_one_ooze_instance() {
		let mut app = App::new();
		app.add_message::<DamageApplied>()
			.insert_resource(stub_library())
			.add_systems(PostUpdate, spawn_ooze_on_damage);
		let target = app.world_mut().spawn_empty().id();
		app.world_mut().write_message(DamageApplied {
			target,
			source: None,
			amount: 20.0,
			applied: 15.0,
			remaining: 5.0,
			point: Vec3::new(1.0, 1.0, 0.0),
		});
		app.update();
		let count = app.world_mut().query::<&VfxInstance>().iter(app.world()).count();
		assert_eq!(count, 1);
	}

	#[test]
	fn zero_effect_hits_stay_silent() {
		let mut app = App::new();
		app.add_message::<DamageApplied>()
			.insert_resource(stub_library())
			.add_systems(PostUpdate, spawn_ooze_on_damage);
		app.world_mut().write_message(DamageApplied {
			target: Entity::PLACEHOLDER,
			source: None,
			amount: 0.0,
			applied: 0.0,
			remaining: 100.0,
			point: Vec3::ZERO,
		});
		app.update();
		assert_eq!(app.world_mut().query::<&VfxInstance>().iter(app.world()).count(), 0);
	}
}
