//! Seeded finish picks: carcass (wood / lacquer / metal) and chest skins.
//!
//! [`carcass`] is wood most of the time. Lacquer and metal are rare
//! (~5% each). Within wood, muted honey / cherry / ebon lead; pale and
//! painted rows are uncommon.
//!
//! # Carcass (frames, legs, backs, cabinet bodies)
//!
//! | Kind | Odds | Recipe | Palette |
//! |---|---|---|---|
//! | Wood | ~90% | `furniture_wood` | muted honey / cherry / ebon; rare birch / olive / drift |
//! | Lacquer | ~5% | `furniture_lacquer` | vermillion, indigo, jade, cream |
//! | Metal | ~5% | `furniture_metal` | brass, copper, pewter, iron |
//!
//! # Chests
//!
//! | Kind | Recipe | Field |
//! |---|---|---|
//! | Ornate | `furniture_ornate` | wood + gold filigree |
//! | Lava | `furniture_lava` | coal + pulsing veins |
//! | Cosmos | `furniture_cosmos` | nebula + star glints |
//! | Scales | `furniture_scales` | overlapping iridescent tiles |
//! | Rockadder | `furniture_rockadder` | terracotta / teal mosaic + gold inlay |
//!
//! Unpainted Richmond kinds stay wireframe.

use bevy::prelude::Color;
use furniture_shaders::{
	RECIPE_FURNITURE_CLOTH, RECIPE_FURNITURE_COSMOS, RECIPE_FURNITURE_LACQUER,
	RECIPE_FURNITURE_LAVA, RECIPE_FURNITURE_MARBLE, RECIPE_FURNITURE_METAL,
	RECIPE_FURNITURE_ORNATE, RECIPE_FURNITURE_ROCKADDER, RECIPE_FURNITURE_SCALES,
	RECIPE_FURNITURE_SOFT, RECIPE_FURNITURE_WOOD,
};
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

const CARCASS_SALT: u64 = 0xCA2C_A55E;
const CHEST_SALT: u64 = 0xC7E5_7B0D;

/// Structural look for frames, legs, backs, and cabinet bodies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CarcassKind {
	Wood,
	Lacquer,
	Metal,
}

/// Chest field look. Trunk and lid share one kind per seed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChestKind {
	Ornate,
	Lava,
	Cosmos,
	Scales,
	Rockadder,
}

/// Muted stained woods first; pale / cool rows are rare picks.
/// Accents are a different hue, not a lighter copy of the same stain.
pub const WOOD: [[f32; 3]; 6] = [
	[0.58, 0.40, 0.24],
	[0.50, 0.28, 0.20],
	[0.78, 0.70, 0.52],
	[0.18, 0.12, 0.10],
	[0.38, 0.40, 0.24],
	[0.42, 0.44, 0.42],
];
pub const WOOD_ACCENT: [[f32; 3]; 6] = [
	[0.72, 0.38, 0.14],
	[0.38, 0.18, 0.24],
	[0.82, 0.58, 0.28],
	[0.42, 0.10, 0.34],
	[0.26, 0.42, 0.22],
	[0.54, 0.38, 0.28],
];
pub const WOOD_SHADE: [[f32; 3]; 6] = [
	[0.36, 0.24, 0.18],
	[0.28, 0.14, 0.12],
	[0.52, 0.44, 0.32],
	[0.10, 0.08, 0.08],
	[0.22, 0.26, 0.16],
	[0.28, 0.28, 0.30],
];

pub const LACQUER: [[f32; 3]; 4] =
	[[0.86, 0.16, 0.14], [0.18, 0.22, 0.62], [0.12, 0.52, 0.38], [0.92, 0.84, 0.68]];
pub const LACQUER_ACCENT: [[f32; 3]; 4] =
	[[1.00, 0.38, 0.22], [0.38, 0.42, 0.88], [0.28, 0.72, 0.52], [1.00, 0.94, 0.80]];

pub const METAL: [[f32; 3]; 4] =
	[[0.78, 0.62, 0.28], [0.72, 0.38, 0.22], [0.58, 0.60, 0.62], [0.22, 0.20, 0.22]];
pub const METAL_ACCENT: [[f32; 3]; 4] =
	[[0.96, 0.82, 0.42], [0.92, 0.52, 0.30], [0.82, 0.84, 0.86], [0.40, 0.38, 0.40]];

pub const CLOTH: [[f32; 3]; 4] =
	[[0.94, 0.62, 0.16], [0.92, 0.36, 0.30], [0.16, 0.64, 0.58], [0.62, 0.26, 0.56]];
pub const CLOTH_ACCENT: [[f32; 3]; 4] =
	[[1.00, 0.82, 0.36], [1.00, 0.58, 0.42], [0.28, 0.86, 0.78], [0.82, 0.42, 0.78]];

pub const MATTRESS: [[f32; 3]; 4] =
	[[0.96, 0.90, 0.78], [0.98, 0.94, 0.86], [0.90, 0.82, 0.66], [0.92, 0.84, 0.80]];

pub const MARBLE: [[f32; 3]; 4] =
	[[0.94, 0.88, 0.76], [0.78, 0.86, 0.74], [0.92, 0.78, 0.76], [0.22, 0.24, 0.28]];
pub const MARBLE_VEIN: [[f32; 3]; 4] =
	[[0.72, 0.52, 0.24], [0.36, 0.48, 0.38], [0.70, 0.36, 0.38], [0.82, 0.70, 0.42]];
pub const MARBLE_SPARK: [[f32; 3]; 4] =
	[[0.99, 0.94, 0.80], [0.90, 0.96, 0.88], [0.99, 0.88, 0.86], [0.95, 0.90, 0.72]];

pub const GOLD: [[f32; 3]; 4] =
	[[0.96, 0.74, 0.24], [0.98, 0.82, 0.34], [0.90, 0.62, 0.18], [0.86, 0.68, 0.28]];
pub const GEM: [[f32; 3]; 4] =
	[[0.18, 0.46, 0.62], [0.62, 0.18, 0.28], [0.16, 0.52, 0.38], [0.42, 0.22, 0.58]];

pub const LAVA_COAL: [[f32; 3]; 4] =
	[[0.10, 0.05, 0.04], [0.14, 0.06, 0.04], [0.08, 0.04, 0.06], [0.16, 0.08, 0.05]];
pub const LAVA_GLOW: [[f32; 3]; 4] =
	[[1.00, 0.32, 0.05], [0.95, 0.22, 0.04], [1.00, 0.42, 0.10], [0.90, 0.18, 0.08]];
pub const LAVA_HOT: [[f32; 3]; 4] =
	[[1.00, 0.88, 0.42], [1.00, 0.78, 0.28], [1.00, 0.94, 0.62], [0.98, 0.70, 0.22]];

pub const COSMOS_VOID: [[f32; 3]; 4] =
	[[0.04, 0.02, 0.09], [0.06, 0.02, 0.12], [0.03, 0.04, 0.10], [0.08, 0.03, 0.08]];
pub const COSMOS_NEBULA: [[f32; 3]; 4] =
	[[0.22, 0.06, 0.40], [0.14, 0.08, 0.48], [0.32, 0.08, 0.36], [0.10, 0.16, 0.42]];
pub const COSMOS_BLOOM: [[f32; 3]; 4] =
	[[0.82, 0.48, 1.00], [0.62, 0.28, 1.00], [0.95, 0.62, 0.88], [0.45, 0.72, 1.00]];

pub const SCALE_BELLY: [[f32; 3]; 4] =
	[[0.22, 0.48, 0.38], [0.48, 0.22, 0.28], [0.18, 0.32, 0.52], [0.62, 0.48, 0.18]];
pub const SCALE_EDGE: [[f32; 3]; 4] =
	[[0.08, 0.16, 0.12], [0.18, 0.08, 0.10], [0.06, 0.10, 0.20], [0.22, 0.16, 0.06]];
pub const SCALE_IRID: [[f32; 3]; 4] =
	[[0.32, 0.88, 0.70], [0.88, 0.42, 0.62], [0.42, 0.72, 0.95], [0.95, 0.78, 0.32]];

pub const ROCK_TILE: [[f32; 3]; 4] =
	[[0.62, 0.28, 0.16], [0.72, 0.38, 0.16], [0.52, 0.22, 0.14], [0.68, 0.32, 0.18]];
pub const ROCK_TEAL: [[f32; 3]; 4] =
	[[0.12, 0.38, 0.36], [0.10, 0.32, 0.40], [0.16, 0.42, 0.30], [0.08, 0.28, 0.32]];
pub const ROCK_GOLD: [[f32; 3]; 4] =
	[[0.90, 0.68, 0.22], [1.00, 0.84, 0.16], [0.78, 0.58, 0.18], [0.96, 0.74, 0.28]];

pub fn mix_seed(seed: u64, salt: u64) -> u64 {
	let mut value = seed ^ salt;
	value ^= value >> 30;
	value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value ^= value >> 27;
	value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ (value >> 31)
}

fn rgb(table: &[[f32; 3]], seed: u64, salt: u64) -> Color {
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

/// Carcass recipe from the assembly seed (same for every woody part).
///
/// Wood is the default. Lacquer and metal each land on one bucket in twenty.
pub fn carcass_kind(seed: u64) -> CarcassKind {
	match mix_seed(seed, CARCASS_SALT) % 20 {
		0 => CarcassKind::Lacquer,
		1 => CarcassKind::Metal,
		_ => CarcassKind::Wood,
	}
}

/// Chest field recipe from the assembly seed (trunk and lid share it).
pub fn chest_kind(seed: u64) -> ChestKind {
	match mix_seed(seed, CHEST_SALT) % 5 {
		0 => ChestKind::Ornate,
		1 => ChestKind::Lava,
		2 => ChestKind::Cosmos,
		3 => ChestKind::Scales,
		_ => ChestKind::Rockadder,
	}
}

/// First seed in `0..limit` that yields `want`, if any.
pub fn first_seed_for_chest(want: ChestKind, limit: u64) -> Option<u64> {
	(0..limit).find(|&seed| chest_kind(seed) == want)
}

fn wood_row(seed: u64, salt: u64) -> usize {
	match mix_seed(seed, salt) % 12 {
		0..=4 => 0,
		5..=8 => 1,
		9 => 3,
		10 => 4,
		_ => {
			if mix_seed(seed, salt.wrapping_add(1)) % 2 == 0 {
				2
			} else {
				5
			}
		}
	}
}

pub fn wood(seed: u64, salt: u64) -> MaterialRef {
	let row = wood_row(seed, salt);
	recipe(
		RECIPE_FURNITURE_WOOD,
		seed ^ salt,
		2.4,
		[
			Color::srgb(WOOD[row][0], WOOD[row][1], WOOD[row][2]),
			Color::srgb(WOOD_ACCENT[row][0], WOOD_ACCENT[row][1], WOOD_ACCENT[row][2]),
			Color::srgb(WOOD_SHADE[row][0], WOOD_SHADE[row][1], WOOD_SHADE[row][2]),
		],
	)
}

pub fn lacquer(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_LACQUER,
		seed ^ salt,
		2.2,
		[rgb(&LACQUER, seed, salt), rgb(&LACQUER_ACCENT, seed, salt.wrapping_add(7))],
	)
}

pub fn metal(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_METAL,
		seed ^ salt,
		2.8,
		[rgb(&METAL, seed, salt), rgb(&METAL_ACCENT, seed, salt.wrapping_add(13))],
	)
}

/// Frame / leg / back / cabinet body. Recipe from `seed`; hue jitter from `salt`.
pub fn carcass(seed: u64, salt: u64) -> MaterialRef {
	match carcass_kind(seed) {
		CarcassKind::Wood => wood(seed, salt),
		CarcassKind::Lacquer => lacquer(seed, salt),
		CarcassKind::Metal => metal(seed, salt),
	}
}

pub fn cloth(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_CLOTH,
		seed ^ salt,
		3.4,
		[rgb(&CLOTH, seed, salt), rgb(&CLOTH_ACCENT, seed, salt.wrapping_add(5))],
	)
}

pub fn mattress(seed: u64, salt: u64) -> MaterialRef {
	recipe(RECIPE_FURNITURE_SOFT, seed ^ salt, 2.2, [rgb(&MATTRESS, seed, salt)])
}

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

pub fn lava(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_LAVA,
		seed ^ salt,
		1.9,
		[
			rgb(&LAVA_COAL, seed, salt),
			rgb(&LAVA_GLOW, seed, salt.wrapping_add(3)),
			rgb(&LAVA_HOT, seed, salt.wrapping_add(7)),
		],
	)
}

pub fn cosmos(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_COSMOS,
		seed ^ salt,
		1.6,
		[
			rgb(&COSMOS_VOID, seed, salt),
			rgb(&COSMOS_NEBULA, seed, salt.wrapping_add(3)),
			rgb(&COSMOS_BLOOM, seed, salt.wrapping_add(7)),
		],
	)
}

pub fn rockadder(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_ROCKADDER,
		seed ^ salt,
		2.0,
		[
			rgb(&ROCK_TILE, seed, salt),
			rgb(&ROCK_TEAL, seed, salt.wrapping_add(3)),
			rgb(&ROCK_GOLD, seed, salt.wrapping_add(7)),
		],
	)
}

pub fn scales(seed: u64, salt: u64) -> MaterialRef {
	recipe(
		RECIPE_FURNITURE_SCALES,
		seed ^ salt,
		2.1,
		[
			rgb(&SCALE_BELLY, seed, salt),
			rgb(&SCALE_EDGE, seed, salt.wrapping_add(3)),
			rgb(&SCALE_IRID, seed, salt.wrapping_add(7)),
		],
	)
}

/// Chest field. Recipe from `seed`; trunk/lid salts only change the row.
pub fn chest(seed: u64, salt: u64) -> MaterialRef {
	match chest_kind(seed) {
		ChestKind::Ornate => ornate(seed, salt),
		ChestKind::Lava => lava(seed, salt),
		ChestKind::Cosmos => cosmos(seed, salt),
		ChestKind::Scales => scales(seed, salt),
		ChestKind::Rockadder => rockadder(seed, salt),
	}
}

#[cfg(test)]
fn recipe_name(material: &MaterialRef) -> Option<&str> {
	match &material.name {
		material_ref::MaterialId::Name(name) => Some(name.as_str()),
		material_ref::MaterialId::Default => None,
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
	fn wood_table_spans_more_than_honey() -> anyhow::Result<()> {
		let warm = WOOD.iter().any(|row| row[0] > row[2] + 0.25);
		let pale_or_cool = WOOD.iter().any(|row| row[2] + 0.05 >= row[0] || row[1] > row[0]);
		if !warm || !pale_or_cool {
			return Err(anyhow::anyhow!("wood table should mix warm and pale/cool rows"));
		}
		Ok(())
	}

	#[test]
	fn wood_accent_is_a_different_hue() -> anyhow::Result<()> {
		for (i, (base, accent)) in WOOD.iter().zip(WOOD_ACCENT.iter()).enumerate() {
			let scale = base[0] / accent[0].max(1e-4);
			let aligned = (base[1] - accent[1] * scale).abs() + (base[2] - accent[2] * scale).abs();
			if aligned < 0.12 {
				return Err(anyhow::anyhow!(
					"wood row {i} accent is a scale of the base, not a hue shift"
				));
			}
		}
		Ok(())
	}

	#[test]
	fn carcass_is_usually_wood() -> anyhow::Result<()> {
		let kinds: Vec<_> = (0..40).map(carcass_kind).collect();
		let wood = kinds.iter().filter(|k| **k == CarcassKind::Wood).count();
		if wood < 30 {
			return Err(anyhow::anyhow!("carcass should be wood on most seeds, got {wood}/40"));
		}
		if (0..256).all(|seed| carcass_kind(seed) == CarcassKind::Wood) {
			return Err(anyhow::anyhow!("rare lacquer/metal never appeared in 0..256"));
		}
		Ok(())
	}

	#[test]
	fn chest_skins_cover_the_set() -> anyhow::Result<()> {
		for want in [
			ChestKind::Ornate,
			ChestKind::Lava,
			ChestKind::Cosmos,
			ChestKind::Scales,
			ChestKind::Rockadder,
		] {
			if first_seed_for_chest(want, 80).is_none() {
				return Err(anyhow::anyhow!("no seed in 0..80 for {want:?}"));
			}
		}
		Ok(())
	}

	#[test]
	fn recipes_match_the_part_roles() -> anyhow::Result<()> {
		if recipe_name(&marble(3, 1)) != Some(RECIPE_FURNITURE_MARBLE) {
			return Err(anyhow::anyhow!("marble recipe drifted"));
		}
		if recipe_name(&lava(3, 1)) != Some(RECIPE_FURNITURE_LAVA) {
			return Err(anyhow::anyhow!("lava recipe drifted"));
		}
		if recipe_name(&rockadder(3, 1)) != Some(RECIPE_FURNITURE_ROCKADDER) {
			return Err(anyhow::anyhow!("rockadder recipe drifted"));
		}
		Ok(())
	}
}
