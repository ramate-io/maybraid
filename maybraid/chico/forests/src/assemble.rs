//! Grow selected groves on 100 m tiles inside a forest cell.
//!
//! Each tile calls `Params::default().with_extent(tile).build_on(world)`. Construction
//! noise stays on the grove default — forests do not bias `build_unit` seeds.

use chico_groves::{
	AlpineParams, AridConiferSaplingParams, BraidGrassParams, BushScrubParams,
	ChristmasTaigaParams, CommonTuftsParams, ConiferMassivesParams, ConiferSaplingParams,
	DateGroveParams, DrylandParams, ForlornSavannaParams, GoettingenFollowParams, GroveExtent,
	GroveWorldSample, HighBushParams, JerrysChaparralParams, JungleLowerMassivesParams,
	JungleMassivesParams, LeewardParams, LevantineScrubParams, LowBushParams, MonsterGrassParams,
	OrchardParams, PalmShadeParams, RiparianGeneralParams, RiparianMixParams, RiverineGreenParams,
	RollingOaksParams, ShamanhomeParams, SpottyBushesParams, StorytellersParams,
	StrangeOasisParams, TallGrassParams, TemperateLowerMassivesParams, TemperateMassivesParams,
	TradeWindsParams, TropicalThicketParams, TropicalTuftsParams, TropicalUndergrowthParams,
	UnendingJungleParams, VineyardParams, WanderingAcaciaParams, WildGrassParams,
};

use crate::blend::{hash_unit_xz, neighbor_tile, pick_kind, Cardinal, GroveNeighbors};
use crate::{ForestExtent, ForestGroveKind, SelectedLayers};

/// One grown grove tile (concrete grove type).
#[derive(Clone)]
pub enum ForestGroveTile {
	Alpine(chico_groves::Alpine),
	AridConiferSapling(chico_groves::AridConiferSapling),
	BraidGrass(chico_groves::BraidGrass),
	BushScrub(chico_groves::BushScrub),
	ChristmasTaiga(chico_groves::ChristmasTaiga),
	CommonTufts(chico_groves::CommonTufts),
	ConiferMassives(chico_groves::ConiferMassives),
	ConiferSapling(chico_groves::ConiferSapling),
	DateGrove(chico_groves::DateGrove),
	Dryland(chico_groves::Dryland),
	ForlornSavanna(chico_groves::ForlornSavanna),
	GoettingenFollow(chico_groves::GoettingenFollow),
	HighBush(chico_groves::HighBush),
	JerrysChaparral(chico_groves::JerrysChaparral),
	JungleLowerMassives(chico_groves::JungleLowerMassives),
	JungleMassives(chico_groves::JungleMassives),
	Leeward(chico_groves::Leeward),
	LevantineScrub(chico_groves::LevantineScrub),
	LowBush(chico_groves::LowBush),
	MonsterGrass(chico_groves::MonsterGrass),
	Orchard(chico_groves::Orchard),
	PalmShade(chico_groves::PalmShade),
	RiparianGeneral(chico_groves::RiparianGeneral),
	RiparianMix(chico_groves::RiparianMix),
	RiverineGreen(chico_groves::RiverineGreen),
	RollingOaks(chico_groves::RollingOaks),
	Shamanhome(chico_groves::Shamanhome),
	SpottyBushes(chico_groves::SpottyBushes),
	Storytellers(chico_groves::Storytellers),
	StrangeOasis(chico_groves::StrangeOasis),
	TallGrass(chico_groves::TallGrass),
	TemperateLowerMassives(chico_groves::TemperateLowerMassives),
	TemperateMassives(chico_groves::TemperateMassives),
	TradeWinds(chico_groves::TradeWinds),
	TropicalThicket(chico_groves::TropicalThicket),
	TropicalTufts(chico_groves::TropicalTufts),
	TropicalUndergrowth(chico_groves::TropicalUndergrowth),
	UnendingJungle(chico_groves::UnendingJungle),
	Vineyard(chico_groves::Vineyard),
	WanderingAcacia(chico_groves::WanderingAcacia),
	WildGrass(chico_groves::WildGrass),
}

/// Grown tiles for the four forest layers (ground cover stays empty).
#[derive(Clone)]
pub struct AssembledForest {
	pub layers: SelectedLayers,
	pub tufts: Vec<ForestGroveTile>,
	pub understory: Vec<ForestGroveTile>,
	pub lower_canopy: Vec<ForestGroveTile>,
	pub upper_canopy: Vec<ForestGroveTile>,
}

impl AssembledForest {
	/// Flattened tiles in layer order: tufts, understory, lower canopy, upper canopy.
	pub fn tiles(&self) -> impl Iterator<Item = &ForestGroveTile> {
		self.tufts
			.iter()
			.chain(self.understory.iter())
			.chain(self.lower_canopy.iter())
			.chain(self.upper_canopy.iter())
	}
}

/// Grow `kind` on one grove tile. Uses default grove params (no construction-seed bias).
pub fn grow_tile(
	kind: ForestGroveKind,
	extent: GroveExtent,
	world: &impl GroveWorldSample,
) -> ForestGroveTile {
	match kind {
		ForestGroveKind::Alpine => {
			ForestGroveTile::Alpine(AlpineParams::default().with_extent(extent).build_on(world))
		}
		ForestGroveKind::AridConiferSapling => ForestGroveTile::AridConiferSapling(
			AridConiferSaplingParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::BraidGrass => ForestGroveTile::BraidGrass(
			BraidGrassParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::BushScrub => ForestGroveTile::BushScrub(
			BushScrubParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::ChristmasTaiga => ForestGroveTile::ChristmasTaiga(
			ChristmasTaigaParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::CommonTufts => ForestGroveTile::CommonTufts(
			CommonTuftsParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::ConiferMassives => ForestGroveTile::ConiferMassives(
			ConiferMassivesParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::ConiferSapling => ForestGroveTile::ConiferSapling(
			ConiferSaplingParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::DateGrove => ForestGroveTile::DateGrove(
			DateGroveParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::Dryland => {
			ForestGroveTile::Dryland(DrylandParams::default().with_extent(extent).build_on(world))
		}
		ForestGroveKind::ForlornSavanna => ForestGroveTile::ForlornSavanna(
			ForlornSavannaParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::GoettingenFollow => ForestGroveTile::GoettingenFollow(
			GoettingenFollowParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::HighBush => {
			ForestGroveTile::HighBush(HighBushParams::default().with_extent(extent).build_on(world))
		}
		ForestGroveKind::JerrysChaparral => ForestGroveTile::JerrysChaparral(
			JerrysChaparralParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::JungleLowerMassives => ForestGroveTile::JungleLowerMassives(
			JungleLowerMassivesParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::JungleMassives => ForestGroveTile::JungleMassives(
			JungleMassivesParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::Leeward => {
			ForestGroveTile::Leeward(LeewardParams::default().with_extent(extent).build_on(world))
		}
		ForestGroveKind::LevantineScrub => ForestGroveTile::LevantineScrub(
			LevantineScrubParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::LowBush => {
			ForestGroveTile::LowBush(LowBushParams::default().with_extent(extent).build_on(world))
		}
		ForestGroveKind::MonsterGrass => ForestGroveTile::MonsterGrass(
			MonsterGrassParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::Orchard => {
			ForestGroveTile::Orchard(OrchardParams::default().with_extent(extent).build_on(world))
		}
		ForestGroveKind::PalmShade => ForestGroveTile::PalmShade(
			PalmShadeParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::RiparianGeneral => ForestGroveTile::RiparianGeneral(
			RiparianGeneralParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::RiparianMix => ForestGroveTile::RiparianMix(
			RiparianMixParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::RiverineGreen => ForestGroveTile::RiverineGreen(
			RiverineGreenParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::RollingOaks => ForestGroveTile::RollingOaks(
			RollingOaksParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::Shamanhome => ForestGroveTile::Shamanhome(
			ShamanhomeParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::SpottyBushes => ForestGroveTile::SpottyBushes(
			SpottyBushesParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::Storytellers => ForestGroveTile::Storytellers(
			StorytellersParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::StrangeOasis => ForestGroveTile::StrangeOasis(
			StrangeOasisParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::TallGrass => ForestGroveTile::TallGrass(
			TallGrassParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::TemperateLowerMassives => ForestGroveTile::TemperateLowerMassives(
			TemperateLowerMassivesParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::TemperateMassives => ForestGroveTile::TemperateMassives(
			TemperateMassivesParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::TradeWinds => ForestGroveTile::TradeWinds(
			TradeWindsParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::TropicalThicket => ForestGroveTile::TropicalThicket(
			TropicalThicketParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::TropicalTufts => ForestGroveTile::TropicalTufts(
			TropicalTuftsParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::TropicalUndergrowth => ForestGroveTile::TropicalUndergrowth(
			TropicalUndergrowthParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::UnendingJungle => ForestGroveTile::UnendingJungle(
			UnendingJungleParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::Vineyard => {
			ForestGroveTile::Vineyard(VineyardParams::default().with_extent(extent).build_on(world))
		}
		ForestGroveKind::WanderingAcacia => ForestGroveTile::WanderingAcacia(
			WanderingAcaciaParams::default().with_extent(extent).build_on(world),
		),
		ForestGroveKind::WildGrass => ForestGroveTile::WildGrass(
			WildGrassParams::default().with_extent(extent).build_on(world),
		),
	}
}

fn grow_layer(
	kind: Option<ForestGroveKind>,
	tiles: &[GroveExtent],
	forest: ForestExtent,
	neighbors: &NeighborLayers,
	layer: LayerSlot,
	world: &impl GroveWorldSample,
) -> Vec<ForestGroveTile> {
	tiles
		.iter()
		.flat_map(|extent| grow_presenting_tile(kind, *extent, forest, neighbors, layer, world))
		.collect()
}

/// Cardinal forest-cell selections for grove slots that sit outside this extent.
///
/// Not grown. Every presenting tile reads these when a neighbor 100 m slot
/// leaves this forest; interior neighbor slots use this cell's own layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NeighborLayers {
	pub north: Option<SelectedLayers>,
	pub east: Option<SelectedLayers>,
	pub south: Option<SelectedLayers>,
	pub west: Option<SelectedLayers>,
}

impl NeighborLayers {
	pub fn none() -> Self {
		Self { north: None, east: None, south: None, west: None }
	}

	pub fn get(self, face: Cardinal) -> Option<SelectedLayers> {
		match face {
			Cardinal::North => self.north,
			Cardinal::East => self.east,
			Cardinal::South => self.south,
			Cardinal::West => self.west,
		}
	}
}

#[derive(Clone, Copy)]
enum LayerSlot {
	Tufts,
	Understory,
	LowerCanopy,
	UpperCanopy,
}

impl LayerSlot {
	fn kind(self, layers: SelectedLayers) -> Option<ForestGroveKind> {
		match self {
			Self::Tufts => layers.tufts,
			Self::Understory => layers.understory,
			Self::LowerCanopy => layers.lower_canopy,
			Self::UpperCanopy => layers.upper_canopy,
		}
	}
}

fn grove_neighbors(
	tile: GroveExtent,
	forest: ForestExtent,
	self_kind: Option<ForestGroveKind>,
	neighbors: &NeighborLayers,
	layer: LayerSlot,
) -> GroveNeighbors {
	let slot = |face: Cardinal| {
		let center = {
			let n = neighbor_tile(tile, face);
			(n.min() + n.max()) * 0.5
		};
		if forest.owns_center_xz(center) {
			Some(self_kind)
		} else {
			neighbors.get(face).map(|layers| layer.kind(layers))
		}
	};
	GroveNeighbors {
		north: slot(Cardinal::North),
		east: slot(Cardinal::East),
		south: slot(Cardinal::South),
		west: slot(Cardinal::West),
	}
}

fn grow_presenting_tile(
	kind: Option<ForestGroveKind>,
	extent: GroveExtent,
	forest: ForestExtent,
	neighbors: &NeighborLayers,
	layer: LayerSlot,
	world: &impl GroveWorldSample,
) -> Vec<ForestGroveTile> {
	grow_blended(kind, extent, grove_neighbors(extent, forest, kind, neighbors, layer), world)
}

fn grow_blended(
	self_kind: Option<ForestGroveKind>,
	extent: GroveExtent,
	neighbors: GroveNeighbors,
	world: &impl GroveWorldSample,
) -> Vec<ForestGroveTile> {
	use std::collections::HashMap;

	use chico_groves::cell_center;

	let mut buckets: HashMap<ForestGroveKind, Vec<gimme_gen::Cell>> = HashMap::new();
	if let Some(self_kind) = self_kind {
		for cell in extent.cells_overlapping(self_kind.cell_extent_xz()) {
			let center = cell_center(&cell);
			if let Some(kind) =
				pick_kind(center, extent, Some(self_kind), neighbors, hash_unit_xz(center))
			{
				buckets.entry(kind).or_default().push(cell);
			}
		}
	} else {
		let mut lattices = Vec::<ForestGroveKind>::new();
		for lattice in neighbors.planted_kinds() {
			if !lattices.contains(&lattice) {
				lattices.push(lattice);
			}
		}
		for lattice in lattices {
			for cell in extent.cells_overlapping(lattice.cell_extent_xz()) {
				let center = cell_center(&cell);
				if pick_kind(center, extent, None, neighbors, hash_unit_xz(center)) == Some(lattice)
				{
					buckets.entry(lattice).or_default().push(cell);
				}
			}
		}
	}
	buckets
		.into_iter()
		.filter(|(_, cells)| !cells.is_empty())
		.map(|(kind, cells)| kind.grow_on_cells(extent, &cells, world))
		.collect()
}

/// Assemble selected layers onto the forest cell's 100 m grove grid.
///
/// Every presenting tile softmax-blends its cardinal neighbor grove recipes
/// (empty layers still grow neighbor islands). `neighbors` supply selections
/// for slots that sit outside this extent.
pub fn assemble(
	extent: ForestExtent,
	layers: SelectedLayers,
	neighbors: NeighborLayers,
	world: &impl GroveWorldSample,
) -> AssembledForest {
	let tiles = extent.default_grove_tiles();
	AssembledForest {
		layers,
		tufts: grow_layer(layers.tufts, &tiles, extent, &neighbors, LayerSlot::Tufts, world),
		understory: grow_layer(
			layers.understory,
			&tiles,
			extent,
			&neighbors,
			LayerSlot::Understory,
			world,
		),
		lower_canopy: grow_layer(
			layers.lower_canopy,
			&tiles,
			extent,
			&neighbors,
			LayerSlot::LowerCanopy,
			world,
		),
		upper_canopy: grow_layer(
			layers.upper_canopy,
			&tiles,
			extent,
			&neighbors,
			LayerSlot::UpperCanopy,
			world,
		),
	}
}

/// Assemble with no neighbor recipes (same as isolated preview tiles).
pub fn assemble_isolated(
	extent: ForestExtent,
	layers: SelectedLayers,
	world: &impl GroveWorldSample,
) -> AssembledForest {
	assemble(extent, layers, NeighborLayers::none(), world)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{select_cell, ForestGroveKind, LayeringKind, SelectedLayers};
	use anyhow::Result;
	use bevy_math::Vec3;
	use chico_groves::FlatTerrainSample;
	use procedural_common::NoiseParams;

	#[test]
	fn grow_one_ag_town_orchard_tile() -> Result<()> {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let tile = grow_tile(ForestGroveKind::Orchard, extent, &FlatTerrainSample::default());
		assert!(matches!(tile, ForestGroveTile::Orchard(_)));
		Ok(())
	}

	#[test]
	fn assemble_empty_layers_grows_nothing() -> Result<()> {
		let cell = ForestExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let layers = SelectedLayers {
			layering: LayeringKind::SunsBarren,
			tufts: None,
			understory: None,
			lower_canopy: None,
			upper_canopy: None,
		};
		let grown = assemble(cell, layers, NeighborLayers::none(), &FlatTerrainSample::default());
		assert!(grown.tufts.is_empty());
		assert!(grown.upper_canopy.is_empty());
		let _ = select_cell(cell, NoiseParams::default());
		Ok(())
	}

	#[test]
	fn presenting_tile_blends_neighbor_recipe() -> Result<()> {
		let forest = ForestExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let layers = SelectedLayers {
			layering: LayeringKind::AgTown,
			tufts: None,
			understory: None,
			lower_canopy: None,
			upper_canopy: Some(ForestGroveKind::Orchard),
		};
		let neighbors = NeighborLayers {
			east: Some(SelectedLayers {
				layering: LayeringKind::MiRobles,
				tufts: None,
				understory: None,
				lower_canopy: None,
				upper_canopy: Some(ForestGroveKind::RollingOaks),
			}),
			..NeighborLayers::none()
		};
		let grown = assemble(forest, layers, neighbors, &FlatTerrainSample::default());
		let kinds = grown
			.upper_canopy
			.iter()
			.map(|tile| match tile {
				ForestGroveTile::Orchard(_) => "orchard",
				ForestGroveTile::RollingOaks(_) => "oaks",
				_ => "other",
			})
			.collect::<Vec<_>>();
		assert!(kinds.contains(&"orchard"), "{kinds:?}");
		assert!(kinds.contains(&"oaks"), "expected a blended oak host, got {kinds:?}");
		Ok(())
	}

	#[test]
	fn interior_slot_uses_self_kind_not_far_forest() -> Result<()> {
		let forest = ForestExtent::default_cell();
		let tiles = forest.default_grove_tiles();
		let interior = tiles
			.iter()
			.copied()
			.find(|tile| {
				let c = (tile.min() + tile.max()) * 0.5;
				c.x.abs() <= 50.0 && c.z.abs() <= 50.0
			})
			.ok_or_else(|| anyhow::anyhow!("expected an origin-centered grove tile"))?;
		let neighbors = NeighborLayers {
			east: Some(SelectedLayers {
				layering: LayeringKind::MiRobles,
				tufts: None,
				understory: None,
				lower_canopy: None,
				upper_canopy: Some(ForestGroveKind::RollingOaks),
			}),
			..NeighborLayers::none()
		};
		let slots = grove_neighbors(
			interior,
			forest,
			Some(ForestGroveKind::Orchard),
			&neighbors,
			LayerSlot::UpperCanopy,
		);
		assert_eq!(slots.east, Some(Some(ForestGroveKind::Orchard)));
		assert_eq!(slots.west, Some(Some(ForestGroveKind::Orchard)));
		Ok(())
	}

	#[test]
	fn empty_tile_grows_neighbor_islands() -> Result<()> {
		let forest = ForestExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let layers = SelectedLayers {
			layering: LayeringKind::SunsBarren,
			tufts: None,
			understory: None,
			lower_canopy: None,
			upper_canopy: None,
		};
		let neighbors = NeighborLayers {
			west: Some(SelectedLayers {
				layering: LayeringKind::AgTown,
				tufts: None,
				understory: None,
				lower_canopy: None,
				upper_canopy: Some(ForestGroveKind::Orchard),
			}),
			..NeighborLayers::none()
		};
		let grown = assemble(forest, layers, neighbors, &FlatTerrainSample::default());
		assert!(
			grown
				.upper_canopy
				.iter()
				.any(|tile| matches!(tile, ForestGroveTile::Orchard(_))),
			"empty layer should still present neighbor islands"
		);
		Ok(())
	}
}
