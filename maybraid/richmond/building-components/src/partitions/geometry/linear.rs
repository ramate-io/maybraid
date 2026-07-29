//! Full-height linear partition geometry and LOD policy.

use bevy::prelude::Transform;
use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::gen::LodSceneLevel;
use lod::lod_ref::LodRef;

use crate::partitions::geometry::PartitionTile;
use crate::partitions::host::{posed_mesh_tier, warm_mesh_host};
use crate::partitions::mesh_set::PartitionMeshSet;
use crate::partitions::probe::{PartitionLodBand, PartitionLodProbe};
use crate::placed::{Placed, Placement};

/// `distance / max_extent` out to this → High.
pub const LINEAR_HIGH_FACTOR: f32 = 5.0;
/// Out to this → Medium.
pub const LINEAR_MEDIUM_FACTOR: f32 = 20.0;
/// Out to this → Low; else UltraLow.
pub const LINEAR_LOW_FACTOR: f32 = 500.0;

/// Default linear thickness scale (\(0.15\) world / \(0.2\) kit half-extent).
pub const DEFAULT_THICK: f32 = 0.15 / 0.2;

/// Suggested full tile width along local \(X\) (matches unscaled kit \(X \in [-1, 1]\)).
pub const DEFAULT_TILE_WIDTH: f32 = 2.0;

/// How many tiles fit a length given a suggested width (roofs-style).
///
/// \(n = \mathrm{round}(\texttt{length}/\texttt{tile\_width})\), at least 1. Callers use
/// \(\texttt{length}/n\) as the actual tile width so tiles span the length exactly.
pub fn fitted_tile_count(length: f32, tile_width: f32) -> u32 {
	let tw = tile_width.max(1e-4);
	((length / tw).round() as i32).max(1) as u32
}

/// Unit linear partition (\(X \in [-1, 1]\), \(Y \in [0, 1]\), \(Z \in [-0.2, 0.2]\)).
///
/// When [`Self::length`] is `None`, tessellation emits one unit kit and placement
/// supplies world size (legacy). When `Some`, tessellation subdivides along local
/// \(X\) with [`Self::tile_width`] fitted so \(n\) tiles span the length exactly;
/// placement should leave \(X\) scale at \(1\).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearPartition {
	/// World-space span along local \(X\). `None` → single unit kit.
	pub length: Option<f32>,
	/// Suggested tile full-width along \(X\); fitted when [`Self::length`] is set.
	pub tile_width: f32,
}

impl Default for LinearPartition {
	fn default() -> Self {
		Self {
			length: None,
			tile_width: DEFAULT_TILE_WIDTH,
		}
	}
}

impl LinearPartition {
	pub fn new() -> Self {
		Self::default()
	}

	/// Continuous span tessellated with a suggested tile width.
	pub fn spanning(length: f32, tile_width: f32) -> Self {
		Self {
			length: Some(length.max(0.0)),
			tile_width: tile_width.max(1e-4),
		}
	}

	pub fn with_length(mut self, length: f32) -> Self {
		self.length = Some(length.max(0.0));
		self
	}

	pub fn with_tile_width(mut self, tile_width: f32) -> Self {
		self.tile_width = tile_width.max(1e-4);
		self
	}

	/// Expand into posed linear tiles (identity parent).
	pub fn tiles(self) -> Vec<Placed<PartitionTile>> {
		let Some(length) = self.length.filter(|l| *l > 1e-6) else {
			return vec![Placed::at_origin(PartitionTile::Linear)];
		};
		let n = fitted_tile_count(length, self.tile_width);
		let width = length / n as f32;
		let half = width * 0.5;
		let mut out = Vec::with_capacity(n as usize);
		for i in 0..n {
			let x = -length * 0.5 + half + i as f32 * width;
			out.push(Placed {
				geom: PartitionTile::Linear,
				placement: Placement::new(Vec3::new(x, 0.0, 0.0), 0.0)
					.with_scale(Vec3::new(half, 1.0, 1.0)),
			});
		}
		out
	}
}

/// LOD banding / posed mesh helpers for linear (and polyline-parent) partitions.
pub struct LinearLod;

impl LinearLod {
	pub fn band_from_distance_factor(factor: f32) -> PartitionLodBand {
		PartitionLodBand::from_distance_factor(factor)
	}

	pub fn level_for_placement(placement: &Placement, viewer: &Transform) -> LodSceneLevel {
		PartitionLodProbe::from_placement(placement).level_for(viewer)
	}

	pub fn posed_tier(
		meshes: PartitionMeshSet,
		transform: Transform,
		level: LodSceneLevel,
	) -> impl Scene + 'static {
		posed_mesh_tier(meshes, transform, level)
	}

	pub fn posed_host(
		meshes: PartitionMeshSet,
		transform: Transform,
		level: LodSceneLevel,
		probe: PartitionLodProbe,
	) -> impl Scene + 'static {
		warm_mesh_host(meshes, transform, level, probe)
	}

	pub fn leaf_host(meshes: PartitionMeshSet, lod_ref: &LodRef) -> impl Scene + 'static {
		let probe = PartitionLodProbe::from_aabb(lod_ref.bounds);
		let level = probe.level_for(lod_ref.current_transform);
		Self::posed_host(meshes, Transform::IDENTITY, level, probe)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn unit_linear_is_single_tile() -> anyhow::Result<()> {
		assert_eq!(LinearPartition::default().tiles().len(), 1);
		Ok(())
	}

	#[test]
	fn spanning_fits_tiles_to_length() -> anyhow::Result<()> {
		let tiles = LinearPartition::spanning(3.0, 1.0).tiles();
		assert_eq!(tiles.len(), 3);
		assert!((tiles[0].scale().x - 0.5).abs() < 1e-4);
		assert!((tiles[0].translation().x - (-1.0)).abs() < 1e-4);
		assert!((tiles[1].translation().x).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn tile_width_suggestion_rounds_and_stretches() -> anyhow::Result<()> {
		let tiles = LinearPartition::spanning(2.4, 1.0).tiles();
		// round(2.4/1)=2 → width 1.2 → half-scale 0.6
		assert_eq!(tiles.len(), 2);
		assert!((tiles[0].scale().x - 0.6).abs() < 1e-4);
		Ok(())
	}
}
