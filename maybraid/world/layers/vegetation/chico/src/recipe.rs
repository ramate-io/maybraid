//! Type-erased [`GroveRecipe`](vegetation_groves::GroveRecipe) for [`ForestGroveKind`].
//!
//! Selection uses the presenting tile's cells. Grow only the placements that won.

use bevy::shape::Aabb3d;
use bevy_math::Vec2;
use gimme_gen::Cell;
use lod::lod_ref::LodRef;
use lod::scene::LodScene;
use lod::{LodSceneCulls, LodSceneLevel, LodSceneStatus, SceneChunk};
use vegetation_groves::{
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

use crate::{ForestGroveKind, ForestGroveTile};

/// Unique woody archetypes for assembled world/forest tiles.
///
/// Authored grove previews keep [`vegetation_groves::GrovePreviewParams::default`]
/// (`100`). Runtime world diversity is a separate setting.
pub const WORLD_FOREST_TREE_VARIANTS: u32 = 32;

/// Blend result for one presenting tile: kind plus the cells that won.
///
/// `cells` is `None` when the tile is uniform (grow the whole footprint).
#[derive(Clone)]
pub struct ForestGroveRecipe {
	pub kind: ForestGroveKind,
	pub extent: GroveExtent,
	pub cells: Option<Vec<Cell>>,
}

impl ForestGroveRecipe {
	pub fn uniform(kind: ForestGroveKind, extent: GroveExtent) -> Self {
		Self { kind, extent, cells: None }
	}

	pub fn on_cells(kind: ForestGroveKind, extent: GroveExtent, cells: Vec<Cell>) -> Self {
		Self { kind, extent, cells: Some(cells) }
	}

	pub fn grow(&self, world: &impl GroveWorldSample) -> ForestGroveTile {
		match &self.cells {
			None => self.kind.grow_tile(self.extent, world),
			Some(cells) => self.kind.grow_on_cells(self.extent, cells, world),
		}
	}
}

macro_rules! impl_kind_recipe {
	($($Kind:ident => $mod:ident, $Params:ident),+ $(,)?) => {
		impl ForestGroveKind {
			/// Authored planting-cell span for this grove.
			pub fn cell_extent_xz(self) -> Vec2 {
				match self {
					$(Self::$Kind => vegetation_groves::$mod::definition().cell_extent_xz,)+
				}
			}

			/// Grow the full tile with default params (no construction-seed bias).
			///
			/// Caps woody archetypes at [`WORLD_FOREST_TREE_VARIANTS`]. Authored
			/// grove previews keep [`vegetation_groves::GrovePreviewParams::default`].
			pub fn grow_tile(
				self,
				extent: GroveExtent,
				world: &impl GroveWorldSample,
			) -> ForestGroveTile {
				match self {
					$(Self::$Kind => ForestGroveTile::$Kind(
						$Params::default()
							.with_extent(extent)
							.with_tree_variants(WORLD_FOREST_TREE_VARIANTS)
							.build_on(world),
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
							.assemble(vegetation_groves::$mod::definition());
						let placements = cells
							.iter()
							.filter_map(|cell| {
								GroveRecipe::select_cell(&grove, cell, &extent, world).into_placed()
							})
							.collect();
						let mut params = $Params::default()
							.with_extent(extent)
							.with_tree_variants(WORLD_FOREST_TREE_VARIANTS);
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

impl ForestGroveTile {
	/// Blade / tuft-patch groves (High kits, not nested woody plants).
	pub fn is_tuft(&self) -> bool {
		matches!(
			self,
			Self::BraidGrass(_)
				| Self::CommonTufts(_)
				| Self::MonsterGrass(_)
				| Self::TallGrass(_)
				| Self::TropicalTufts(_)
				| Self::WildGrass(_)
		)
	}

	pub fn as_orchard(&self) -> Option<&vegetation_groves::Orchard> {
		match self {
			Self::Orchard(grove) => Some(grove),
			_ => None,
		}
	}

	/// Stable finite family label for bounded diagnostics (not a zone name).
	pub fn family_name(&self) -> &'static str {
		match self {
			Self::Alpine(_) => "Alpine",
			Self::AridConiferSapling(_) => "AridConiferSapling",
			Self::BraidGrass(_) => "BraidGrass",
			Self::BushScrub(_) => "BushScrub",
			Self::ChristmasTaiga(_) => "ChristmasTaiga",
			Self::CommonTufts(_) => "CommonTufts",
			Self::ConiferMassives(_) => "ConiferMassives",
			Self::ConiferSapling(_) => "ConiferSapling",
			Self::DateGrove(_) => "DateGrove",
			Self::Dryland(_) => "Dryland",
			Self::ForlornSavanna(_) => "ForlornSavanna",
			Self::GoettingenFollow(_) => "GoettingenFollow",
			Self::HighBush(_) => "HighBush",
			Self::JerrysChaparral(_) => "JerrysChaparral",
			Self::JungleLowerMassives(_) => "JungleLowerMassives",
			Self::JungleMassives(_) => "JungleMassives",
			Self::Leeward(_) => "Leeward",
			Self::LevantineScrub(_) => "LevantineScrub",
			Self::LowBush(_) => "LowBush",
			Self::MonsterGrass(_) => "MonsterGrass",
			Self::Orchard(_) => "Orchard",
			Self::PalmShade(_) => "PalmShade",
			Self::RiparianGeneral(_) => "RiparianGeneral",
			Self::RiparianMix(_) => "RiparianMix",
			Self::RiverineGreen(_) => "RiverineGreen",
			Self::RollingOaks(_) => "RollingOaks",
			Self::Shamanhome(_) => "Shamanhome",
			Self::SpottyBushes(_) => "SpottyBushes",
			Self::Storytellers(_) => "Storytellers",
			Self::StrangeOasis(_) => "StrangeOasis",
			Self::TallGrass(_) => "TallGrass",
			Self::TemperateLowerMassives(_) => "TemperateLowerMassives",
			Self::TemperateMassives(_) => "TemperateMassives",
			Self::TradeWinds(_) => "TradeWinds",
			Self::TropicalThicket(_) => "TropicalThicket",
			Self::TropicalTufts(_) => "TropicalTufts",
			Self::TropicalUndergrowth(_) => "TropicalUndergrowth",
			Self::UnendingJungle(_) => "UnendingJungle",
			Self::Vineyard(_) => "Vineyard",
			Self::WanderingAcacia(_) => "WanderingAcacia",
			Self::WildGrass(_) => "WildGrass",
		}
	}
	pub fn woody_lod(&self) -> Option<vegetation_groves::grove::WoodyGroveLod> {
		match self {
			Self::Alpine(g) => Some(g.woody_lod()),
			Self::AridConiferSapling(g) => Some(g.woody_lod()),
			Self::BushScrub(g) => Some(g.woody_lod()),
			Self::ChristmasTaiga(g) => Some(g.woody_lod()),
			Self::ConiferMassives(g) => Some(g.woody_lod()),
			Self::ConiferSapling(g) => Some(g.woody_lod()),
			Self::DateGrove(g) => Some(g.woody_lod()),
			Self::Dryland(g) => Some(g.woody_lod()),
			Self::ForlornSavanna(g) => Some(g.woody_lod()),
			Self::GoettingenFollow(g) => Some(g.woody_lod()),
			Self::HighBush(g) => Some(g.woody_lod()),
			Self::JerrysChaparral(g) => Some(g.woody_lod()),
			Self::JungleLowerMassives(g) => Some(g.woody_lod()),
			Self::JungleMassives(g) => Some(g.woody_lod()),
			Self::Leeward(g) => Some(g.woody_lod()),
			Self::LevantineScrub(g) => Some(g.woody_lod()),
			Self::LowBush(g) => Some(g.woody_lod()),
			Self::Orchard(g) => Some(g.woody_lod()),
			Self::PalmShade(g) => Some(g.woody_lod()),
			Self::RiparianGeneral(g) => Some(g.woody_lod()),
			Self::RiparianMix(g) => Some(g.woody_lod()),
			Self::RiverineGreen(g) => Some(g.woody_lod()),
			Self::RollingOaks(g) => Some(g.woody_lod()),
			Self::Shamanhome(g) => Some(g.woody_lod()),
			Self::SpottyBushes(g) => Some(g.woody_lod()),
			Self::Storytellers(g) => Some(g.woody_lod()),
			Self::StrangeOasis(g) => Some(g.woody_lod()),
			Self::TemperateLowerMassives(g) => Some(g.woody_lod()),
			Self::TemperateMassives(g) => Some(g.woody_lod()),
			Self::TradeWinds(g) => Some(g.woody_lod()),
			Self::TropicalThicket(g) => Some(g.woody_lod()),
			Self::TropicalUndergrowth(g) => Some(g.woody_lod()),
			Self::UnendingJungle(g) => Some(g.woody_lod()),
			Self::Vineyard(g) => Some(g.woody_lod()),
			Self::WanderingAcacia(g) => Some(g.woody_lod()),
			_ => None,
		}
	}
	pub fn plant_list_identity(&self) -> Option<usize> {
		match self {
			Self::Alpine(g) => Some(g.plant_list_identity()),
			Self::AridConiferSapling(g) => Some(g.plant_list_identity()),
			Self::BushScrub(g) => Some(g.plant_list_identity()),
			Self::ChristmasTaiga(g) => Some(g.plant_list_identity()),
			Self::ConiferMassives(g) => Some(g.plant_list_identity()),
			Self::ConiferSapling(g) => Some(g.plant_list_identity()),
			Self::DateGrove(g) => Some(g.plant_list_identity()),
			Self::Dryland(g) => Some(g.plant_list_identity()),
			Self::ForlornSavanna(g) => Some(g.plant_list_identity()),
			Self::GoettingenFollow(g) => Some(g.plant_list_identity()),
			Self::HighBush(g) => Some(g.plant_list_identity()),
			Self::JerrysChaparral(g) => Some(g.plant_list_identity()),
			Self::JungleLowerMassives(g) => Some(g.plant_list_identity()),
			Self::JungleMassives(g) => Some(g.plant_list_identity()),
			Self::Leeward(g) => Some(g.plant_list_identity()),
			Self::LevantineScrub(g) => Some(g.plant_list_identity()),
			Self::LowBush(g) => Some(g.plant_list_identity()),
			Self::Orchard(g) => Some(g.plant_list_identity()),
			Self::PalmShade(g) => Some(g.plant_list_identity()),
			Self::RiparianGeneral(g) => Some(g.plant_list_identity()),
			Self::RiparianMix(g) => Some(g.plant_list_identity()),
			Self::RiverineGreen(g) => Some(g.plant_list_identity()),
			Self::RollingOaks(g) => Some(g.plant_list_identity()),
			Self::Shamanhome(g) => Some(g.plant_list_identity()),
			Self::SpottyBushes(g) => Some(g.plant_list_identity()),
			Self::Storytellers(g) => Some(g.plant_list_identity()),
			Self::StrangeOasis(g) => Some(g.plant_list_identity()),
			Self::TemperateLowerMassives(g) => Some(g.plant_list_identity()),
			Self::TemperateMassives(g) => Some(g.plant_list_identity()),
			Self::TradeWinds(g) => Some(g.plant_list_identity()),
			Self::TropicalThicket(g) => Some(g.plant_list_identity()),
			Self::TropicalUndergrowth(g) => Some(g.plant_list_identity()),
			Self::UnendingJungle(g) => Some(g.plant_list_identity()),
			Self::Vineyard(g) => Some(g.plant_list_identity()),
			Self::WanderingAcacia(g) => Some(g.plant_list_identity()),
			_ => None,
		}
	}
	pub fn visit_posed_plants<V: vegetation_groves::grove::PosedPlantVisitor<Output = ()>>(
		&self,
		visitor: &mut V,
	) -> bool {
		match self {
			Self::Alpine(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::AridConiferSapling(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::BushScrub(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::ChristmasTaiga(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::ConiferMassives(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::ConiferSapling(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::DateGrove(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::Dryland(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::ForlornSavanna(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::GoettingenFollow(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::HighBush(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::JerrysChaparral(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::JungleLowerMassives(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::JungleMassives(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::Leeward(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::LevantineScrub(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::LowBush(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::Orchard(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::PalmShade(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::RiparianGeneral(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::RiparianMix(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::RiverineGreen(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::RollingOaks(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::Shamanhome(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::SpottyBushes(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::Storytellers(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::StrangeOasis(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::TemperateLowerMassives(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::TemperateMassives(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::TradeWinds(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::TropicalThicket(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::TropicalUndergrowth(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::UnendingJungle(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::Vineyard(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			Self::WanderingAcacia(g) => {
				g.visit_posed_plants(visitor);
				true
			}
			_ => false,
		}
	}
	pub fn proxy_stick_nodes(&self, level: LodSceneLevel) -> Vec<vegetation_components::StickNode> {
		use vegetation_components::VegetationComponents;
		match_forest_grove_tile!(self, g => g.stick_nodes_for_level(level).flatten())
	}
	pub fn proxy_foliage_nodes(
		&self,
		level: LodSceneLevel,
	) -> Vec<vegetation_components::FoliageNode> {
		use vegetation_components::VegetationComponents;
		match_forest_grove_tile!(self, g => g.foliage_nodes_for_level(level).flatten())
	}
	pub fn structural_lod(&self) -> Option<vegetation_components::StructuralLod> {
		use vegetation_components::VegetationComponents;
		match_forest_grove_tile!(self, g => g.structural_lod())
	}

	pub fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		match_forest_grove_tile!(self, g => g.scene_lod_level(lod_ref))
	}

	pub fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		match_forest_grove_tile!(self, g => g.scene_lod_status(lod_ref))
	}

	pub fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		match_forest_grove_tile!(self, g => g.scene_lod_culls(lod_ref, current))
	}

	pub fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		match_forest_grove_tile!(self, g => g.scene_chunks_with_level(lod_ref, level))
	}

	pub fn scene_bounds(&self) -> Aabb3d {
		match_forest_grove_tile!(self, g => g.scene_bounds())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use bevy_math::Vec3;
	use vegetation_groves::{cell_center, FlatTerrainSample, GroveExtent, OrchardParams};

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

	#[test]
	fn forest_assemble_does_not_evaluate_authored_constraint_bands() -> Result<()> {
		use gimme_gen::Cell;
		use vegetation_groves::levantine_scrub::LevantineScrubCell;
		use vegetation_groves::{ForestGroveBiases, GroveCellOutcome, GroveFrontend};

		// Forest grow assembles with [`GroveFrontend::default`] — default biases and
		// no elevation / steepness normalization domain ([RFC-183 3.5.1], 3.4.2.4).
		// Levantine `DryHighBush` authored steepness ends at 0.65; 0.69 would empty
		// every placed bucket if constraints ran. The owner currently places anyway.
		let grove =
			GroveFrontend::default().assemble(vegetation_groves::levantine_scrub::definition());
		assert_eq!(*grove.biases(), ForestGroveBiases::default());
		let terrain = FlatTerrainSample { elevation: 0.25, steepness: 0.69 };
		let outcome = grove.distribution().select_from(
			3,
			Vec3::new(5.0, 0.25, 5.0),
			1.0,
			Cell::from_min_max(Vec3::ZERO, Vec3::ONE),
			&terrain,
		);
		match outcome {
			GroveCellOutcome::Placed { variant, .. } => {
				assert_eq!(variant, LevantineScrubCell::DryHighBush);
			}
			other => {
				anyhow::bail!("expected DryHighBush without constraint evaluation, got {other:?}")
			}
		}
		Ok(())
	}

	#[test]
	fn world_forest_tree_variants_stay_below_authored_preview_default() {
		assert_eq!(WORLD_FOREST_TREE_VARIANTS, 32);
		let preview = vegetation_groves::GrovePreviewParams::<()>::default();
		assert_eq!(preview.tree_variants, 100);
		assert!(WORLD_FOREST_TREE_VARIANTS < preview.tree_variants);
		let params = OrchardParams::default()
			.with_extent(GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0)))
			.with_tree_variants(WORLD_FOREST_TREE_VARIANTS);
		assert_eq!(params.tree_variants, WORLD_FOREST_TREE_VARIANTS);
	}
}
