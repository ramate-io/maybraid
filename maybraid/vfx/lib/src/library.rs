//! Compiled handles and named definitions, loaded once.

use bevy::prelude::*;
use bevy_hanabi::prelude::{EffectMaterial, ParticleEffect};
use bevy_hanabi::EffectAsset;

use crate::composition::{EffectDefinition, EffectRecipe};
use crate::effects::dust::Dust;
use crate::effects::fiery_explosion::FieryExplosion;
use crate::effects::ooze_burst::OozeBurst;
use crate::effects::spirit_dissolve::SpiritDissolve;
use crate::effects::spirit_spawn::SpiritSpawn;
use crate::layers::{fireball, flash, ooze, smoke, sparks, spirit, spirit_spawn};
use crate::names::{
	DUST, FIERY_EXPLOSION, FIREBALL, FLASH, OOZE_BURST, SMOKE, SPARKS, SPIRIT_DISSOLVE, SPIRIT_SPAWN,
};
use crate::parts::lobe::rounded_lobe_mesh;
use crate::parts::particle::assets::{
	fire_flipbook, ooze_blob, smoke_flipbook, spark_taper, VfxFlipbooks,
};

/// Shared definitions. Clone a handle out; never mutate the compiled assets.
#[derive(Resource, Clone, Debug)]
pub struct VfxLibrary {
	pub dust: Dust,
	pub flipbooks: VfxFlipbooks,
	pub flash: EffectDefinition,
	pub fireball: EffectDefinition,
	pub smoke: EffectDefinition,
	pub sparks: EffectDefinition,
	pub ooze_burst: OozeBurst,
	pub spirit_dissolve: SpiritDissolve,
	pub spirit_spawn: SpiritSpawn,
	pub fiery_explosion: FieryExplosion,
}

impl VfxLibrary {
	pub fn get(&self, name: &str) -> Option<EffectDefinition> {
		Some(match canonicalize_effect_name(name)? {
			DUST => self.dust.effect_definition(),
			FLASH => self.flash.clone(),
			FIREBALL => self.fireball.clone(),
			SMOKE => self.smoke.clone(),
			SPARKS => self.sparks.clone(),
			OOZE_BURST => self.ooze_burst.effect_definition(),
			SPIRIT_DISSOLVE => self.spirit_dissolve.effect_definition(),
			SPIRIT_SPAWN => self.spirit_spawn.effect_definition(),
			FIERY_EXPLOSION => self.fiery_explosion.effect_definition(),
			_ => return None,
		})
	}
}

/// Accept kebab, snake, and a few aliases used by the playground.
pub fn canonicalize_effect_name(name: &str) -> Option<&'static str> {
	match name.trim().to_ascii_lowercase().replace('_', "-").as_str() {
		"dust" | "rolling-wave" => Some(DUST),
		"flash" => Some(FLASH),
		"fireball" | "fire" => Some(FIREBALL),
		"smoke" => Some(SMOKE),
		"sparks" | "spark" => Some(SPARKS),
		"ooze" | "ooze-burst" | "blood" => Some(OOZE_BURST),
		"spirit" | "spirit-dissolve" => Some(SPIRIT_DISSOLVE),
		"spirit-spawn" | "spawn" => Some(SPIRIT_SPAWN),
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
		ooze: ooze_blob(&mut images),
	};
	let flash_mesh = flash::compile_mesh(lobe_mesh.clone());
	let fireball_mesh = fireball::compile_mesh(lobe_mesh.clone());
	let fireball_wisps = fireball::compile_wisps(&mut effects, &flipbooks.fire);
	let smoke_mesh = smoke::compile_mesh(lobe_mesh.clone());
	let smoke_wisps = smoke::compile_wisps(&mut effects, &flipbooks.smoke);
	let sparks_part = sparks::compile(&mut effects, flipbooks.spark.clone());
	let ooze_splat = ooze::compile_splat(lobe_mesh.clone());
	let ooze_droplets = ooze::compile_droplets(&mut effects, flipbooks.ooze.clone());
	let ooze_drips = ooze::compile_drips(&mut effects, flipbooks.ooze.clone());
	let spirit_shell = spirit::compile_shell(lobe_mesh.clone());
	let spirit_streaks = spirit::compile_streaks(&mut effects, flipbooks.spark.clone());
	let spirit_motes = spirit::compile_motes(&mut effects, &flipbooks.smoke);
	let spirit_embers = spirit::compile_embers_layer(&mut effects, flipbooks.spark.clone());
	let spawn_shell = spirit_spawn::compile_shell(lobe_mesh.clone());
	let spawn_streaks = spirit_spawn::compile_streaks(&mut effects, flipbooks.spark.clone());
	let spawn_motes = spirit_spawn::compile_motes(&mut effects, &flipbooks.smoke);
	let spawn_embers = spirit_spawn::compile_embers_layer(&mut effects, flipbooks.spark.clone());
	let library = VfxLibrary {
		dust: Dust::rolling_wave(&server, &mut meshes),
		flash: flash::definition(flash_mesh.clone()),
		fireball: fireball::definition(fireball_mesh.clone(), fireball_wisps.clone()),
		smoke: smoke::definition(smoke_mesh.clone(), smoke_wisps.clone()),
		sparks: sparks::definition(sparks_part.clone()),
		ooze_burst: OozeBurst::new(ooze_splat, ooze_droplets.clone(), ooze_drips.clone()),
		spirit_dissolve: SpiritDissolve::new(
			spirit_shell.clone(),
			spirit_streaks.clone(),
			spirit_motes.clone(),
			spirit_embers.clone(),
		),
		spirit_spawn: SpiritSpawn::new(
			spawn_shell.clone(),
			spawn_streaks.clone(),
			spawn_motes.clone(),
			spawn_embers.clone(),
		),
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
	for part in [
		&fireball_wisps,
		&smoke_wisps,
		&sparks_part,
		&ooze_droplets,
		&ooze_drips,
		&spirit_streaks,
		&spirit_motes,
		&spirit_embers,
		&spawn_streaks,
		&spawn_motes,
		&spawn_embers,
	] {
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
