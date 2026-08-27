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

use crate::blend::{
	hash_unit_xz, pick_source, FaceNeighbors, CANOPY_BLEND_WIDTH, TUFT_BLEND_WIDTH,
	UNDERSTORY_BLEND_WIDTH,
};
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
	let Some(kind) = kind else {
		return Vec::new();
	};
	tiles
		.iter()
		.flat_map(|extent| grow_presenting_tile(kind, *extent, forest, neighbors, layer, world))
		.collect()
}

/// Cardinal forest-cell selections used only for edge-tile blend (not grown).
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

	fn blend_width(self) -> f32 {
		match self {
			Self::Tufts => TUFT_BLEND_WIDTH,
			Self::Understory => UNDERSTORY_BLEND_WIDTH,
			Self::LowerCanopy | Self::UpperCanopy => CANOPY_BLEND_WIDTH,
		}
	}
}

fn face_neighbors(
	tile: GroveExtent,
	forest: ForestExtent,
	neighbors: &NeighborLayers,
	layer: LayerSlot,
) -> FaceNeighbors {
	const FACE_EPS: f32 = 0.5;
	let open = |on_face: bool, layers: Option<SelectedLayers>| {
		if on_face {
			Some(layers.and_then(|l| layer.kind(l)))
		} else {
			None
		}
	};
	FaceNeighbors {
		north: open((forest.max().z - tile.max().z).abs() < FACE_EPS, neighbors.north),
		east: open((forest.max().x - tile.max().x).abs() < FACE_EPS, neighbors.east),
		south: open((tile.min().z - forest.min().z).abs() < FACE_EPS, neighbors.south),
		west: open((tile.min().x - forest.min().x).abs() < FACE_EPS, neighbors.west),
	}
}

fn grow_presenting_tile(
	kind: ForestGroveKind,
	extent: GroveExtent,
	forest: ForestExtent,
	neighbors: &NeighborLayers,
	layer: LayerSlot,
	world: &impl GroveWorldSample,
) -> Vec<ForestGroveTile> {
	let faces = face_neighbors(extent, forest, neighbors, layer);
	if !faces.needs_blend(kind) {
		return vec![grow_tile(kind, extent, world)];
	}
	grow_blended(kind, extent, faces, layer.blend_width(), world)
}

fn grow_blended(
	self_kind: ForestGroveKind,
	extent: GroveExtent,
	faces: FaceNeighbors,
	blend_width: f32,
	world: &impl GroveWorldSample,
) -> Vec<ForestGroveTile> {
	use std::collections::HashMap;

	use chico_groves::cell_center;

	let cells = extent.cells_overlapping(self_kind.cell_extent_xz());
	let mut buckets: HashMap<ForestGroveKind, Vec<gimme_gen::Cell>> = HashMap::new();
	for cell in cells {
		let center = cell_center(&cell);
		let source = pick_source(center, extent, blend_width, faces, hash_unit_xz(center));
		let winner = match source {
			None => Some(self_kind),
			Some(face) => faces.get(face).and_then(|k| k),
		};
		if let Some(kind) = winner {
			buckets.entry(kind).or_default().push(cell);
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
/// `neighbors` are Hopscotch selections for the four adjacent forest cells
/// (not grown). Edge tiles blend; interior tiles stay a single `grow_tile`.
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
	fn edge_tile_blends_two_kinds() -> Result<()> {
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
}
