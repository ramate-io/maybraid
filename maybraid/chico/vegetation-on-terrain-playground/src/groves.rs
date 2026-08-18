//! Tile groves onto Durham via [`GroveWorldSample::height_at`].

use bevy::prelude::*;
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
use chico_vegetation_components::{
	spawn_lod_scene_host, spawn_vegetation_components, vegetation_bounds, VegetationComponents,
};
use durham_terrain_models::{BaseTerrainNoise, TerrainCellLayout, TerrainEntryStore};
use lod::gen::LodScene;

use crate::commands::GroveKind;
use crate::PlaygroundConfig;

#[derive(Component)]
pub struct GroveRoot;

pub struct DurhamGroveSample<'a> {
	store: &'a TerrainEntryStore,
	layout: &'a TerrainCellLayout,
	fallback: &'a BaseTerrainNoise,
}

impl GroveWorldSample for DurhamGroveSample<'_> {
	fn height_at(&self, position: Vec3) -> f32 {
		self.store
			.composed_height_at(self.layout, position.x, position.z)
			.unwrap_or_else(|| self.fallback.height_at(position.x, position.z))
	}

	fn steepness_at(&self, _position: Vec3) -> f32 {
		0.0
	}
}

pub fn spawn_tiled_groves(
	commands: &mut Commands,
	config: &PlaygroundConfig,
	store: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	fallback: &BaseTerrainNoise,
) -> usize {
	let world = DurhamGroveSample { store, layout, fallback };
	let tile = config.grove_extent_xz.max(1.0);
	let radius = config.tile_radius.max(0);
	let mut count = 0usize;
	for ix in -radius..=radius {
		for iz in -radius..=radius {
			// Tile (0, 0) is the `tile`×`tile` square centered on the origin.
			let min = Vec3::new((ix as f32 - 0.5) * tile, 0.0, (iz as f32 - 0.5) * tile);
			let extent = GroveExtent::new(min, min + Vec3::new(tile, 1.0, tile));
			for entity in spawn_kind(commands, config.grove, extent, &world) {
				commands.entity(entity).insert(GroveRoot);
				count += 1;
			}
		}
	}
	count
}

fn spawn_components<T>(commands: &mut Commands, grove: &T) -> Vec<Entity>
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
{
	let bounds = vegetation_bounds(grove);
	spawn_vegetation_components(commands, grove, Transform::IDENTITY, bounds)
}

fn spawn_host<T>(commands: &mut Commands, grove: &T) -> Vec<Entity>
where
	T: LodScene + VegetationComponents + Component + Clone + Send + Sync + 'static,
{
	let bounds = grove
		.structural_lod()
		.map(|p| p.footprint_aabb())
		.unwrap_or_else(|| vegetation_bounds(grove));
	spawn_lod_scene_host(commands, grove, Transform::IDENTITY, bounds)
}

fn spawn_kind(
	commands: &mut Commands,
	kind: GroveKind,
	extent: GroveExtent,
	world: &impl GroveWorldSample,
) -> Vec<Entity> {
	match kind {
		GroveKind::MonsterGrass => spawn_components(
			commands,
			&MonsterGrassParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::BraidGrass => spawn_components(
			commands,
			&BraidGrassParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::TropicalTufts => spawn_components(
			commands,
			&TropicalTuftsParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::CommonTufts => spawn_components(
			commands,
			&CommonTuftsParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::TallGrass => spawn_components(
			commands,
			&TallGrassParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::WildGrass => spawn_components(
			commands,
			&WildGrassParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::BushScrub => {
			spawn_host(commands, &BushScrubParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::TropicalUndergrowth => spawn_host(
			commands,
			&TropicalUndergrowthParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::LevantineScrub => spawn_host(
			commands,
			&LevantineScrubParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::StrangeOasis => {
			spawn_host(commands, &StrangeOasisParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::TropicalThicket => spawn_host(
			commands,
			&TropicalThicketParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::RollingOaks => {
			spawn_host(commands, &RollingOaksParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::Orchard => {
			spawn_host(commands, &OrchardParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::RiparianGeneral => spawn_host(
			commands,
			&RiparianGeneralParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::ForlornSavanna => spawn_host(
			commands,
			&ForlornSavannaParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::GoettingenFollow => spawn_host(
			commands,
			&GoettingenFollowParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::Vineyard => {
			spawn_host(commands, &VineyardParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::Dryland => {
			spawn_host(commands, &DrylandParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::Leeward => {
			spawn_host(commands, &LeewardParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::TemperateLowerMassives => spawn_host(
			commands,
			&TemperateLowerMassivesParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::TemperateMassives => spawn_host(
			commands,
			&TemperateMassivesParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::Storytellers => {
			spawn_host(commands, &StorytellersParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::WanderingAcacia => spawn_host(
			commands,
			&WanderingAcaciaParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::TradeWinds => {
			spawn_host(commands, &TradeWindsParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::HighBush => {
			spawn_host(commands, &HighBushParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::SpottyBushes => {
			spawn_host(commands, &SpottyBushesParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::RiverineGreen => spawn_host(
			commands,
			&RiverineGreenParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::LowBush => {
			spawn_host(commands, &LowBushParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::JungleMassives => spawn_host(
			commands,
			&JungleMassivesParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::JungleLowerMassives => spawn_host(
			commands,
			&JungleLowerMassivesParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::UnendingJungle => spawn_host(
			commands,
			&UnendingJungleParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::JerrysChaparral => spawn_host(
			commands,
			&JerrysChaparralParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::RiparianMix => {
			spawn_host(commands, &RiparianMixParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::Alpine => {
			spawn_host(commands, &AlpineParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::ChristmasTaiga => spawn_host(
			commands,
			&ChristmasTaigaParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::ConiferSapling => spawn_host(
			commands,
			&ConiferSaplingParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::AridConiferSapling => spawn_host(
			commands,
			&AridConiferSaplingParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::ConiferMassives => spawn_host(
			commands,
			&ConiferMassivesParams::default().with_extent(extent).build_on(world),
		),
		GroveKind::PalmShade => {
			spawn_host(commands, &PalmShadeParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::Shamanhome => {
			spawn_host(commands, &ShamanhomeParams::default().with_extent(extent).build_on(world))
		}
		GroveKind::DateGrove => {
			spawn_host(commands, &DateGroveParams::default().with_extent(extent).build_on(world))
		}
	}
}
