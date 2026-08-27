//! Softmax blend of neighbor **grove** slots.
//!
//! Every presenting 100 m tile walks planting cells and draws a recipe from
//! itself plus the cardinal neighbor grove slots. Distance sets the likelihood;
//! a high-frequency logit wrinkle plus the CDF hash make the realized front
//! jagged (holes, islands). Forest cells only choose each slot's recipe.

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

	fn logit_salt(self) -> u32 {
		self.index() as u32 + 1
	}
}

/// Distance scale (m). Long enough that \(P\) stays messy across a full 100 m tile.
pub const GROVE_BLEND_INFLUENCE: f32 = 220.0;

/// Softmax temperature. \(1\) leaves the distance logits unsharpened.
pub const GROVE_BLEND_TEMPERATURE: f32 = 1.0;

/// Amplitude of per-source spatial wrinkle added to each logit (`[-amp, amp]`).
pub const GROVE_BLEND_NOISE: f32 = 1.35;

/// Recipe on each cardinal neighbor **grove** slot.
///
/// `None` = slot not in cache (omit from softmax). `Some(None)` = empty layer
/// (vote to skip the plant). `Some(Some(kind))` = that grove recipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GroveNeighbors {
	pub north: Option<Option<ForestGroveKind>>,
	pub east: Option<Option<ForestGroveKind>>,
	pub south: Option<Option<ForestGroveKind>>,
	pub west: Option<Option<ForestGroveKind>>,
}

impl GroveNeighbors {
	pub fn none() -> Self {
		Self { north: None, east: None, south: None, west: None }
	}

	pub fn get(self, face: Cardinal) -> Option<Option<ForestGroveKind>> {
		match face {
			Cardinal::North => self.north,
			Cardinal::East => self.east,
			Cardinal::South => self.south,
			Cardinal::West => self.west,
		}
	}

	/// Distinct planted kinds on present neighbor slots.
	pub fn planted_kinds(self) -> impl Iterator<Item = ForestGroveKind> {
		Cardinal::ALL.into_iter().filter_map(move |face| self.get(face).and_then(|k| k))
	}
}

/// Adjacent 100 m grove footprint in `face`.
pub fn neighbor_tile(tile: GroveExtent, face: Cardinal) -> GroveExtent {
	let d = face.offset_xz();
	GroveExtent::new(tile.min() + d, tile.max() + d)
}

fn center_xz(tile: GroveExtent) -> Vec3 {
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

fn source_logit(position: Vec3, source_center: Vec3, salt: u32) -> f32 {
	let distance = -dist_xz(position, source_center) / logit_scale();
	let wrinkle = (hash_unit_salt(position, salt) * 2.0 - 1.0) * GROVE_BLEND_NOISE;
	distance + wrinkle
}

/// Softmax winner: `None` skips the plant (empty self or neighbor mass).
///
/// `self_kind` may be `None` (this tile's layer is empty). Missing neighbor
/// slots are omitted. A spatial wrinkle on each logit plus `hash_unit` as the
/// CDF draw make the front jagged.
pub fn pick_kind(
	position: Vec3,
	tile: GroveExtent,
	self_kind: Option<ForestGroveKind>,
	neighbors: GroveNeighbors,
	hash_unit: f32,
) -> Option<ForestGroveKind> {
	let hash = hash_unit.clamp(0.0, 1.0);
	let mut kinds = [None; 5];
	let mut logits = [0.0_f32; 5];
	let mut n = 0usize;

	kinds[n] = self_kind;
	logits[n] = source_logit(position, center_xz(tile), 0);
	n += 1;
	for face in Cardinal::ALL {
		let Some(kind) = neighbors.get(face) else {
			continue;
		};
		kinds[n] = kind;
		logits[n] = source_logit(position, center_xz(neighbor_tile(tile, face)), face.logit_salt());
		n += 1;
	}

	let mut max = logits[0];
	for logit in logits.iter().take(n).skip(1) {
		if *logit > max {
			max = *logit;
		}
	}
	let mut mass = [0.0_f32; 5];
	let mut total = 0.0;
	for i in 0..n {
		let w = (logits[i] - max).exp();
		mass[i] = w;
		total += w;
	}
	let total = total.max(1e-8);
	let mut acc = 0.0;
	let mut choice = n - 1;
	for i in 0..n {
		acc += mass[i] / total;
		if hash < acc {
			choice = i;
			break;
		}
	}
	kinds[choice]
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

	fn east_oaks() -> GroveNeighbors {
		GroveNeighbors { east: Some(Some(ForestGroveKind::RollingOaks)), ..GroveNeighbors::none() }
	}

	fn draws(
		p: Vec3,
		self_kind: Option<ForestGroveKind>,
		neighbors: GroveNeighbors,
	) -> Vec<Option<ForestGroveKind>> {
		(0..24)
			.map(|i| pick_kind(p, tile(), self_kind, neighbors, i as f32 / 24.0))
			.collect()
	}

	#[test]
	fn missing_neighbors_always_self() -> Result<()> {
		let p = Vec3::new(99.0, 0.0, 50.0);
		for hash in [0.0, 0.5, 0.99] {
			assert_eq!(
				pick_kind(p, tile(), Some(ForestGroveKind::Orchard), GroveNeighbors::none(), hash),
				Some(ForestGroveKind::Orchard)
			);
		}
		Ok(())
	}

	#[test]
	fn neighbor_and_self_both_win_near_the_seam() -> Result<()> {
		let p = Vec3::new(99.0, 0.0, 50.0);
		let picks = draws(p, Some(ForestGroveKind::Orchard), east_oaks());
		assert!(picks.iter().any(|k| *k == Some(ForestGroveKind::Orchard)), "{picks:?}");
		assert!(picks.iter().any(|k| *k == Some(ForestGroveKind::RollingOaks)), "{picks:?}");
		Ok(())
	}

	#[test]
	fn empty_self_can_grow_neighbor_islands() -> Result<()> {
		let p = Vec3::new(99.0, 0.0, 50.0);
		let picks = draws(p, None, east_oaks());
		assert!(
			picks.iter().any(|k| *k == Some(ForestGroveKind::RollingOaks)),
			"expected neighbor islands, got {picks:?}"
		);
		assert!(picks.iter().any(Option::is_none), "expected holes, got {picks:?}");
		Ok(())
	}

	#[test]
	fn empty_neighbor_can_skip() -> Result<()> {
		let neighbors = GroveNeighbors { east: Some(None), ..GroveNeighbors::none() };
		let p = Vec3::new(99.0, 0.0, 50.0);
		let picks = draws(p, Some(ForestGroveKind::Orchard), neighbors);
		assert!(picks.iter().any(Option::is_none), "{picks:?}");
		assert!(picks.iter().any(|k| *k == Some(ForestGroveKind::Orchard)), "{picks:?}");
		Ok(())
	}

	#[test]
	fn neighbor_tile_is_one_grove_step() -> Result<()> {
		let east = neighbor_tile(tile(), Cardinal::East);
		assert!((east.min().x - 100.0).abs() < 1e-4);
		assert!((east.max().x - 200.0).abs() < 1e-4);
		Ok(())
	}
}
