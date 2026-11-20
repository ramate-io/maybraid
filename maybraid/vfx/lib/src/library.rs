//! Compiled handles and named definitions, loaded once.

use bevy::prelude::*;
use bevy_hanabi::prelude::{EffectMaterial, ParticleEffect};
use bevy_hanabi::EffectAsset;

use crate::composition::{EffectDefinition, EffectRecipe};
use crate::effects::dust::Dust;
use crate::effects::fiery_explosion::FieryExplosion;
use crate::layers::{fireball, flash, smoke, sparks};
use crate::names::{DUST, FIERY_EXPLOSION, FIREBALL, FLASH, SMOKE, SPARKS};
use crate::parts::lobe::rounded_lobe_mesh;
use crate::parts::particle::assets::{fire_flipbook, smoke_flipbook, spark_taper, VfxFlipbooks};

/// Shared definitions. Clone a handle out; never mutate the compiled assets.
#[derive(Resource, Clone, Debug)]
pub struct VfxLibrary {
	pub cartoon_explosion: crate::effects::cartoon_explosion::CartoonExplosion,
	pub combined_explosion: crate::effects::combined_explosion::CombinedExplosion,
	pub dust: Dust,
	pub combined_landing: crate::effects::combined_choreography::CombinedLanding,
	pub combined_landing_contact: crate::effects::combined_choreography::CombinedLanding,
	pub combined_wave_contact: crate::effects::combined_choreography::CombinedWaveContact,
	pub combined: crate::effects::combined::CombinedDust,
	pub cartoon_wave: crate::effects::cartoon_wave::CartoonWave,
	pub cartoon_landing: crate::effects::cartoon_landing::CartoonLanding,
	pub flipbooks: VfxFlipbooks,
	pub flash: EffectDefinition,
	pub fireball: EffectDefinition,
	pub smoke: EffectDefinition,
	pub sparks: EffectDefinition,
	pub fiery_explosion: FieryExplosion,
}

impl VfxLibrary {
	pub fn get(&self, name: &str) -> Option<EffectDefinition> {
		let name = canonicalize_effect_name(name)?;
		Some(match name {
			"cartoon-explosion" => self.cartoon_explosion.effect_definition(),
			"combined-explosion" => self.combined_explosion.effect_definition(),
			"explosion-compare" => {
				let mut a = self.cartoon_explosion.effect_definition();
				let mut b = self.combined_explosion.effect_definition();
				a.name = "explosion-compare".into();
				for l in &mut a.layers {
					l.transform.translation.x = -4.;
				}
				for l in &mut b.layers {
					l.transform.translation.x = 4.;
				}
				a.layers.extend(b.layers);
				a
			}
			"combined-landing" => self.combined_landing.effect_definition(),
			"combined-landing-contact" => self.combined_landing_contact.effect_definition(),
			"combined-wave-contact" => self.combined_wave_contact.effect_definition(),
			"cartoon-landing" => self.cartoon_landing.effect_definition(),
			"cartoon-wave" => self.cartoon_wave.effect_definition(),
			"cartoon-wave-contact" | "cartoon-landing-contact" => {
				let landing = name.contains("landing");
				let mut d = if landing {
					self.cartoon_landing.effect_definition()
				} else {
					self.cartoon_wave.effect_definition()
				};
				d.name = name.into();
				for layer in &mut d.layers {
					let meshes = match &layer.part {
						crate::composition::EffectPart::CartoonLanding(p) => p.meshes.clone(),
						crate::composition::EffectPart::CartoonWave(p) => p.meshes.clone(),
						_ => return None,
					};
					layer.part = crate::composition::EffectPart::CartoonFlow(
						crate::composition::CartoonFlowPart {
							meshes,
							landing,
							refined: name.ends_with("-contact"),
						},
					);
				}
				d
			}
			"combined-dust" | "combined-wave" => self.combined.effect_definition(),
			DUST => self.dust.effect_definition(),
			FLASH => self.flash.clone(),
			FIREBALL => self.fireball.clone(),
			SMOKE => self.smoke.clone(),
			SPARKS => self.sparks.clone(),
			FIERY_EXPLOSION => self.fiery_explosion.effect_definition(),
			_ => return None,
		})
	}
}

/// Accept kebab, snake, and a few aliases used by the playground.
pub fn canonicalize_effect_name(name: &str) -> Option<&'static str> {
	match name.trim().to_ascii_lowercase().replace('_', "-").as_str() {
		"cartoon-explosion" => Some("cartoon-explosion"),
		"combined-explosion" => Some("combined-explosion"),
		"explosion-compare" => Some("explosion-compare"),
		"cartoon-landing" => Some("cartoon-landing"),
		"cartoon-wave" => Some("cartoon-wave"),
		"cartoon-wave-contact" => Some("cartoon-wave-contact"),
		"cartoon-landing-contact" => Some("cartoon-landing-contact"),
		"landing-compare" => Some("landing-compare"),
		"wave-compare" => Some("wave-compare"),
		"landing-contact-compare" => Some("landing-contact-compare"),
		"wave-contact-compare" => Some("wave-contact-compare"),
		"combined-wave" => Some("combined-dust"),
		"combined-landing" => Some("combined-landing"),
		"combined-landing-contact" => Some("combined-landing-contact"),
		"combined-wave-contact" => Some("combined-wave-contact"),
		"combined-dust" => Some("combined-dust"),
		"dust" | "rolling-wave" => Some(DUST),
		"flash" => Some(FLASH),
		"fireball" | "fire" => Some(FIREBALL),
		"smoke" => Some(SMOKE),
		"sparks" | "spark" => Some(SPARKS),
		"fiery-explosion" | "explosion" => Some(FIERY_EXPLOSION),
		_ => None,
	}
}

pub fn setup_vfx_library(
	mut commands: Commands,
	server: Res<AssetServer>,
	mut images: ResMut<Assets<Image>>,
	mut effects: ResMut<Assets<EffectAsset>>,
	mut meshes: ResMut<Assets<Mesh>>,
) {
	let lobe_mesh = meshes.add(rounded_lobe_mesh());
	let flipbooks = VfxFlipbooks {
		fire: fire_flipbook(&mut images),
		smoke: smoke_flipbook(&mut images),
		spark: spark_taper(&mut images),
	};
	let flash_mesh = flash::compile_mesh(lobe_mesh.clone());
	let fireball_mesh = fireball::compile_mesh(lobe_mesh.clone());
	let fireball_wisps = fireball::compile_wisps(&mut effects, &flipbooks.fire);
	let smoke_mesh = smoke::compile_mesh(lobe_mesh);
	let smoke_wisps = smoke::compile_wisps(&mut effects, &flipbooks.smoke);
	let sparks_part = sparks::compile(&mut effects, flipbooks.spark.clone());
	let library = VfxLibrary {
		cartoon_explosion: crate::effects::cartoon_explosion::CartoonExplosion::load(&server),
		combined_explosion: crate::effects::combined_explosion::CombinedExplosion::load(&server),
		dust: Dust::rolling_wave(&server, &mut meshes),
		combined_landing: crate::effects::combined_choreography::CombinedLanding::load(&server),
		combined_landing_contact: crate::effects::combined_choreography::CombinedLanding::contact(
			&server,
		),
		combined_wave_contact: crate::effects::combined_choreography::CombinedWaveContact::load(
			&server,
		),
		cartoon_landing: crate::effects::cartoon_landing::CartoonLanding::load(&server),
		cartoon_wave: crate::effects::cartoon_wave::CartoonWave::load(&server),
		combined: crate::effects::combined::CombinedDust::load(&server),
		flash: flash::definition(flash_mesh.clone()),
		fireball: fireball::definition(fireball_mesh.clone(), fireball_wisps.clone()),
		smoke: smoke::definition(smoke_mesh.clone(), smoke_wisps.clone()),
		sparks: sparks::definition(sparks_part.clone()),
		fiery_explosion: FieryExplosion::new(
			flash_mesh,
			fireball_mesh,
			fireball_wisps.clone(),
			smoke_mesh,
			smoke_wisps.clone(),
			sparks_part.clone(),
		),
		flipbooks,
	};
	for part in [&fireball_wisps, &smoke_wisps, &sparks_part] {
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
		assert_eq!(canonicalize_effect_name("fiery_explosion"), Some(FIERY_EXPLOSION));
		assert_eq!(canonicalize_effect_name("fiery-explosion"), Some(FIERY_EXPLOSION));
		assert_eq!(canonicalize_effect_name("Fire"), Some(FIREBALL));
		assert_eq!(canonicalize_effect_name("nope"), None);
	}
}

#[cfg(test)]
mod combined_wave_alias_tests {
	#[test]
	fn exact_legacy_alias() {
		assert_eq!(
			super::canonicalize_effect_name("combined-wave"),
			super::canonicalize_effect_name("combined-dust")
		);
		assert_eq!(super::canonicalize_effect_name("rolling-wave"), Some(crate::names::DUST));
	}
}
