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

/// High-frequency spatial unit sample.
///
/// Uses a fine world lattice so nearby leaves decorrelate, mixed with the
/// point's bit identity so two leaves on the same lattice cell still differ.
/// Returns a single hash draw (do not average multiple unit samples — that
/// shrinks variance toward ½ and flattens lake sizes).
pub fn n01_freq(seed: u32, salt: u32, p: Vec2, freq: f32) -> f32 {
	let f = freq.max(1.0e-3);
	let ix = (p.x * f).floor() as i32;
	let iz = (p.y * f).floor() as i32;
	// Second, higher octave for extra spatial scrambling.
	let jx = (p.x * f * 4.17).floor() as i32;
	let jz = (p.y * f * 4.17).floor() as i32;
	let ax = ix
		.wrapping_mul(374_761_393)
		.wrapping_add(jx.wrapping_mul(668_265_263))
		.wrapping_add(p.x.to_bits() as i32);
	let az = iz
		.wrapping_mul(127_412_617)
		.wrapping_add(jz.wrapping_mul(224_682_251))
		.wrapping_add(p.y.to_bits() as i32);
	n01(seed, salt, ax, az)
}

/// Signed sample in `[-1, 1)`.
pub fn n11(seed: u32, salt: u32, ix: i32, iz: i32) -> f32 {
	n01(seed, salt, ix, iz) * 2.0 - 1.0
}

pub fn n11_at(seed: u32, salt: u32, p: Vec2) -> f32 {
	n01_at(seed, salt, p) * 2.0 - 1.0
}
