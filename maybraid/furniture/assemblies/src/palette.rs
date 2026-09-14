//! Noisy wood / cloth / mattress picks keyed by a slot finish seed.
//!
//! # Color table (seed picks a row; recipe is fixed per part)
//!
//! | Part | Recipe | Palette |
//! |---|---|---|
//! | Bed frame, chair legs/back, chest, counter | `wood` | brown |
//! | Chair seat, bed covers | `hay` | gold / yellow / plum / teal |
//! | Mattress | `stucco` | ivory / white / oatmeal / pale sage |
//!
//! Unpainted Richmond kinds (nightstand, dresser, wardrobe, …) stay wireframe
//! using [`richmond_building_components::FurnitureGeometry::wireframe_color`].

use bevy::prelude::Color;
use material_ref::MaterialRef;
use richmond_building_shaders::{RECIPE_HAY, RECIPE_STUCCO, RECIPE_WOOD};

/// Wood: frame, legs, back, trunk, lid, counter.
pub const WOOD: [[f32; 3]; 4] =
	[[0.42, 0.28, 0.16], [0.52, 0.36, 0.20], [0.28, 0.16, 0.10], [0.34, 0.22, 0.12]];

/// Dyed cloth: covers and chair seats (not mattresses).
pub const CLOTH: [[f32; 3]; 4] =
	[[0.62, 0.50, 0.24], [0.72, 0.58, 0.28], [0.38, 0.22, 0.28], [0.22, 0.38, 0.48]];

/// Pale ticking: mattress only, so it cannot collide with covers.
pub const MATTRESS: [[f32; 3]; 4] =
	[[0.93, 0.90, 0.82], [0.96, 0.95, 0.91], [0.86, 0.80, 0.68], [0.80, 0.86, 0.78]];

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

/// Wood recipe + one palette slot (frame, trunk, legs, footer, …).
pub fn wood(seed: u64, salt: u64) -> MaterialRef {
	MaterialRef::named(RECIPE_WOOD).with_palette([rgb(&WOOD, seed, salt)])
}

/// Cloth / hay recipe for covers and seat pads.
pub fn cloth(seed: u64, salt: u64) -> MaterialRef {
	MaterialRef::named(RECIPE_HAY).with_palette([rgb(&CLOTH, seed, salt)])
}

/// Stucco recipe + pale ticking for mattresses (never the covers table).
pub fn mattress(seed: u64, salt: u64) -> MaterialRef {
	MaterialRef::named(RECIPE_STUCCO).with_palette([rgb(&MATTRESS, seed, salt)])
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
}
