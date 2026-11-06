//! Canonical shared realization: flash, fireball, smoke, sparks.

use bevy::prelude::*;

use crate::composition::{
	EffectDefinition, EffectLayer, EffectRecipe, MeshPart, ParticlePart, ScaleBounds,
	DEFAULT_SCALE_MIN,
};
use crate::layers::{flash, smoke};
use crate::names::FIERY_EXPLOSION;

/// Large enough for street-sized grenade blasts without raising the generic VFX cap.
pub const MAX_SCALE: f32 = 20.0;

/// Authored fiery explosion recipe. Mutating this after spawn does not rebuild the instance.
#[derive(Component, Clone, Debug)]
pub struct FieryExplosion {
	pub flash_mesh: MeshPart,
	pub fireball_mesh: MeshPart,
	pub fireball_wisps: ParticlePart,
	pub smoke_mesh: MeshPart,
	pub smoke_wisps: ParticlePart,
	pub sparks_part: ParticlePart,
}

impl FieryExplosion {
	pub fn new(
		flash_mesh: MeshPart,
		fireball_mesh: MeshPart,
		fireball_wisps: ParticlePart,
		smoke_mesh: MeshPart,
		smoke_wisps: ParticlePart,
		sparks_part: ParticlePart,
	) -> Self {
		Self { flash_mesh, fireball_mesh, fireball_wisps, smoke_mesh, smoke_wisps, sparks_part }
	}

	pub fn scale_bounds() -> ScaleBounds {
		ScaleBounds { min: DEFAULT_SCALE_MIN, max: MAX_SCALE }
	}
}

impl EffectRecipe for FieryExplosion {
	fn effect_definition(&self) -> EffectDefinition {
		EffectDefinition::new(
			FIERY_EXPLOSION,
			[
				EffectLayer::mesh(self.flash_mesh.clone()),
				EffectLayer::light(flash::pulse()),
				EffectLayer::mesh(self.fireball_mesh.clone()),
				EffectLayer::particle(self.fireball_wisps.clone()),
				EffectLayer::mesh(self.smoke_mesh.clone()).with_delay(smoke::SMOKE_DELAY),
				EffectLayer::particle(self.smoke_wisps.clone()).with_delay(smoke::SMOKE_DELAY),
				EffectLayer::particle(self.sparks_part.clone()),
			],
		)
		.with_scale_bounds(Self::scale_bounds())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::composition::{EffectPart, LobeKind, ParticleShade};

	fn dummy_particle(name: &str, life: f32) -> ParticlePart {
		ParticlePart {
			name: name.into(),
			effect: Default::default(),
			images: Vec::new(),
			count: 1.0,
			capacity: 1,
			max_lifetime: life,
			shade: ParticleShade::Fire,
		}
	}

	fn dummy_mesh(name: &str, life: f32, kind: LobeKind) -> MeshPart {
		MeshPart {
			name: name.into(),
			mesh: Default::default(),
			kind,
			lobes: Vec::new(),
			duration: life,
		}
	}

	fn sample() -> FieryExplosion {
		FieryExplosion::new(
			dummy_mesh("flash", 0.12, LobeKind::Flash),
			dummy_mesh("fireball", 0.55, LobeKind::Fire),
			dummy_particle("fireball-wisps", 0.55),
			dummy_mesh("smoke", 2.4, LobeKind::Smoke),
			dummy_particle("smoke-wisps", 2.4),
			dummy_particle("sparks", 0.4),
		)
	}

	#[test]
	fn fiery_explosion_assembles_the_four_modules() {
		let def = sample().effect_definition();
		assert_eq!(def.name, FIERY_EXPLOSION);
		assert_eq!(def.layers.len(), 7);
		assert!(matches!(def.layers[0].part, EffectPart::Mesh(_)));
		assert!(matches!(def.layers[1].part, EffectPart::Light(_)));
		assert!((def.layers[4].delay - smoke::SMOKE_DELAY).abs() < 1e-4);
		assert!((def.duration() - (smoke::SMOKE_DELAY + 2.4)).abs() < 1e-4);
		assert_eq!(def.scale_bounds.max, MAX_SCALE);
	}

	#[test]
	fn fiery_explosion_keeps_grenade_scale() {
		let def = sample().effect_definition();
		let spawn = crate::spawn::VfxSpawn { scale: 13.0, ..Default::default() }.resolved_for(&def);
		assert!((spawn.scale - 13.0).abs() < 1e-4);
	}
}
