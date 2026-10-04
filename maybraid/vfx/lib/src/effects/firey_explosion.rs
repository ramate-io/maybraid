//! Canonical shared realization: flash, fireball, smoke, sparks.

use crate::composition::{EffectDefinition, EffectLayer, ParticlePart, FIREY_EXPLOSION};
use crate::layers::{flash, smoke};

/// Overlay the four layers with coordinated delays. Does not duplicate emitters.
pub fn definition(
	fireball_part: ParticlePart,
	smoke_part: ParticlePart,
	sparks_part: ParticlePart,
) -> EffectDefinition {
	EffectDefinition::new(
		FIREY_EXPLOSION,
		[
			EffectLayer::light(flash::pulse()),
			EffectLayer::particle(fireball_part),
			EffectLayer::particle(smoke_part).with_delay(smoke::SMOKE_DELAY),
			EffectLayer::particle(sparks_part),
		],
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::composition::EffectPart;

	fn dummy_part(name: &str, life: f32) -> ParticlePart {
		ParticlePart {
			name: name.into(),
			effect: Default::default(),
			images: Vec::new(),
			count: 1.0,
			capacity: 1,
			max_lifetime: life,
		}
	}

	#[test]
	fn firey_explosion_overlays_four_layers() {
		let def = definition(
			dummy_part("fireball", 0.55),
			dummy_part("smoke", 2.4),
			dummy_part("sparks", 0.4),
		);
		assert_eq!(def.name, FIREY_EXPLOSION);
		assert_eq!(def.layers.len(), 4);
		assert!(matches!(def.layers[0].part, EffectPart::Light(_)));
		assert!((def.layers[2].delay - smoke::SMOKE_DELAY).abs() < 1e-4);
		assert!((def.duration() - (smoke::SMOKE_DELAY + 2.4)).abs() < 1e-4);
	}
}
