//! Compiled handles and named definitions, loaded once.

use bevy::prelude::*;
use bevy_hanabi::prelude::{EffectMaterial, ParticleEffect};
use bevy_hanabi::EffectAsset;

use crate::assets::{fire_flipbook, smoke_flipbook};
use crate::composition::{
	EffectDefinition, VfxFlipbooks, FIREBALL, FIREY_EXPLOSION, FLASH, SMOKE, SPARKS,
};
use crate::effects::firey_explosion;
use crate::layers::{fireball, flash, smoke, sparks};

/// Shared definitions. Clone a handle out; never mutate the compiled assets.
#[derive(Resource, Clone, Debug)]
pub struct VfxLibrary {
	pub flipbooks: VfxFlipbooks,
	pub flash: EffectDefinition,
	pub fireball: EffectDefinition,
	pub smoke: EffectDefinition,
	pub sparks: EffectDefinition,
	pub firey_explosion: EffectDefinition,
}

impl VfxLibrary {
	pub fn get(&self, name: &str) -> Option<&EffectDefinition> {
		match canonicalize_effect_name(name)? {
			FLASH => Some(&self.flash),
			FIREBALL => Some(&self.fireball),
			SMOKE => Some(&self.smoke),
			SPARKS => Some(&self.sparks),
			FIREY_EXPLOSION => Some(&self.firey_explosion),
			_ => None,
		}
	}
}

/// Accept kebab, snake, and a few aliases used by the playground.
pub fn canonicalize_effect_name(name: &str) -> Option<&'static str> {
	match name.trim().to_ascii_lowercase().replace('_', "-").as_str() {
		"flash" => Some(FLASH),
		"fireball" | "fire" => Some(FIREBALL),
		"smoke" => Some(SMOKE),
		"sparks" | "spark" => Some(SPARKS),
		"firey-explosion" | "fiery-explosion" | "explosion" => Some(FIREY_EXPLOSION),
		_ => None,
	}
}

pub fn setup_vfx_library(
	mut commands: Commands,
	mut images: ResMut<Assets<Image>>,
	mut effects: ResMut<Assets<EffectAsset>>,
) {
	let flipbooks =
		VfxFlipbooks { fire: fire_flipbook(&mut images), smoke: smoke_flipbook(&mut images) };
	let fireball_part = fireball::compile(&mut effects, &flipbooks.fire);
	let smoke_part = smoke::compile(&mut effects, &flipbooks.smoke);
	let sparks_part = sparks::compile(&mut effects);
	let library = VfxLibrary {
		flash: flash::definition(),
		fireball: fireball::definition(fireball_part.clone()),
		smoke: smoke::definition(smoke_part.clone()),
		sparks: sparks::definition(sparks_part.clone()),
		firey_explosion: firey_explosion::definition(
			fireball_part.clone(),
			smoke_part.clone(),
			sparks_part.clone(),
		),
		flipbooks,
	};
	// Hidden instances compile GPU shaders before the first visible burst.
	for part in [&fireball_part, &smoke_part, &sparks_part] {
		let mut warmup = commands.spawn((
			Name::new(format!("vfx-warmup-{}", part.name)),
			ParticleEffect::new(part.effect.clone()),
			Transform::from_xyz(0.0, -80.0, 0.0),
			Visibility::Hidden,
		));
		if !part.images.is_empty() {
			warmup.insert(EffectMaterial { images: part.images.clone() });
		}
	}
	commands.insert_resource(library);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn aliases_resolve_to_canonical_names() {
		assert_eq!(canonicalize_effect_name("firey_explosion"), Some(FIREY_EXPLOSION));
		assert_eq!(canonicalize_effect_name("fiery-explosion"), Some(FIREY_EXPLOSION));
		assert_eq!(canonicalize_effect_name("Fire"), Some(FIREBALL));
		assert_eq!(canonicalize_effect_name("nope"), None);
	}
}
