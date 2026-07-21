//! Deterministic unit noise for Marazion anchors via [`procedural_common::SeededHash`].

use bevy_math::Vec2;
use procedural_common::SeededHash;

/// Unit sample in `[0, 1)` from `seed` ⊕ `salt` at integer lattice coords.
pub fn n01(seed: u32, salt: u32, ix: i32, iz: i32) -> f32 {
	SeededHash::new(seed.wrapping_add(salt)).unit_i32(ix, iz)
}

/// Unit sample in `[0, 1)` keyed by a world point (bit-cast coords).
pub fn n01_at(seed: u32, salt: u32, p: Vec2) -> f32 {
	n01(seed, salt, p.x.to_bits() as i32, p.y.to_bits() as i32)
}

/// Signed sample in `[-1, 1)`.
pub fn n11(seed: u32, salt: u32, ix: i32, iz: i32) -> f32 {
	n01(seed, salt, ix, iz) * 2.0 - 1.0
}

pub fn n11_at(seed: u32, salt: u32, p: Vec2) -> f32 {
	n01_at(seed, salt, p) * 2.0 - 1.0
}
