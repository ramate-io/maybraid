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
mod extent;
mod frontend;
mod outcome;
mod palette;
mod params;
mod placement;
mod terrain;
mod variant_weights;

#[cfg(feature = "render")]
mod vec3_args;

pub use biases::ForestGroveBiases;
pub use bucket::Bucket;
pub use cell_grove::CellGrove;
pub use constraints::PlacementConstraints;
pub use distribution::{GroveBucket, GroveDistribution, PreparedGroveDistribution};
pub use extent::{GroveExtent, GroveOverspillPolicy, DEFAULT_GROVE_EXTENT_XZ};
pub use frontend::GroveFrontend;
pub use outcome::GroveCellOutcome;
pub use palette::{patch_spawned_leaf_material, PaletteColor, PaletteMix, PaletteSlot, WithPalette};
pub use params::{
	placement_noise, GroveNoiseConfig, GrovePlacementRanges, SampledCellParams,
};
pub use placement::CellXzOffset;
pub use terrain::{FlatTerrainSample, TerrainSample};
pub use variant_weights::{parse_variant_weights, VariantWeightOverrides};

#[cfg(feature = "render")]
pub use vec3_args::{parse_vec2_csv, parse_vec3_csv};

use bevy_math::{Vec2, Vec3};
use gimme_gen::Cell;

/// Assembled grove definition with forest biases, shared noise, and a pre-built distribution.
pub struct Grove<G: CellGrove> {
	definition: G,
	biases: ForestGroveBiases,
	noise: GroveNoiseConfig,
	prepared: PreparedGroveDistribution<G::Variant>,
}

/// Sampled placement for one vegetation cell before bucket selection.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GroveCellPlacement {
	/// The raw cell-center + offset point. This may lie outside the grove extent.
	pub candidate_position: Vec3,
	/// The point used for terrain checks and rendering after overspill handling.
	pub position: Vec3,
	/// Per-instance scale, foliage noise, and offset sampled for this cell.
	pub sampled: SampledCellParams,
}

/// One placed grove item ready for materialization.
#[derive(Debug, Clone, PartialEq)]
pub struct GrovePlacedCell<V> {
	pub variant: V,
	pub position: Vec3,
	pub scale: f32,
}

impl<V: Clone> GrovePlacedCell<V> {
	pub fn new(variant: V, position: Vec3, scale: f32) -> Self {
		Self { variant, position, scale }
	}
}

impl<V: Clone> From<GroveCellOutcome<V>> for Option<GrovePlacedCell<V>> {
	fn from(outcome: GroveCellOutcome<V>) -> Self {
		match outcome {
			GroveCellOutcome::Placed { variant, position, scale } => {
				Some(GrovePlacedCell { variant, position, scale })
			}
			GroveCellOutcome::Empty { .. } | GroveCellOutcome::Rejected { .. } => None,
		}
	}
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
		let prepared =
			definition.distribution().clone().prepare(&biases, &noise, perturbation_origin);
		Self { definition, biases, noise, prepared }
	}

	/// Select and place every vegetation cell in this grove LOD unit.
	///
	/// The grove extent is the union of `cells`: individual cell offsets may overspill, but
	/// ownership/culling stays bounded by the grove.
	pub fn select_placements(
		&self,
		grove_extent: &GroveExtent,
		cells: &[Cell],
		terrain: &impl TerrainSample,
	) -> Vec<GrovePlacedCell<G::Variant>>
	where
		G::Variant: Clone,
	{
		self.select_placements_with_policy(
			grove_extent,
			cells,
			terrain,
			GroveOverspillPolicy::Discard,
		)
	}

	/// Like [`Self::select_placements`], with explicit handling for candidates outside the grove.
	pub fn select_placements_with_policy(
		&self,
		grove_extent: &GroveExtent,
		cells: &[Cell],
		terrain: &impl TerrainSample,
		overspill_policy: GroveOverspillPolicy,
	) -> Vec<GrovePlacedCell<G::Variant>>
	where
		G::Variant: Clone,
	{
		cells
			.iter()
			.filter_map(|cell| {
				Option::<GrovePlacedCell<G::Variant>>::from(self.select_cell_with_policy(
					cell,
					grove_extent,
					overspill_policy,
					terrain,
				))
			})
			.collect()
	}

	/// Sample, place, validate, and choose a bucket for one vegetation cell.
	pub fn select_cell(
		&self,
		cell: &Cell,
		grove_extent: &GroveExtent,
		terrain: &impl TerrainSample,
	) -> GroveCellOutcome<G::Variant> {
		self.select_cell_with_policy(cell, grove_extent, GroveOverspillPolicy::Discard, terrain)
	}

	/// Like [`Self::select_cell`], with an explicit overspill policy when validating grove extent.
	pub fn select_cell_with_policy(
		&self,
		cell: &Cell,
		grove_extent: &GroveExtent,
		overspill_policy: GroveOverspillPolicy,
		terrain: &impl TerrainSample,
	) -> GroveCellOutcome<G::Variant> {
		let placement = match self.place_cell(cell, grove_extent, overspill_policy) {
			Ok(placement) => placement,
			Err(candidate_position) => {
				return GroveCellOutcome::Rejected { position: candidate_position }
			}
		};
		self.prepared
			.select_at(placement.position, placement.sampled, &self.noise, terrain)
	}

	/// Sample a cell and resolve its candidate point against the grove extent.
	pub fn place_cell(
		&self,
		cell: &Cell,
		grove_extent: &GroveExtent,
		overspill_policy: GroveOverspillPolicy,
	) -> Result<GroveCellPlacement, Vec3> {
		let sampled =
			self.definition.placement_ranges().sample_cell(&self.biases, &self.noise, cell);
		let candidate_position = sampled.position_in(cell);
		let Some(position) = grove_extent.resolve_xz(candidate_position, overspill_policy) else {
			return Err(candidate_position);
		};
		Ok(GroveCellPlacement { candidate_position, position, sampled })
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

	pub fn placement_ranges(&self) -> GrovePlacementRanges {
		self.definition.placement_ranges()
	}

	pub fn cell_extent_xz(&self) -> Vec2 {
		self.definition.cell_extent_xz()
	}

	pub fn distribution(&self) -> &GroveDistribution<G::Variant> {
		self.definition.distribution()
	}

	pub fn prepared(&self) -> &PreparedGroveDistribution<G::Variant> {
		&self.prepared
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
		cell_extent_xz: Vec2,
		placement: GrovePlacementRanges,
		distribution: GroveDistribution<&'static str>,
	}

	impl CellGrove for MockGrove {
		type Variant = &'static str;

		fn cell_extent_xz(&self) -> Vec2 {
			self.cell_extent_xz
		}

		fn placement_ranges(&self) -> GrovePlacementRanges {
			self.placement
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
				cell_extent_xz: Vec2::splat(10.0),
				placement: GrovePlacementRanges::new(
					UnitRange::new(0.8, 1.2),
					UnitRange::new(-0.2, 0.2),
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(10.0, 1.0, 10.0));
		let outcome =
			grove.select_cell(&cell, &extent, &FlatTerrain { elevation: 0.4, steepness: 0.1 });
		assert!(matches!(outcome, GroveCellOutcome::Placed { variant: "tree", .. }));
		Ok(())
	}

	#[test]
	fn select_cell_rejects_placement_outside_grove_extent() -> Result<()> {
		let mut distribution = GroveDistribution::new();
		distribution.push(GroveBucket {
			weight: 1.0,
			constraints: PlacementConstraints::UNCONSTRAINED,
			item: Some("tree"),
		});
		let grove = Grove::assemble(
			MockGrove {
				cell_extent_xz: Vec2::splat(10.0),
				placement: GrovePlacementRanges::new(
					UnitRange::new(1.0, 1.0),
					UnitRange::new(20.0, 20.0),
					UnitRange::new(0.1, 0.1),
					UnitRange::new(0.05, 0.05),
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
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(10.0, 1.0, 10.0));
		let outcome =
			grove.select_cell(&cell, &extent, &FlatTerrain { elevation: 0.4, steepness: 0.1 });
		assert!(matches!(outcome, GroveCellOutcome::Rejected { .. }));
		Ok(())
	}
}
