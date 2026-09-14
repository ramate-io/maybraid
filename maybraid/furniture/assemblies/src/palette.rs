//! Noisy wood / cloth / marble / ornate picks keyed by a slot finish seed.
//!
//! BotW-warm: honey woods, saffron cloth, veined marble, gold inlay. Seed
//! picks a row; recipe is fixed per part.
//!
//! # Color table (sRGB; seed picks a row)
//!
//! | Part | Recipe | Palette |
//! |---|---|---|
//! | Frame, legs, back, counter body | `furniture_wood` | honey / amber / cherry / teak |
//! | Chair seat, bed covers | `furniture_cloth` | saffron / coral / teal / plum |
//! | Mattress | `furniture_soft` | warm ivory / cream / oatmeal / blush |
//! | Countertop | `furniture_marble` | cream+gold / sage / rose / ink |
//! | Chest trunk + lid | `furniture_ornate` | wood + gold + gem panel |
//!
//! Unpainted Richmond kinds (nightstand, dresser, wardrobe, …) stay wireframe
//! using [`richmond_building_components::FurnitureGeometry::wireframe_color`].

use bevy::prelude::Color;
use furniture_shaders::{
	RECIPE_FURNITURE_CLOTH, RECIPE_FURNITURE_MARBLE, RECIPE_FURNITURE_ORNATE,
	RECIPE_FURNITURE_SOFT, RECIPE_FURNITURE_WOOD,
};
use material_ref::{MaterialId, MaterialRef};
use procedural_common::NoiseParams;

/// Honey / amber / cherry / teak — lifted out of urban mud.
pub const WOOD: [[f32; 3]; 4] =
	[[0.78, 0.50, 0.22], [0.86, 0.56, 0.20], [0.70, 0.32, 0.18], [0.58, 0.36, 0.16]];
/// Grain highlight (gold-orange).
pub const WOOD_ACCENT: [[f32; 3]; 4] =
	[[0.96, 0.72, 0.32], [0.98, 0.78, 0.38], [0.88, 0.48, 0.24], [0.82, 0.58, 0.28]];

/// Dyed cloth: covers and chair seats.
pub const CLOTH: [[f32; 3]; 4] =
	[[0.94, 0.62, 0.16], [0.92, 0.36, 0.30], [0.16, 0.64, 0.58], [0.62, 0.26, 0.56]];
pub const CLOTH_ACCENT: [[f32; 3]; 4] =
	[[1.00, 0.82, 0.36], [1.00, 0.58, 0.42], [0.28, 0.86, 0.78], [0.82, 0.42, 0.78]];

/// Pale ticking: mattress only.
pub const MATTRESS: [[f32; 3]; 4] =
	[[0.96, 0.90, 0.78], [0.98, 0.94, 0.86], [0.90, 0.82, 0.66], [0.92, 0.84, 0.80]];

/// Countertop field.
pub const MARBLE: [[f32; 3]; 4] =
	[[0.94, 0.88, 0.76], [0.78, 0.86, 0.74], [0.92, 0.78, 0.76], [0.22, 0.24, 0.28]];
/// Vein / gold hairline.
pub const MARBLE_VEIN: [[f32; 3]; 4] =
	[[0.72, 0.52, 0.24], [0.36, 0.48, 0.38], [0.70, 0.36, 0.38], [0.82, 0.70, 0.42]];
/// Specular sparkle.
pub const MARBLE_SPARK: [[f32; 3]; 4] =
	[[0.99, 0.94, 0.80], [0.90, 0.96, 0.88], [0.99, 0.88, 0.86], [0.95, 0.90, 0.72]];

/// Chest gold leaf.
pub const GOLD: [[f32; 3]; 4] =
	[[0.96, 0.74, 0.24], [0.98, 0.82, 0.34], [0.90, 0.62, 0.18], [0.86, 0.68, 0.28]];
/// Painted chest panel / gem.
pub const GEM: [[f32; 3]; 4] =
	[[0.18, 0.46, 0.62], [0.62, 0.18, 0.28], [0.16, 0.52, 0.38], [0.42, 0.22, 0.58]];

/// Splitmix-style mix so nearby seeds diverge.
pub fn mix_seed(seed: u64, salt: u64) -> u64 {
	let mut value = seed ^ salt;
	value ^= value >> 30;
	value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value ^= value >> 27;
	value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ (value >> 31)
}

fn rgb(table: &[[f32; 3]; 4], seed: u64, salt: u64) -> Color {
	let [r, g, b] = table[(mix_seed(seed, salt) as usize) % table.len()];
	Color::srgb(r, g, b)
}

fn look_noise(seed: u64, frequency: f32) -> NoiseParams {
	NoiseParams { seed: seed as i32, frequency, ..NoiseParams::default() }
}

fn recipe(
	name: &'static str,
	seed: u64,
	frequency: f32,
	palette: impl IntoIterator<Item = Color>,
) -> MaterialRef {
	MaterialRef::named(name)
		.with_palette(palette)
		.with_noise(look_noise(seed, frequency))
}

/// Honey wood + grain accent (frame, legs, footer, cabinet).
pub fn wood(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_WOOD,
		seed ^ salt,
		2.6,
		[rgb(&WOOD, seed, salt), rgb(&WOOD_ACCENT, seed, salt.wrapping_add(11))],
	)
}

/// Vivid cloth / weave for covers and seat pads.
pub fn cloth(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_CLOTH,
		seed ^ salt,
		3.4,
		[rgb(&CLOTH, seed, salt), rgb(&CLOTH_ACCENT, seed, salt.wrapping_add(5))],
	)
}

/// Soft ticking for mattresses (never the covers table).
pub fn mattress(seed: u64, salt: u64) -> MaterialRef {
	recipe(RECIPE_FURNITURE_SOFT, seed ^ salt, 2.2, [rgb(&MATTRESS, seed, salt)])
}

/// Veined marble for countertops.
pub fn marble(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_MARBLE,
		seed ^ salt,
		1.8,
		[
			rgb(&MARBLE, seed, salt),
			rgb(&MARBLE_VEIN, seed, salt.wrapping_add(3)),
			rgb(&MARBLE_SPARK, seed, salt.wrapping_add(7)),
		],
	)
}

/// Painted wood + gold filigree for chests.
pub fn ornate(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_ORNATE,
		seed ^ salt,
		2.4,
		[
			rgb(&WOOD, seed, salt),
			rgb(&GOLD, seed, salt.wrapping_add(5)),
			rgb(&GEM, seed, salt.wrapping_add(9)),
		],
	)
}

#[cfg(test)]
fn recipe_name(material: &MaterialRef) -> Option<&str> {
	match &material.name {
		MaterialId::Name(name) => Some(name.as_str()),
		MaterialId::Default => None,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn mattress_paint_is_not_cover_paint() -> anyhow::Result<()> {
		for seed in [1_u64, 7, 99] {
			if mattress(seed, 2) == cloth(seed, 3) {
				return Err(anyhow::anyhow!("seed {seed}: mattress matched covers"));
			}
		}
		Ok(())
	}

	#[test]
	fn wood_rows_stay_warmer_than_cool_brown() -> anyhow::Result<()> {
		for row in WOOD {
			if row[0] <= row[2] + 0.12 {
				return Err(anyhow::anyhow!("wood row {row:?} is not warm (R should lead B)"));
			}
			if row[0] + row[1] < 0.85 {
				return Err(anyhow::anyhow!("wood row {row:?} is too drab"));
			}
		}
		Ok(())
	}

	#[test]
	fn recipes_match_the_part_roles() -> anyhow::Result<()> {
		if recipe_name(&marble(3, 1)) != Some(RECIPE_FURNITURE_MARBLE) {
			return Err(anyhow::anyhow!("marble recipe drifted"));
		}
		if recipe_name(&ornate(3, 1)) != Some(RECIPE_FURNITURE_ORNATE) {
			return Err(anyhow::anyhow!("ornate recipe drifted"));
		}
		Ok(())
	}
}
