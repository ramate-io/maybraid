//! Type-erased [`GroveRecipe`](chico_groves::GroveRecipe) for [`ForestGroveKind`].
//!
//! Selection uses the presenting tile's cells. Grow only the placements that won.

use bevy_math::Vec2;
use chico_groves::{
	AlpineParams, AridConiferSaplingParams, BraidGrassParams, BushScrubParams,
	ChristmasTaigaParams, CommonTuftsParams, ConiferMassivesParams, ConiferSaplingParams,
	DateGroveParams, DrylandParams, ForlornSavannaParams, GoettingenFollowParams, GroveExtent,
	GroveFrontend, GroveRecipe, GroveWorldSample, HighBushParams, JerrysChaparralParams,
	JungleLowerMassivesParams, JungleMassivesParams, LeewardParams, LevantineScrubParams,
	LowBushParams, MonsterGrassParams, OrchardParams, PalmShadeParams, RiparianGeneralParams,
	RiparianMixParams, RiverineGreenParams, RollingOaksParams, ShamanhomeParams,
	SpottyBushesParams, StorytellersParams, StrangeOasisParams, TallGrassParams,
	TemperateLowerMassivesParams, TemperateMassivesParams, TradeWindsParams, TropicalThicketParams,
	TropicalTuftsParams, TropicalUndergrowthParams, UnendingJungleParams, VineyardParams,
	WanderingAcaciaParams, WildGrassParams,
};
use gimme_gen::Cell;

use crate::{ForestGroveKind, ForestGroveTile};

macro_rules! impl_kind_recipe {
	($($Kind:ident => $mod:ident, $Params:ident),+ $(,)?) => {
		impl ForestGroveKind {
			/// Authored planting-cell span for this grove.
			pub fn cell_extent_xz(self) -> Vec2 {
				match self {
					$(Self::$Kind => chico_groves::$mod::definition().cell_extent_xz,)+
				}
			}

			/// Grow the full tile with default params (no construction-seed bias).
			pub fn grow_tile(
				self,
				extent: GroveExtent,
				world: &impl GroveWorldSample,
			) -> ForestGroveTile {
				match self {
					$(Self::$Kind => ForestGroveTile::$Kind(
						$Params::default().with_extent(extent).build_on(world),
					),)+
				}
			}

			/// `select_cell` on `cells`, then grow those placements on `extent`.
			pub fn grow_on_cells(
				self,
				extent: GroveExtent,
				cells: &[Cell],
				world: &impl GroveWorldSample,
			) -> ForestGroveTile {
				match self {
					$(Self::$Kind => {
						let grove = GroveFrontend::default()
							.assemble(chico_groves::$mod::definition());
						let placements = cells
							.iter()
							.filter_map(|cell| {
								GroveRecipe::select_cell(&grove, cell, &extent, world).into_placed()
							})
							.collect();
						let mut params = $Params::default().with_extent(extent);
						params.preview = params.preview.clone().with_resolved_placements(placements);
						ForestGroveTile::$Kind(params.build_on(world))
					})+
				}
			}
		}

		/// Match a [`ForestGroveTile`] and bind the concrete grove as `$g`.
		#[macro_export]
		macro_rules! match_forest_grove_tile {
			($tile:expr, $g:ident => $body:expr) => {
				match $tile {
					$($crate::ForestGroveTile::$Kind($g) => $body,)+
				}
			};
		}
	};
}

impl_kind_recipe! {
	Alpine => alpine, AlpineParams,
	AridConiferSapling => arid_conifer_sapling, AridConiferSaplingParams,
	BraidGrass => braid_grass, BraidGrassParams,
	BushScrub => bush_scrub, BushScrubParams,
	ChristmasTaiga => christmas_taiga, ChristmasTaigaParams,
	CommonTufts => common_tufts, CommonTuftsParams,
	ConiferMassives => conifer_massives, ConiferMassivesParams,
	ConiferSapling => conifer_sapling, ConiferSaplingParams,
	DateGrove => date_grove, DateGroveParams,
	Dryland => dryland, DrylandParams,
	ForlornSavanna => forlorn_savanna, ForlornSavannaParams,
	GoettingenFollow => goettingen_follow, GoettingenFollowParams,
	HighBush => high_bush, HighBushParams,
	JerrysChaparral => jerrys_chaparral, JerrysChaparralParams,
	JungleLowerMassives => jungle_lower_massives, JungleLowerMassivesParams,
	JungleMassives => jungle_massives, JungleMassivesParams,
	Leeward => leeward, LeewardParams,
	LevantineScrub => levantine_scrub, LevantineScrubParams,
	LowBush => low_bush, LowBushParams,
	MonsterGrass => monster_grass, MonsterGrassParams,
	Orchard => orchard, OrchardParams,
	PalmShade => palm_shade, PalmShadeParams,
	RiparianGeneral => riparian_general, RiparianGeneralParams,
	RiparianMix => riparian_mix, RiparianMixParams,
	RiverineGreen => riverine_green, RiverineGreenParams,
	RollingOaks => rolling_oaks, RollingOaksParams,
	Shamanhome => shamanhome, ShamanhomeParams,
	SpottyBushes => spotty_bushes, SpottyBushesParams,
	Storytellers => storytellers, StorytellersParams,
	StrangeOasis => strange_oasis, StrangeOasisParams,
	TallGrass => tall_grass, TallGrassParams,
	TemperateLowerMassives => temperate_lower_massives, TemperateLowerMassivesParams,
	TemperateMassives => temperate_massives, TemperateMassivesParams,
	TradeWinds => trade_winds, TradeWindsParams,
	TropicalThicket => tropical_thicket, TropicalThicketParams,
	TropicalTufts => tropical_tufts, TropicalTuftsParams,
	TropicalUndergrowth => tropical_undergrowth, TropicalUndergrowthParams,
	UnendingJungle => unending_jungle, UnendingJungleParams,
	Vineyard => vineyard, VineyardParams,
	WanderingAcacia => wandering_acacia, WanderingAcaciaParams,
	WildGrass => wild_grass, WildGrassParams,
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy_math::Vec3;
	use chico_groves::{cell_center, FlatTerrainSample, GroveExtent};

	#[test]
	fn orchard_cell_extent_matches_definition() -> Result<()> {
		assert_eq!(ForestGroveKind::Orchard.cell_extent_xz(), Vec2::splat(11.0));
		Ok(())
	}

	#[test]
	fn grow_on_cells_uses_only_those_cells() -> Result<()> {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let cells = extent.cells_overlapping(ForestGroveKind::Orchard.cell_extent_xz());
		let one = cells.first().cloned().ok_or_else(|| anyhow::anyhow!("expected a cell"))?;
		let tile = ForestGroveKind::Orchard.grow_on_cells(
			extent,
			std::slice::from_ref(&one),
			&FlatTerrainSample::default(),
		);
		assert!(matches!(tile, ForestGroveTile::Orchard(_)));
		let _ = cell_center(&one);
		Ok(())
	}
}
