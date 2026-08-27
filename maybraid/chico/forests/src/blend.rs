//! Softmax blend of **grove** slots.
//!
//! Every presenting 100 m tile draws a recipe from itself and a cardinal run of
//! neighbor grove tiles. Each slot's kind is whoever **produced** that grove.
//! Same-kind slots share one logit (best influence) so a block of identical
//! groves does not drown the seam. Distance is the likelihood; a logit wrinkle
//! plus the CDF hash make the realized front jagged.

use bevy_math::Vec3;
use chico_groves::GroveExtent;

use crate::{ForestGroveKind, DEFAULT_FOREST_GROVE_TILE_XZ};

/// North, east, south, west.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Cardinal {
	North,
	East,
	South,
	West,
}

impl Cardinal {
	pub const ALL: [Self; 4] = [Self::North, Self::East, Self::South, Self::West];

	pub fn index(self) -> usize {
		match self {
			Self::North => 0,
			Self::East => 1,
			Self::South => 2,
			Self::West => 3,
		}
	}

	fn offset_xz(self) -> Vec3 {
		let s = DEFAULT_FOREST_GROVE_TILE_XZ;
		match self {
			Self::North => Vec3::new(0.0, 0.0, s),
			Self::East => Vec3::new(s, 0.0, 0.0),
			Self::South => Vec3::new(0.0, 0.0, -s),
			Self::West => Vec3::new(-s, 0.0, 0.0),
		}
	}
}

/// How many 100 m grove steps to read along each cardinal.
pub const GROVE_BLEND_RADIUS: u32 = 8;

/// Distance scale (m). Sized so a grove several tiles away still has mass.
pub const GROVE_BLEND_INFLUENCE: f32 = 500.0;

/// Softmax temperature. \(1\) leaves the distance logits unsharpened.
pub const GROVE_BLEND_TEMPERATURE: f32 = 1.0;

/// Amplitude of per-source spatial wrinkle added to each logit (`[-amp, amp]`).
pub const GROVE_BLEND_NOISE: f32 = 1.5;

/// One produced grove (center + kind). `kind` is `None` for an empty layer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlendSlot {
	pub center: Vec3,
	pub kind: Option<ForestGroveKind>,
}

impl BlendSlot {
	pub fn planted_kinds(slots: &[Self]) -> impl Iterator<Item = ForestGroveKind> + '_ {
		slots.iter().filter_map(|slot| slot.kind)
	}
}

/// Adjacent 100 m grove footprint in `face` (`steps` tiles away).
pub fn neighbor_tile(tile: GroveExtent, face: Cardinal) -> GroveExtent {
	neighbor_tile_steps(tile, face, 1)
}

/// Grove footprint `steps` cardinal tiles from `tile`.
pub fn neighbor_tile_steps(tile: GroveExtent, face: Cardinal, steps: u32) -> GroveExtent {
	let d = face.offset_xz() * steps as f32;
	GroveExtent::new(tile.min() + d, tile.max() + d)
}

pub fn tile_center_xz(tile: GroveExtent) -> Vec3 {
	(tile.min() + tile.max()) * 0.5
}

fn dist_xz(a: Vec3, b: Vec3) -> f32 {
	let dx = a.x - b.x;
	let dz = a.z - b.z;
	(dx * dx + dz * dz).sqrt()
}

fn logit_scale() -> f32 {
	(GROVE_BLEND_INFLUENCE * GROVE_BLEND_TEMPERATURE.max(1e-3)).max(1e-3)
}

fn slot_salt(center: Vec3) -> u32 {
	center.x.to_bits().wrapping_add(center.z.to_bits().rotate_left(11))
}

fn source_logit(position: Vec3, source_center: Vec3) -> f32 {
	let distance = -dist_xz(position, source_center) / logit_scale();
	let wrinkle =
		(hash_unit_salt(position, slot_salt(source_center)) * 2.0 - 1.0) * GROVE_BLEND_NOISE;
	distance + wrinkle
}

/// Softmax winner over **kinds**: each kind keeps its best slot logit.
///
/// `None` skips the plant (empty-layer mass). Missing slots are simply absent
/// from `slots`.
pub fn pick_kind(position: Vec3, slots: &[BlendSlot], hash_unit: f32) -> Option<ForestGroveKind> {
	if slots.is_empty() {
		return None;
	}
	let hash = hash_unit.clamp(0.0, 1.0);
	let mut groups: Vec<(Option<ForestGroveKind>, f32)> = Vec::with_capacity(4);
	for slot in slots {
		let logit = source_logit(position, slot.center);
		if let Some((_, best)) = groups.iter_mut().find(|(kind, _)| *kind == slot.kind) {
			if logit > *best {
				*best = logit;
			}
		} else {
			groups.push((slot.kind, logit));
		}
	}
	let Some(max) = groups.iter().map(|(_, l)| *l).reduce(f32::max) else {
		return None;
	};
	let mut total = 0.0;
	let mut mass = vec![0.0_f32; groups.len()];
	for (i, (_, logit)) in groups.iter().enumerate() {
		let w = (logit - max).exp();
		mass[i] = w;
		total += w;
	}
	let total = total.max(1e-8);
	let mut acc = 0.0;
	let mut choice = groups.len() - 1;
	for (i, w) in mass.iter().enumerate() {
		acc += *w / total;
		if hash < acc {
			choice = i;
			break;
		}
	}
	groups[choice].0
}

/// Stable `0..1` from a world XZ (cell center).
pub fn hash_unit_xz(position: Vec3) -> f32 {
	hash_unit_salt(position, 0)
}

fn hash_unit_salt(position: Vec3, salt: u32) -> f32 {
	let h = position
		.x
		.to_bits()
		.wrapping_mul(0x9e3779b9)
		.wrapping_add(position.z.to_bits().wrapping_mul(0x85ebca77))
		.wrapping_add(salt.wrapping_mul(0xc2b2ae35));
	(h as f32) / (u32::MAX as f32)
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy_math::Vec3;

	fn tile() -> GroveExtent {
		GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0))
	}

	fn self_orchard() -> BlendSlot {
		BlendSlot { center: tile_center_xz(tile()), kind: Some(ForestGroveKind::Orchard) }
	}

	fn east_oaks() -> Vec<BlendSlot> {
		vec![
			self_orchard(),
			BlendSlot {
				center: tile_center_xz(neighbor_tile(tile(), Cardinal::East)),
				kind: Some(ForestGroveKind::RollingOaks),
			},
		]
	}

	fn draws(p: Vec3, slots: &[BlendSlot]) -> Vec<Option<ForestGroveKind>> {
		(0..24).map(|i| pick_kind(p, slots, i as f32 / 24.0)).collect()
	}

	#[test]
	fn missing_neighbors_always_self() -> Result<()> {
		let p = Vec3::new(99.0, 0.0, 50.0);
		let slots = [self_orchard()];
		for hash in [0.0, 0.5, 0.99] {
			assert_eq!(pick_kind(p, &slots, hash), Some(ForestGroveKind::Orchard));
		}
		Ok(())
	}

	#[test]
	fn same_kind_neighbors_do_not_drown_the_seam() -> Result<()> {
		let p = Vec3::new(99.0, 0.0, 50.0);
		let mut slots = east_oaks();
		for face in [Cardinal::North, Cardinal::South, Cardinal::West] {
			slots.push(BlendSlot {
				center: tile_center_xz(neighbor_tile(tile(), face)),
				kind: Some(ForestGroveKind::Orchard),
			});
		}
		let picks = draws(p, &slots);
		assert!(
			picks.iter().any(|k| *k == Some(ForestGroveKind::RollingOaks)),
			"max-per-kind should let oaks win often, got {picks:?}"
		);
		Ok(())
	}

	#[test]
	fn neighbor_and_self_both_win_near_the_seam() -> Result<()> {
		let p = Vec3::new(99.0, 0.0, 50.0);
		let picks = draws(p, &east_oaks());
		assert!(picks.iter().any(|k| *k == Some(ForestGroveKind::Orchard)), "{picks:?}");
		assert!(picks.iter().any(|k| *k == Some(ForestGroveKind::RollingOaks)), "{picks:?}");
		Ok(())
	}

	#[test]
	fn empty_self_can_grow_neighbor_islands() -> Result<()> {
		let p = Vec3::new(99.0, 0.0, 50.0);
		let slots = [
			BlendSlot { center: tile_center_xz(tile()), kind: None },
			BlendSlot {
				center: tile_center_xz(neighbor_tile(tile(), Cardinal::East)),
				kind: Some(ForestGroveKind::RollingOaks),
			},
		];
		let picks = draws(p, &slots);
		assert!(
			picks.iter().any(|k| *k == Some(ForestGroveKind::RollingOaks)),
			"expected neighbor islands, got {picks:?}"
		);
		assert!(picks.iter().any(Option::is_none), "expected holes, got {picks:?}");
		Ok(())
	}

	#[test]
	fn empty_neighbor_can_skip() -> Result<()> {
		let slots = [
			self_orchard(),
			BlendSlot { center: tile_center_xz(neighbor_tile(tile(), Cardinal::East)), kind: None },
		];
		let p = Vec3::new(99.0, 0.0, 50.0);
		let picks = draws(p, &slots);
		assert!(picks.iter().any(Option::is_none), "{picks:?}");
		assert!(picks.iter().any(|k| *k == Some(ForestGroveKind::Orchard)), "{picks:?}");
		Ok(())
	}

	#[test]
	fn neighbor_tile_is_one_grove_step() -> Result<()> {
		let east = neighbor_tile(tile(), Cardinal::East);
		assert!((east.min().x - 100.0).abs() < 1e-4);
		assert!((east.max().x - 200.0).abs() < 1e-4);
		let far = neighbor_tile_steps(tile(), Cardinal::East, 3);
		assert!((far.min().x - 300.0).abs() < 1e-4);
		Ok(())
	}
}
