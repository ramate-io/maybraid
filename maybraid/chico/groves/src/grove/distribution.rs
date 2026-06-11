//! Grove distribution and per-cell selection pipeline ([RFC-183 3.4.2]).

use bevy_math::Vec3;
use procedural_common::{
	perturb_weights, BucketThrow, FirstFitIndices, NoiseConfig, MIN_BUCKET_WEIGHT,
};

use super::{
	biases::ForestGroveBiases,
	constraints::PlacementConstraints,
	outcome::GroveCellOutcome,
	params::{GroveNoiseConfig, SampledCellParams},
	terrain::TerrainSample,
};

/// One ordered bucket in a grove distribution.
#[derive(Debug, Clone, PartialEq)]
pub struct GroveBucket<V> {
	pub weight: f32,
	pub constraints: PlacementConstraints,
	pub item: Option<V>,
}

impl<V> GroveBucket<V> {
	/// Whether this bucket may occupy `position` on `terrain`. Explicit `None` buckets always pass.
	pub fn valid_at(&self, position: Vec3, terrain: &impl TerrainSample) -> bool {
		if self.item.is_none() {
			return true;
		}
		self.constraints
			.allows(terrain.elevation_at(position), terrain.steepness_at(position))
	}
}

/// Ordered weighted grove variants with per-bucket constraints.
#[derive(Debug, Clone, PartialEq)]
pub struct GroveDistribution<V> {
	pub buckets: Vec<GroveBucket<V>>,
	/// Base perturbation strength before forest bias scaling.
	pub base_bucket_perturbation_strength: f32,
}

impl<V> Default for GroveDistribution<V> {
	fn default() -> Self {
		Self { buckets: Vec::new(), base_bucket_perturbation_strength: 0.0 }
	}
}

impl<V> GroveDistribution<V> {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn with_perturbation_strength(mut self, strength: f32) -> Self {
		self.base_bucket_perturbation_strength = strength;
		self
	}

	pub fn push(&mut self, bucket: GroveBucket<V>) {
		self.buckets.push(bucket);
	}

	pub fn len(&self) -> usize {
		self.buckets.len()
	}

	pub fn is_empty(&self) -> bool {
		self.buckets.is_empty()
	}

	/// Perturb bucket weights at `perturbation_origin`, then freeze the throw for many cell draws.
	pub fn prepare(
		self,
		biases: &ForestGroveBiases,
		noise: &GroveNoiseConfig,
		perturbation_origin: Vec3,
	) -> PreparedGroveDistribution<V> {
		let (bucket_throw, throw_bucket_indices) =
			build_perturbed_bucket_throw(&self, biases, noise, perturbation_origin);
		PreparedGroveDistribution { buckets: self.buckets, bucket_throw, throw_bucket_indices }
	}
}

/// Grove distribution with bucket weights perturbed once for a forest/grove instance.
#[derive(Debug, Clone)]
pub struct PreparedGroveDistribution<V> {
	buckets: Vec<GroveBucket<V>>,
	bucket_throw: BucketThrow,
	/// Maps compressed [`BucketThrow`] indices to [`Self::buckets`] indices (zero-weight buckets omitted).
	throw_bucket_indices: Vec<usize>,
}

impl<V> PreparedGroveDistribution<V> {
	pub fn len(&self) -> usize {
		self.buckets.len()
	}

	pub fn is_empty(&self) -> bool {
		self.buckets.is_empty()
	}

	/// Select at an explicit candidate point (used when placement is precomputed).
	pub fn select_at(
		&self,
		position: Vec3,
		sampled: SampledCellParams,
		noise: &GroveNoiseConfig,
		terrain: &impl TerrainSample,
	) -> GroveCellOutcome<V>
	where
		V: Clone,
	{
		if self.is_empty() {
			return GroveCellOutcome::Rejected { position };
		}

		let start = self.start_index(noise, position);

		self.select_at_with_start(start, position, sampled, terrain)
	}

	/// Select starting from a known bucket index (used by tests and debugging).
	pub fn select_at_with_start(
		&self,
		start: usize,
		position: Vec3,
		sampled: SampledCellParams,
		terrain: &impl TerrainSample,
	) -> GroveCellOutcome<V>
	where
		V: Clone,
	{
		if self.is_empty() {
			return GroveCellOutcome::Rejected { position };
		}

		for index in FirstFitIndices::new(self.buckets.len(), start) {
			let bucket = &self.buckets[index];
			if !bucket.valid_at(position, terrain) {
				continue;
			}
			return match &bucket.item {
				Some(variant) => GroveCellOutcome::Placed {
					variant: variant.clone(),
					position,
					scale: sampled.scale,
				},
				None => GroveCellOutcome::Empty { position },
			};
		}

		GroveCellOutcome::Rejected { position }
	}

	fn start_index(&self, noise: &GroveNoiseConfig, position: Vec3) -> usize {
		let selection_noise = bucket_selection_noise(noise, position);
		let throw = selection_noise * self.bucket_throw.total_weight() * 0.5;
		let throw_index = self.bucket_throw.select(throw).unwrap_or(0);
		self.throw_bucket_indices.get(throw_index).copied().unwrap_or(0)
	}
}

fn build_perturbed_bucket_throw<V>(
	distribution: &GroveDistribution<V>,
	biases: &ForestGroveBiases,
	noise: &GroveNoiseConfig,
	perturbation_origin: Vec3,
) -> (BucketThrow, Vec<usize>) {
	let mut base = BucketThrow::new();
	let mut throw_bucket_indices = Vec::new();
	for (index, bucket) in distribution.buckets.iter().enumerate() {
		if base.add(bucket.weight) {
			throw_bucket_indices.push(index);
		}
	}

	let strength =
		distribution.base_bucket_perturbation_strength * (1.0 + biases.bucket_perturbation_bias);
	let total = base.total_weight();
	if strength.abs() <= f32::EPSILON || base.is_empty() {
		return (base.with_mean_anchor(bucket_mean_shift(biases, total)), throw_bucket_indices);
	}

	let n = NoiseConfig::new(noise.base);
	let bucket_noises: Vec<f32> = throw_bucket_indices
		.iter()
		.map(|&index| {
			n.sample_3d_world(perturbation_origin + Vec3::new(20.0 + index as f32, 0.0, 0.0))
		})
		.collect();
	let perturbed = perturb_weights(&base, strength, &bucket_noises, MIN_BUCKET_WEIGHT);
	let total = perturbed.total_weight();
	(perturbed.with_mean_anchor(bucket_mean_shift(biases, total)), throw_bucket_indices)
}

fn bucket_mean_shift(biases: &ForestGroveBiases, total_weight: f32) -> f32 {
	biases.bucket_mean_shift * total_weight
}

fn bucket_selection_noise(noise: &GroveNoiseConfig, position: Vec3) -> f32 {
	NoiseConfig::new(noise.base).sample_3d_world(position + Vec3::new(30.0, 0.0, 0.0))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::params::{GroveNoiseConfig, GrovePlacementRanges};
	use anyhow::Result;
	use bevy_math::bounding::Aabb3d;
	use gimme_gen::Cell;
	use procedural_common::UnitRange;

	struct FlatTerrain {
		elevation: f32,
		steepness: f32,
	}

	impl TerrainSample for FlatTerrain {
		fn elevation_at(&self, _position: Vec3) -> f32 {
			self.elevation
		}

		fn steepness_at(&self, _position: Vec3) -> f32 {
			self.steepness
		}
	}

	fn test_ranges() -> GrovePlacementRanges {
		GrovePlacementRanges::new(
			UnitRange::new(0.8, 1.2),
			UnitRange::new(-0.2, 0.2),
			UnitRange::new(0.02, 0.12),
			UnitRange::new(0.01, 0.03),
		)
	}

	fn test_cell() -> Cell {
		Cell(Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 1.0, 10.0)))
	}

	fn prepared_dist<V>(dist: GroveDistribution<V>) -> PreparedGroveDistribution<V> {
		dist.prepare(&ForestGroveBiases::default(), &GroveNoiseConfig::default(), Vec3::ZERO)
	}

	#[test]
	fn selects_none_bucket() -> Result<()> {
		let mut dist: GroveDistribution<()> = GroveDistribution::new();
		dist.push(GroveBucket {
			weight: 1.0,
			constraints: PlacementConstraints::UNCONSTRAINED,
			item: None,
		});
		let prepared = prepared_dist(dist);
		let outcome = prepared.select_at(
			Vec3::ZERO,
			test_ranges().sample_at(
				&ForestGroveBiases::default(),
				&GroveNoiseConfig::default(),
				Vec3::ZERO,
			),
			&GroveNoiseConfig::default(),
			&FlatTerrain { elevation: 0.5, steepness: 0.1 },
		);
		assert!(matches!(outcome, GroveCellOutcome::Empty { .. }));
		Ok(())
	}

	#[test]
	fn first_fit_falls_back_to_valid_variant() -> Result<()> {
		let mut dist = GroveDistribution::new();
		dist.push(GroveBucket {
			weight: 1.0,
			constraints: PlacementConstraints::new(
				UnitRange::new(0.8, 1.0),
				UnitRange::new(0.0, 0.1),
			),
			item: Some("steep_only"),
		});
		dist.push(GroveBucket {
			weight: 1.0,
			constraints: PlacementConstraints::new(
				UnitRange::new(0.0, 0.5),
				UnitRange::new(0.0, 0.5),
			),
			item: Some("flat"),
		});
		let prepared = prepared_dist(dist);
		let terrain = FlatTerrain { elevation: 0.3, steepness: 0.2 };
		let sampled = test_ranges().sample_at(
			&ForestGroveBiases { bucket_mean_shift: 0.0, ..Default::default() },
			&GroveNoiseConfig::default(),
			Vec3::new(5.0, 0.0, 5.0),
		);
		let position = sampled.position_in(&test_cell(), &0.0);
		let outcome = prepared.select_at_with_start(0, position, sampled, &terrain);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => assert_eq!(variant, "flat"),
			other => panic!("expected Placed flat, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn zero_weight_none_bucket_does_not_shadow_blade_variants() -> Result<()> {
		use crate::BraidGrassCell;
		use procedural_common::NoiseParams;

		let mut dist = BraidGrassCell::grove_distribution();
		dist.buckets[0].weight = 0.0;
		dist.buckets[1].weight = 9.0;
		let prepared = dist.prepare(
			&ForestGroveBiases::default(),
			&GroveNoiseConfig::new(NoiseParams::from_scalar(0.0, 1.0, 0.0, 1)),
			Vec3::ZERO,
		);
		let terrain = FlatTerrain { elevation: 0.4, steepness: 0.1 };
		let braid_ranges = crate::braid_grass::BraidGrassDefinition::PLACEMENT_RANGES;
		let mut cells = Vec::new();
		for x in 0..3 {
			for z in 0..3 {
				cells.push(Cell(Aabb3d::from_min_max(
					Vec3::new(x as f32 * 4.25, 0.0, z as f32 * 4.25),
					Vec3::new((x + 1) as f32 * 4.25, 1.0, (z + 1) as f32 * 4.25),
				)));
			}
		}
		let mut placed = 0usize;
		for cell in &cells {
			let noise = GroveNoiseConfig::new(NoiseParams::from_scalar(0.0, 1.0, 0.0, 1));
			let sampled = braid_ranges.sample_cell(&ForestGroveBiases::default(), &noise, cell);
			let outcome =
				prepared.select_at(sampled.position_in(cell, &0.0), sampled, &noise, &terrain);
			if matches!(outcome, GroveCellOutcome::Placed { .. }) {
				placed += 1;
			}
		}
		assert!(
			placed > 0,
			"expected blade placements when None weight is zero, got {placed} placed cells"
		);
		Ok(())
	}

	#[test]
	fn prepare_reuses_perturbed_throw_across_cells() -> Result<()> {
		let mut dist = GroveDistribution::new();
		dist.push(GroveBucket {
			weight: 1.0,
			constraints: PlacementConstraints::UNCONSTRAINED,
			item: Some("tree"),
		});
		let prepared = dist.prepare(
			&ForestGroveBiases::default(),
			&GroveNoiseConfig::default(),
			Vec3::new(100.0, 0.0, 50.0),
		);
		let terrain = FlatTerrain { elevation: 0.4, steepness: 0.1 };
		let ranges = test_ranges();
		let biases = ForestGroveBiases::default();
		let noise = GroveNoiseConfig::default();
		let sampled = ranges.sample_cell(&biases, &noise, &test_cell());
		let position = sampled.position_in(&test_cell(), &0.0);
		let a = prepared.select_at(position, sampled, &noise, &terrain);
		let b = prepared.select_at(position, sampled, &noise, &terrain);
		assert!(matches!(a, GroveCellOutcome::Placed { .. }));
		assert_eq!(a, b);
		Ok(())
	}
}
