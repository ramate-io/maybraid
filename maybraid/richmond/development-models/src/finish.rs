//! Per-cell urban finish: wall / roof [`MaterialRef`] recipes and palettes.

use bevy::prelude::Color;
use material_ref::MaterialRef;
use procedural_common::{NoiseParams, SeededHash};
use richmond_building_shaders::{
	RECIPE_HAY, RECIPE_IRON, RECIPE_STUCCO, RECIPE_TERRACOTTA, RECIPE_WOOD,
};

/// One wall look and one roof look for a filled development cell.
#[derive(Debug, Clone, PartialEq)]
pub struct DevelopmentFinish {
	pub wall: MaterialRef,
	pub roof: MaterialRef,
}

impl DevelopmentFinish {
	pub fn pick(hash: SeededHash) -> Self {
		let wall_recipe = if hash.unit(19) < 0.5 { RECIPE_STUCCO } else { RECIPE_WOOD };
		let roof_recipe = match (hash.unit(23) * 3.0).floor() as u32 {
			0 => RECIPE_IRON,
			1 => RECIPE_TERRACOTTA,
			_ => RECIPE_HAY,
		};
		Self {
			wall: recipe_material(wall_recipe, hash, 29),
			roof: recipe_material(roof_recipe, hash, 31),
		}
	}

	/// Brighter timber or darker masonry beneath a hay/thatch roof.
	pub fn pick_shepherds(hash: SeededHash, wooden: bool) -> Self {
		let wall = if wooden {
			custom_recipe_material(
				RECIPE_WOOD,
				hash,
				41,
				[(0.58, 0.39, 0.20), (0.76, 0.58, 0.32)],
				[(0.34, 0.20, 0.10), (0.50, 0.31, 0.15)],
				[0.0, 0.68, 0.22],
				0.62,
			)
		} else {
			custom_recipe_material(
				RECIPE_STUCCO,
				hash,
				41,
				[(0.22, 0.24, 0.22), (0.38, 0.36, 0.30)],
				[(0.14, 0.16, 0.15), (0.48, 0.44, 0.34)],
				[0.0, 0.46, 0.38],
				0.34,
			)
		};
		let roof = custom_recipe_material(
			RECIPE_HAY,
			hash,
			47,
			[(0.82, 0.68, 0.32), (0.50, 0.38, 0.16)],
			[(0.92, 0.80, 0.44), (0.34, 0.24, 0.10)],
			[0.0, 0.78, 0.24],
			0.76,
		);
		Self { wall, roof }
	}
}

struct RecipeLook {
	palette: [(f32, f32, f32); 2],
	accent: [(f32, f32, f32); 2],
	/// roughness, grain/mottle scale, wear
	scalars: [f32; 3],
	frequency: f32,
}

fn recipe_look(name: &str) -> RecipeLook {
	match name {
		RECIPE_WOOD => RecipeLook {
			palette: [(0.42, 0.28, 0.16), (0.52, 0.36, 0.20)],
			accent: [(0.28, 0.16, 0.10), (0.34, 0.22, 0.12)],
			scalars: [0.0, 0.62, 0.28],
			frequency: 0.55,
		},
		RECIPE_TERRACOTTA => RecipeLook {
			palette: [(0.72, 0.32, 0.18), (0.62, 0.24, 0.14)],
			accent: [(0.48, 0.18, 0.10), (0.82, 0.42, 0.24)],
			scalars: [0.0, 0.48, 0.22],
			frequency: 0.42,
		},
		RECIPE_HAY => RecipeLook {
			palette: [(0.72, 0.58, 0.28), (0.62, 0.50, 0.24)],
			accent: [(0.48, 0.36, 0.16), (0.80, 0.68, 0.38)],
			scalars: [0.0, 0.72, 0.18],
			frequency: 0.70,
		},
		RECIPE_IRON => RecipeLook {
			palette: [(0.38, 0.40, 0.42), (0.28, 0.30, 0.32)],
			accent: [(0.55, 0.22, 0.12), (0.42, 0.18, 0.10)],
			scalars: [0.0, 0.40, 0.45],
			frequency: 0.38,
		},
		_ => RecipeLook {
			palette: [(0.78, 0.72, 0.62), (0.70, 0.64, 0.54)],
			accent: [(0.62, 0.54, 0.42), (0.84, 0.78, 0.68)],
			scalars: [0.0, 0.35, 0.30],
			frequency: 0.48,
		},
	}
}

fn recipe_material(name: &'static str, hash: SeededHash, salt: u32) -> MaterialRef {
	let look = recipe_look(name);
	custom_recipe_material(
		name,
		hash,
		salt,
		look.palette,
		look.accent,
		look.scalars,
		look.frequency,
	)
}

fn custom_recipe_material(
	name: &'static str,
	hash: SeededHash,
	salt: u32,
	palette: [(f32, f32, f32); 2],
	accent: [(f32, f32, f32); 2],
	scalars: [f32; 3],
	frequency: f32,
) -> MaterialRef {
	let pick = hash.unit(salt);
	let mix = hash.unit(salt.wrapping_add(3));
	let base = lerp_rgb(palette[0], palette[1], pick);
	let accent = lerp_rgb(accent[0], accent[1], mix);
	let seed = (hash.unit(salt.wrapping_add(7)) * 10_000.0) as i32;
	MaterialRef::named(name)
		.with_palette([srgb(base), srgb(accent)])
		.with_scalars(scalars)
		.with_noise(NoiseParams {
			seed,
			frequency,
			amplitude: 1.0,
			octaves: 3,
			..NoiseParams::default()
		})
}

fn lerp_rgb(a: (f32, f32, f32), b: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
	(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
}

fn srgb(rgb: (f32, f32, f32)) -> Color {
	Color::srgb(rgb.0, rgb.1, rgb.2)
}

#[cfg(test)]
mod tests {
	use material_ref::MaterialId;
	use richmond_building_shaders::is_urban_surface_recipe;

	use super::*;

	#[test]
	fn pick_uses_wall_and_roof_recipes() {
		let finish = DevelopmentFinish::pick(SeededHash::new(42));
		let MaterialId::Name(wall) = &finish.wall.name else {
			panic!("wall recipe should be named");
		};
		let MaterialId::Name(roof) = &finish.roof.name else {
			panic!("roof recipe should be named");
		};
		assert!(matches!(wall.as_str(), RECIPE_STUCCO | RECIPE_WOOD));
		assert!(matches!(roof.as_str(), RECIPE_IRON | RECIPE_TERRACOTTA | RECIPE_HAY));
		assert!(is_urban_surface_recipe(wall));
		assert!(is_urban_surface_recipe(roof));
		assert_eq!(finish.wall.palette.len(), 2);
		assert_eq!(finish.roof.palette.len(), 2);
	}

	#[test]
	fn pick_is_stable_for_the_same_hash() {
		let a = DevelopmentFinish::pick(SeededHash::new(7));
		let b = DevelopmentFinish::pick(SeededHash::new(7));
		assert_eq!(a, b);
	}

	#[test]
	fn shepherds_finish_is_wood_or_dark_masonry_with_hay() {
		let wood = DevelopmentFinish::pick_shepherds(SeededHash::new(11), true);
		let stone = DevelopmentFinish::pick_shepherds(SeededHash::new(11), false);
		assert!(matches!(&wood.wall.name, MaterialId::Name(n) if n == RECIPE_WOOD));
		assert!(matches!(&stone.wall.name, MaterialId::Name(n) if n == RECIPE_STUCCO));
		assert!(matches!(&wood.roof.name, MaterialId::Name(n) if n == RECIPE_HAY));
		assert_ne!(wood.wall.palette, stone.wall.palette);
	}
}
