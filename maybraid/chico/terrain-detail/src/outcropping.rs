//! Outcropping recipes — the grove analog ([#785](https://github.com/ramate-io/maybraid/issues/785)).
//!
//! Each recipe owns one 40 m cell (center-in-tile). Offsets may nibble past
//! the cell. Embed is `y = terrain_height(xz) - embed` ([RFC-170 §3.1.5]).

use std::f32::consts::TAU;
use std::sync::OnceLock;

use bevy_math::{Quat, Vec2, Vec3};
use procedural_common::{Bounds2, HysteresisConfig, HysteresisGraph, NoiseParams, SeededHash};

use crate::{
	OutcroppingExtent, RockComponent, TerrainDetailWorldSample, DEFAULT_OUTCROPPING_EXTENT_XZ,
};

/// Count hashed into this inclusive range ("or so" ~20).
const CLUSTER_COUNT: std::ops::RangeInclusive<u32> = 16..=24;
/// A few mixed rocks so Empty / background cells still read as terrain.
const SPARSE_COUNT: std::ops::RangeInclusive<u32> = 2..=5;

/// One placed unit rock. Scale is world metres (authored GLB is 1 m tall).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RockPlacement {
	pub component: RockComponent,
	pub translation: Vec3,
	pub yaw: f32,
	pub scale: f32,
}

impl RockPlacement {
	pub fn transform(self) -> bevy::prelude::Transform {
		bevy::prelude::Transform {
			translation: self.translation,
			rotation: Quat::from_rotation_y(self.yaw),
			scale: Vec3::splat(self.scale),
		}
	}
}

/// Local recipe grown inside one 40 m cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum OutcroppingKind {
	/// ~20 moderate (~4 m) RoundRocks in a mound.
	RockPile,
	/// ~20 RoundRock + RockKnob, sunk, random yaw, 3–10 m ([RFC-170 §3.1]).
	BoulderPatch,
	/// Veins of SharpRock with some Round + Knob ([RFC-170 §3.2]).
	Crag,
	/// One rock, any component.
	Loner,
	/// A few mixed components scattered through the cell.
	SparseMix,
}

impl OutcroppingKind {
	pub const ALL: [Self; 5] =
		[Self::RockPile, Self::BoulderPatch, Self::Crag, Self::Loner, Self::SparseMix];

	pub fn as_kebab(self) -> &'static str {
		match self {
			Self::RockPile => "rock-pile",
			Self::BoulderPatch => "boulder-patch",
			Self::Crag => "crag",
			Self::Loner => "loner",
			Self::SparseMix => "sparse-mix",
		}
	}

	pub fn from_kebab(name: &str) -> Option<Self> {
		let key = name.trim().to_ascii_lowercase();
		Self::ALL.iter().copied().find(|kind| kind.as_kebab() == key)
	}

	/// Deterministic placements for this cell. Count hashes in [`CLUSTER_COUNT`]
	/// except [`Self::Loner`] (always one) and [`Self::SparseMix`] ([`SPARSE_COUNT`]).
	pub fn populate(
		self,
		extent: OutcroppingExtent,
		noise: NoiseParams,
		world: &impl TerrainDetailWorldSample,
	) -> Vec<RockPlacement> {
		let hash = cell_hash(extent, noise);
		match self {
			Self::RockPile => populate_pile(extent, &hash, world),
			Self::BoulderPatch => populate_patch(extent, &hash, world),
			Self::Crag => populate_crag(extent, &hash, world),
			Self::Loner => populate_loner(extent, &hash, world),
			Self::SparseMix => populate_sparse_mix(extent, &hash, world),
		}
	}
}

/// Grown 40 m origin: recipe identity plus placements after present (or a test)
/// calls [`Self::ensure_grown`]. Generate stores the kind only.
pub struct TerrainOutcropping {
	pub extent: OutcroppingExtent,
	pub kind: OutcroppingKind,
	pub noise: NoiseParams,
	placements: OnceLock<Vec<RockPlacement>>,
}

impl Clone for TerrainOutcropping {
	fn clone(&self) -> Self {
		let placements = OnceLock::new();
		if let Some(ready) = self.placements.get() {
			let _ = placements.set(ready.clone());
		}
		Self { extent: self.extent, kind: self.kind, noise: self.noise, placements }
	}
}

impl TerrainOutcropping {
	pub fn selected(extent: OutcroppingExtent, kind: OutcroppingKind, noise: NoiseParams) -> Self {
		Self { extent, kind, noise, placements: OnceLock::new() }
	}

	pub fn grown_placements(&self) -> Option<&[RockPlacement]> {
		self.placements.get().map(Vec::as_slice)
	}

	pub fn ensure_grown(&self, world: &impl TerrainDetailWorldSample) -> &[RockPlacement] {
		self.placements
			.get_or_init(|| self.kind.populate(self.extent, self.noise, world))
	}

	/// `None` means this call grew and the caller should wait for the next slot.
	pub fn placements_ready_to_present(
		&self,
		world: &impl TerrainDetailWorldSample,
	) -> Option<&[RockPlacement]> {
		if self.placements.get().is_some() {
			return self.grown_placements();
		}
		self.ensure_grown(world);
		None
	}
}

fn cell_hash(extent: OutcroppingExtent, noise: NoiseParams) -> SeededHash {
	let (ix, iz) = OutcroppingExtent::cell_index_containing(extent.center());
	let mixed = (noise.seed as u32)
		.wrapping_add((ix as u32).wrapping_mul(73856093))
		.wrapping_add((iz as u32).wrapping_mul(19349663));
	SeededHash::new(mixed)
}

fn hashed_count_in(hash: &SeededHash, salt: u32, range: std::ops::RangeInclusive<u32>) -> u32 {
	let lo = *range.start();
	let hi = *range.end();
	let span = hi.saturating_sub(lo);
	lo + ((hash.unit(salt) * (span as f32 + 1.0 - 1e-6)) as u32).min(span)
}

fn hashed_count(hash: &SeededHash, salt: u32) -> u32 {
	hashed_count_in(hash, salt, CLUSTER_COUNT)
}

fn sit(xz: Vec2, scale: f32, embed_frac: f32, world: &impl TerrainDetailWorldSample) -> Vec3 {
	let height = world.height_at(Vec3::new(xz.x, 0.0, xz.y));
	Vec3::new(xz.x, height - scale * embed_frac, xz.y)
}

fn polar_offset(hash: &SeededHash, salt: u32, radius: f32) -> Vec2 {
	let angle = hash.unit(salt) * TAU;
	let reach = hash.unit(salt.wrapping_add(1)).sqrt() * radius;
	Vec2::new(angle.cos(), angle.sin()) * reach
}

fn populate_pile(
	extent: OutcroppingExtent,
	hash: &SeededHash,
	world: &impl TerrainDetailWorldSample,
) -> Vec<RockPlacement> {
	let count = hashed_count(hash, 1);
	let center = Vec2::new(extent.center().x, extent.center().z);
	(0..count)
		.map(|i| {
			let salt = 10 + i * 4;
			let xz = center + polar_offset(hash, salt, 9.0);
			let scale = 3.4 + hash.unit(salt.wrapping_add(2)) * 1.2;
			RockPlacement {
				component: RockComponent::RoundRock,
				translation: sit(xz, scale, 0.18, world),
				yaw: hash.unit(salt.wrapping_add(3)) * TAU,
				scale,
			}
		})
		.collect()
}

fn populate_patch(
	extent: OutcroppingExtent,
	hash: &SeededHash,
	world: &impl TerrainDetailWorldSample,
) -> Vec<RockPlacement> {
	let count = hashed_count(hash, 2);
	let center = Vec2::new(extent.center().x, extent.center().z);
	(0..count)
		.map(|i| {
			let salt = 80 + i * 5;
			let xz = center + polar_offset(hash, salt, 16.0);
			let scale = 3.0 + hash.unit(salt.wrapping_add(2)) * 7.0;
			let component = if hash.unit(salt.wrapping_add(3)) < 0.55 {
				RockComponent::RoundRock
			} else {
				RockComponent::RockKnob
			};
			RockPlacement {
				component,
				translation: sit(xz, scale, 0.38, world),
				yaw: hash.unit(salt.wrapping_add(4)) * TAU,
				scale,
			}
		})
		.collect()
}

fn populate_crag(
	extent: OutcroppingExtent,
	hash: &SeededHash,
	world: &impl TerrainDetailWorldSample,
) -> Vec<RockPlacement> {
	let min = extent.min();
	let max = extent.max();
	let bounds = Bounds2::from_xz(min.x, min.z, max.x, max.z);
	let start = bounds.project(Vec2::new(
		min.x + 4.0 + hash.unit(3) * (DEFAULT_OUTCROPPING_EXTENT_XZ - 8.0),
		min.z + 4.0 + hash.unit(4) * (DEFAULT_OUTCROPPING_EXTENT_XZ - 8.0),
	));
	let end = bounds.project(Vec2::new(
		min.x + 4.0 + hash.unit(5) * (DEFAULT_OUTCROPPING_EXTENT_XZ - 8.0),
		min.z + 4.0 + hash.unit(6) * (DEFAULT_OUTCROPPING_EXTENT_XZ - 8.0),
	));
	let config = HysteresisConfig {
		max_segments: 8,
		step_len: 4.0,
		snap_radius: 3.0,
		connect_radius: 8.0,
		min_progress: 0.5,
		hysteresis: 0.55,
		max_turn_radians: 0.7,
	};
	let graph = HysteresisGraph::degree1(bounds, hash.seed, start, end, &config);
	let path = graph.primary_polyline();
	let count = hashed_count(hash, 7).min(path.len().max(1) as u32);
	(0..count)
		.map(|i| {
			let salt = 200 + i * 6;
			let along = if path.is_empty() {
				Vec2::new(extent.center().x, extent.center().z)
			} else {
				path[(i as usize).min(path.len() - 1)]
			};
			let lateral = (hash.unit(salt) * 2.0 - 1.0) * 2.4;
			let tangent = if path.len() >= 2 {
				let next = path[(i as usize + 1).min(path.len() - 1)];
				let prev = path[i as usize];
				let delta = next - prev;
				Vec2::new(-delta.y, delta.x).normalize_or_zero()
			} else {
				Vec2::X
			};
			let xz = along + tangent * lateral;
			let mix = hash.unit(salt.wrapping_add(1));
			let component = if mix < 0.62 {
				RockComponent::SharpRock
			} else if mix < 0.84 {
				RockComponent::RoundRock
			} else {
				RockComponent::RockKnob
			};
			let scale = 2.2 + hash.unit(salt.wrapping_add(2)) * 3.4;
			RockPlacement {
				component,
				translation: sit(xz, scale, 0.42, world),
				yaw: hash.unit(salt.wrapping_add(3)) * TAU,
				scale,
			}
		})
		.collect()
}

fn populate_loner(
	extent: OutcroppingExtent,
	hash: &SeededHash,
	world: &impl TerrainDetailWorldSample,
) -> Vec<RockPlacement> {
	let center = Vec2::new(extent.center().x, extent.center().z);
	let xz = center + polar_offset(hash, 9, 6.0);
	let component = mixed_component(hash, 11);
	let scale = 2.5 + hash.unit(12) * 5.5;
	vec![RockPlacement {
		component,
		translation: sit(xz, scale, 0.22, world),
		yaw: hash.unit(13) * TAU,
		scale,
	}]
}

fn mixed_component(hash: &SeededHash, salt: u32) -> RockComponent {
	let pick = hash.unit(salt);
	if pick < 0.40 {
		RockComponent::RoundRock
	} else if pick < 0.75 {
		RockComponent::RockKnob
	} else {
		RockComponent::SharpRock
	}
}

fn populate_sparse_mix(
	extent: OutcroppingExtent,
	hash: &SeededHash,
	world: &impl TerrainDetailWorldSample,
) -> Vec<RockPlacement> {
	let count = hashed_count_in(hash, 3, SPARSE_COUNT);
	let center = Vec2::new(extent.center().x, extent.center().z);
	(0..count)
		.map(|i| {
			let salt = 300 + i * 5;
			let xz = center + polar_offset(hash, salt, 18.0);
			let scale = 2.0 + hash.unit(salt.wrapping_add(2)) * 3.5;
			RockPlacement {
				component: mixed_component(hash, salt.wrapping_add(3)),
				translation: sit(xz, scale, 0.22, world),
				yaw: hash.unit(salt.wrapping_add(4)) * TAU,
				scale,
			}
		})
		.collect()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::FlatTerrainDetailSample;
	use anyhow::Result;
	use bevy_math::Vec3Swizzles;

	fn extent() -> OutcroppingExtent {
		OutcroppingExtent::from_cell_index(2, -1)
	}

	#[test]
	fn kebab_round_trips_every_outcropping() -> Result<()> {
		for kind in OutcroppingKind::ALL {
			assert_eq!(OutcroppingKind::from_kebab(kind.as_kebab()), Some(kind));
		}
		assert!(OutcroppingKind::from_kebab("not-an-outcropping").is_none());
		Ok(())
	}

	#[test]
	fn pile_and_patch_hash_count_in_range() -> Result<()> {
		let world = FlatTerrainDetailSample::default();
		let noise = NoiseParams::default();
		for kind in [OutcroppingKind::RockPile, OutcroppingKind::BoulderPatch] {
			let n = kind.populate(extent(), noise, &world).len() as u32;
			assert!(CLUSTER_COUNT.contains(&n), "{kind:?} count {n}");
		}
		Ok(())
	}

	#[test]
	fn loner_is_one_rock() -> Result<()> {
		let rocks = OutcroppingKind::Loner.populate(
			extent(),
			NoiseParams::default(),
			&FlatTerrainDetailSample::default(),
		);
		assert_eq!(rocks.len(), 1);
		Ok(())
	}

	#[test]
	fn sparse_mix_is_a_few_mixed_rocks() -> Result<()> {
		let rocks = OutcroppingKind::SparseMix.populate(
			extent(),
			NoiseParams::default(),
			&FlatTerrainDetailSample::default(),
		);
		let n = rocks.len() as u32;
		assert!(SPARSE_COUNT.contains(&n), "sparse mix count {n}");
		Ok(())
	}

	#[test]
	fn crag_is_mostly_sharp_and_stays_near_the_cell() -> Result<()> {
		let extent = extent();
		let rocks = OutcroppingKind::Crag.populate(
			extent,
			NoiseParams::default(),
			&FlatTerrainDetailSample::default(),
		);
		assert!(!rocks.is_empty());
		let sharp = rocks.iter().filter(|r| r.component == RockComponent::SharpRock).count();
		assert!(sharp * 2 >= rocks.len(), "crag should bias SharpRock");
		for rock in &rocks {
			let d = (rock.translation.xz() - extent.center().xz()).length();
			assert!(d < 30.0, "crag spilled too far: {d}");
		}
		Ok(())
	}

	#[test]
	fn boulder_patch_scales_and_sinks() -> Result<()> {
		let rocks = OutcroppingKind::BoulderPatch.populate(
			extent(),
			NoiseParams::default(),
			&FlatTerrainDetailSample { elevation: 10.0 },
		);
		for rock in &rocks {
			assert!(rock.scale >= 3.0 - 1e-3 && rock.scale <= 10.0 + 1e-3);
			assert!(rock.translation.y < 10.0);
			assert!(
				rock.component == RockComponent::RoundRock
					|| rock.component == RockComponent::RockKnob
			);
		}
		Ok(())
	}

	#[test]
	fn populate_is_deterministic() -> Result<()> {
		let world = FlatTerrainDetailSample::default();
		let noise = NoiseParams::from_scalar(9.0, 0.01, 1.0, 1);
		let a = OutcroppingKind::RockPile.populate(extent(), noise, &world);
		let b = OutcroppingKind::RockPile.populate(extent(), noise, &world);
		assert_eq!(a, b);
		Ok(())
	}

	#[test]
	fn ensure_grown_is_once() -> Result<()> {
		let outcropping =
			TerrainOutcropping::selected(extent(), OutcroppingKind::Loner, NoiseParams::default());
		assert!(outcropping.grown_placements().is_none());
		let first = outcropping.ensure_grown(&FlatTerrainDetailSample::default()).len();
		assert_eq!(first, 1);
		assert_eq!(outcropping.ensure_grown(&FlatTerrainDetailSample::default()).len(), first);
		Ok(())
	}

	#[test]
	fn ready_grows_then_returns() -> Result<()> {
		let outcropping =
			TerrainOutcropping::selected(extent(), OutcroppingKind::Loner, NoiseParams::default());
		let world = FlatTerrainDetailSample::default();
		assert!(outcropping.placements_ready_to_present(&world).is_none());
		let ready = outcropping
			.placements_ready_to_present(&world)
			.ok_or_else(|| anyhow::anyhow!("ready"))?;
		assert_eq!(ready.len(), 1);
		Ok(())
	}
}
