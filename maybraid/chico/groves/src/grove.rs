//! Common grove selection container ([RFC-183 §4.7], [#192](https://github.com/ramate-io/maybraid/issues/192)).
//!
//! v1 exposes direct assembly constructors; [`gimme_gen::CellGenerator`] integration follows once
//! the spatial index lands in gimme.

mod biases;
mod bucket;
mod buckets_macro;
mod cell_grove;
mod constraints;
mod distribution;
mod outcome;
mod palette;
mod params;
mod frontend;
mod placement;
mod terrain;
mod variant_weights;

#[cfg(feature = "render")]
mod vec3_args;

#[cfg(feature = "render")]
mod render_item;

pub use biases::ForestGroveBiases;
pub use bucket::Bucket;
pub use cell_grove::CellGrove;
pub use constraints::PlacementConstraints;
pub use distribution::{
	GroveBucket, GroveDistribution, GroveDistributionBuilder, PreparedGroveDistribution,
};
pub use frontend::GroveFrontend;
pub use outcome::GroveCellOutcome;
pub use palette::{PaletteColor, PaletteMix, PaletteSlot, WithPaletteMix};
pub use params::{
	biased_sample, sample_cell_params, GroveNoiseConfig, GroveParamRanges, SampledCellParams,
};
pub use placement::{candidate_position, cell_origin};
pub use terrain::{FlatTerrainSample, TerrainSample};
pub use variant_weights::{parse_variant_weights, VariantWeightOverrides};

#[cfg(feature = "render")]
pub use vec3_args::parse_vec3_csv;

#[cfg(feature = "render")]
pub use render_item::{GrovePlacedCell, GroveRenderHelper, GroveRenderRule};

use bevy_math::Vec3;
use gimme_gen::Cell;

/// Assembled grove definition with forest biases, shared noise, and a pre-built distribution.
pub struct Grove<G: CellGrove> {
	definition: G,
	biases: ForestGroveBiases,
	noise: GroveNoiseConfig,
	prepared: PreparedGroveDistribution<G::Variant>,
}

impl<G: CellGrove> Grove<G> {
	/// Assemble a grove and perturb bucket weights once at `perturbation_origin`.
	pub fn assemble(
		definition: G,
		biases: ForestGroveBiases,
		noise: GroveNoiseConfig,
		perturbation_origin: Vec3,
	) -> Self
	where
		G::Variant: Clone,
	{
		let prepared = definition
			.distribution()
			.clone()
			.builder()
			.build(&biases, &noise, perturbation_origin);
		Self { definition, biases, noise, prepared }
	}

	pub fn definition(&self) -> &G {
		&self.definition
	}

	pub fn biases(&self) -> &ForestGroveBiases {
		&self.biases
	}

	pub fn noise(&self) -> &GroveNoiseConfig {
		&self.noise
	}

	pub fn param_ranges(&self) -> GroveParamRanges {
		self.definition.param_ranges()
	}

	pub fn distribution(&self) -> &GroveDistribution<G::Variant> {
		self.definition.distribution()
	}

	pub fn prepared(&self) -> &PreparedGroveDistribution<G::Variant> {
		&self.prepared
	}

	/// Run bucket throw, first-fit, and constraint validation for one cell.
	pub fn select_cell(
		&self,
		cell: &Cell,
		terrain: &impl TerrainSample,
	) -> GroveCellOutcome<G::Variant> {
		self.prepared.select_cell(
			cell,
			&self.definition.param_ranges(),
			&self.biases,
			&self.noise,
			terrain,
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy_math::bounding::Aabb3d;
	use gimme_gen::Cell;
	use procedural_common::UnitRange;

	struct MockGrove {
		ranges: GroveParamRanges,
		distribution: GroveDistribution<&'static str>,
	}

	impl CellGrove for MockGrove {
		type Variant = &'static str;

		fn param_ranges(&self) -> GroveParamRanges {
			self.ranges
		}

		fn distribution(&self) -> &GroveDistribution<Self::Variant> {
			&self.distribution
		}
	}

	struct FlatTerrain {
		elevation: f32,
		steepness: f32,
	}

	impl TerrainSample for FlatTerrain {
		fn elevation_at(&self, _position: bevy_math::Vec3) -> f32 {
			self.elevation
		}

		fn steepness_at(&self, _position: bevy_math::Vec3) -> f32 {
			self.steepness
		}
	}

	#[test]
	fn assemble_selects_via_direct_constructor() -> Result<()> {
		let mut distribution = GroveDistribution::new();
		distribution.push(GroveBucket {
			weight: 1.0,
			constraints: PlacementConstraints::UNCONSTRAINED,
			item: Some("tree"),
		});
		let grove = Grove::assemble(
			MockGrove {
				ranges: GroveParamRanges::new(
					UnitRange::new(8.0, 16.0),
					UnitRange::new(0.8, 1.2),
					UnitRange::new(0.1, 0.5),
					UnitRange::new(0.0, 0.2),
					UnitRange::new(0.02, 0.12),
					UnitRange::new(0.01, 0.03),
				),
				distribution,
			},
			ForestGroveBiases::default(),
			GroveNoiseConfig::default(),
			Vec3::ZERO,
		);
		let cell = Cell(Aabb3d::from_min_max(
			bevy_math::Vec3::ZERO,
			bevy_math::Vec3::new(10.0, 1.0, 10.0),
		));
		let outcome = grove.select_cell(&cell, &FlatTerrain { elevation: 0.4, steepness: 0.1 });
		assert!(matches!(outcome, GroveCellOutcome::Placed { variant: "tree", .. }));
		Ok(())
	}
}
