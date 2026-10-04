//! Canonical shared realization: flash, fireball, smoke, sparks.

use crate::composition::{EffectDefinition, EffectLayer, MeshPart, ParticlePart};
use crate::names::FIREY_EXPLOSION;
use crate::layers::{flash, smoke};

/// Overlay the four layer modules. Mesh cores plus optional cards, coordinated in time.
pub fn definition(
	flash_mesh: MeshPart,
	fireball_mesh: MeshPart,
	fireball_wisps: ParticlePart,
	smoke_mesh: MeshPart,
	smoke_wisps: ParticlePart,
	sparks_part: ParticlePart,
) -> EffectDefinition {
	EffectDefinition::new(
		FIREY_EXPLOSION,
		[
			EffectLayer::mesh(flash_mesh),
			EffectLayer::light(flash::pulse()),
			EffectLayer::mesh(fireball_mesh),
			EffectLayer::particle(fireball_wisps),
			EffectLayer::mesh(smoke_mesh).with_delay(smoke::SMOKE_DELAY),
			EffectLayer::particle(smoke_wisps).with_delay(smoke::SMOKE_DELAY),
			EffectLayer::particle(sparks_part),
		],
	)
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

	#[test]
	fn firey_explosion_assembles_the_four_modules() {
		let def = definition(
			dummy_mesh("flash", 0.12, LobeKind::Flash),
			dummy_mesh("fireball", 0.55, LobeKind::Fire),
			dummy_particle("fireball-wisps", 0.55),
			dummy_mesh("smoke", 2.4, LobeKind::Smoke),
			dummy_particle("smoke-wisps", 2.4),
			dummy_particle("sparks", 0.4),
		);
		assert_eq!(def.name, FIREY_EXPLOSION);
		assert_eq!(def.layers.len(), 7);
		assert!(matches!(def.layers[0].part, EffectPart::Mesh(_)));
		assert!(matches!(def.layers[1].part, EffectPart::Light(_)));
		assert!((def.layers[4].delay - smoke::SMOKE_DELAY).abs() < 1e-4);
		assert!((def.duration() - (smoke::SMOKE_DELAY + 2.4)).abs() < 1e-4);
	}
}
