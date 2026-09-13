//! Noisy wood / cloth picks keyed by a slot finish seed.

use bevy::prelude::Color;
use material_ref::MaterialRef;
use richmond_building_shaders::{RECIPE_HAY, RECIPE_WOOD};

const WOOD: [[f32; 3]; 4] =
	[[0.42, 0.28, 0.16], [0.52, 0.36, 0.20], [0.28, 0.16, 0.10], [0.34, 0.22, 0.12]];

const CLOTH: [[f32; 3]; 4] =
	[[0.62, 0.50, 0.24], [0.72, 0.58, 0.28], [0.38, 0.22, 0.28], [0.22, 0.38, 0.48]];

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

/// Cloth / hay recipe for mattress, covers, and seat pads.
pub fn cloth(seed: u64, salt: u64) -> MaterialRef {
	MaterialRef::named(RECIPE_HAY).with_palette([rgb(&CLOTH, seed, salt)])
}
