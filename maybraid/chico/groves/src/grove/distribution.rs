//! Grove distribution and per-cell selection pipeline ([RFC-183 3.4.2]).

use bevy_math::Vec3;
use gimme_gen::Cell;
use procedural_common::{perturb_weights, BucketThrow, NoiseConfig, MIN_BUCKET_WEIGHT};

use super::{
	biases::ForestGroveBiases,
	constraints::PlacementConstraints,
	outcome::GroveCellOutcome,
	params::{sample_cell_params, GroveNoiseConfig, GroveParamRanges, SampledCellParams},
	placement::candidate_position,
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
		self.constraints.allows(
			terrain.elevation_at(position),
			terrain.steepness_at(position),
		)
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

	/// Start building a reusable selection distribution for this grove instance.
	pub fn builder(self) -> GroveDistributionBuilder<V> {
		GroveDistributionBuilder { distribution: self }
	}
}

/// Builds a [`PreparedGroveDistribution`] with forest-level bucket perturbation applied once.
#[derive(Debug, Clone, PartialEq)]
pub struct GroveDistributionBuilder<V> {
	distribution: GroveDistribution<V>,
}

impl<V> GroveDistributionBuilder<V> {
	/// Perturb bucket weights at `perturbation_origin`, then freeze the throw for many cell draws.
	pub fn build(
		self,
		biases: &ForestGroveBiases,
		noise: &GroveNoiseConfig,
		perturbation_origin: Vec3,
	) -> PreparedGroveDistribution<V> {
		let bucket_throw =
			build_perturbed_bucket_throw(&self.distribution, biases, noise, perturbation_origin);
		PreparedGroveDistribution { buckets: self.distribution.buckets, bucket_throw }
	}
}

/// Grove distribution with bucket weights perturbed once for a forest/grove instance.
#[derive(Debug, Clone)]
pub struct PreparedGroveDistribution<V> {
	buckets: Vec<GroveBucket<V>>,
	bucket_throw: BucketThrow,
}

impl<V> PreparedGroveDistribution<V> {
	pub fn len(&self) -> usize {
		self.buckets.len()
	}

	pub fn is_empty(&self) -> bool {
		self.buckets.is_empty()
	}

	/// Run the selection pipeline for one grove cell using the pre-built bucket throw.
	pub fn select_cell(
		&self,
		cell: &Cell,
		ranges: &GroveParamRanges,
		biases: &ForestGroveBiases,
		noise: &GroveNoiseConfig,
		terrain: &impl TerrainSample,
	) -> GroveCellOutcome<V>
	where
		V: Clone,
	{
		let sampled = sample_cell_params(ranges, biases, noise, super::placement::cell_origin(cell));
		let position = candidate_position(cell, sampled.offset);
		self.select_at(position, sampled, noise, terrain)
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

		let selection_noise = bucket_selection_noise(noise, position);
		let throw = selection_noise * self.bucket_throw.total_weight() * 0.5;
		let start = self.bucket_throw.select(throw).unwrap_or(0);

		for index in self.bucket_throw.first_fit_from(start) {
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

		for index in self.bucket_throw.first_fit_from(start) {
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
}

fn build_perturbed_bucket_throw<V>(
	distribution: &GroveDistribution<V>,
	biases: &ForestGroveBiases,
	noise: &GroveNoiseConfig,
	perturbation_origin: Vec3,
) -> BucketThrow {
	let mut base = BucketThrow::new();
	for bucket in &distribution.buckets {
		base.add(bucket.weight);
	}

	let strength =
		distribution.base_bucket_perturbation_strength * (1.0 + biases.bucket_perturbation_bias);
	let total = base.total_weight();
	if strength.abs() <= f32::EPSILON || base.is_empty() {
		return base.with_mean_anchor(bucket_mean_shift(biases, total));
	}

	let n = NoiseConfig::new(noise.base);
	let bucket_noises: Vec<f32> = (0..distribution.buckets.len())
		.map(|index| {
			n.sample_3d_world(perturbation_origin + Vec3::new(20.0 + index as f32, 0.0, 0.0))
		})
		.collect();
	let perturbed = perturb_weights(&base, strength, &bucket_noises, MIN_BUCKET_WEIGHT);
	let total = perturbed.total_weight();
	perturbed.with_mean_anchor(bucket_mean_shift(biases, total))
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
	use crate::grove::params::GroveNoiseConfig;
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

	fn test_ranges() -> GroveParamRanges {
		GroveParamRanges::new(
			UnitRange::new(8.0, 16.0),
			UnitRange::new(0.8, 1.2),
			UnitRange::new(0.1, 0.5),
			UnitRange::new(0.0, 0.2),
			UnitRange::new(0.02, 0.12),
			UnitRange::new(0.01, 0.03),
		)
	}

	fn test_cell() -> Cell {
		Cell(Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(10.0, 1.0, 10.0)))
	}

	fn prepared_dist<V>(dist: GroveDistribution<V>) -> PreparedGroveDistribution<V> {
		dist.builder().build(
			&ForestGroveBiases::default(),
			&GroveNoiseConfig::default(),
			Vec3::ZERO,
		)
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
		let outcome = prepared.select_cell(
			&test_cell(),
			&test_ranges(),
			&ForestGroveBiases::default(),
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
			constraints: PlacementConstraints::new(UnitRange::new(0.8, 1.0), UnitRange::new(0.0, 0.1)),
			item: Some("steep_only"),
		});
		dist.push(GroveBucket {
			weight: 1.0,
			constraints: PlacementConstraints::new(UnitRange::new(0.0, 0.5), UnitRange::new(0.0, 0.5)),
			item: Some("flat"),
		});
		let prepared = prepared_dist(dist);
		let terrain = FlatTerrain { elevation: 0.3, steepness: 0.2 };
		let sampled = sample_cell_params(
			&test_ranges(),
			&ForestGroveBiases { bucket_mean_shift: 0.0, ..Default::default() },
			&GroveNoiseConfig::default(),
			Vec3::new(5.0, 0.0, 5.0),
		);
		let position = candidate_position(&test_cell(), sampled.offset);
		let outcome = prepared.select_at_with_start(0, position, sampled, &terrain);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => assert_eq!(variant, "flat"),
			other => panic!("expected Placed flat, got {other:?}"),
		}
		Ok(())
	}

	#[test]
	fn builder_reuses_perturbed_throw_across_cells() -> Result<()> {
		let mut dist = GroveDistribution::new();
		dist.push(GroveBucket {
			weight: 1.0,
			constraints: PlacementConstraints::UNCONSTRAINED,
			item: Some("tree"),
		});
		let prepared = dist.builder().build(
			&ForestGroveBiases::default(),
			&GroveNoiseConfig::default(),
			Vec3::new(100.0, 0.0, 50.0),
		);
		let terrain = FlatTerrain { elevation: 0.4, steepness: 0.1 };
		let ranges = test_ranges();
		let biases = ForestGroveBiases::default();
		let noise = GroveNoiseConfig::default();
		let a = prepared.select_cell(&test_cell(), &ranges, &biases, &noise, &terrain);
		let b = prepared.select_cell(&test_cell(), &ranges, &biases, &noise, &terrain);
		assert!(matches!(a, GroveCellOutcome::Placed { .. }));
		assert_eq!(a, b);
		Ok(())
	}
}
