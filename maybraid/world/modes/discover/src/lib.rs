//! Discovery: the streamed Maybraid world.
//!
//! The game shell still owns cameras, loading, and pause. This crate is the
//! session those systems ask when the home row enters Discovery.

use std::any::TypeId;

use barking::{install_mob_grid_stream, Barking, BarkingConfig};
use bevy::prelude::*;
use chico::{install_vegetation_stream, Chico, ChicoConfig};
use durham::{playable_world_cell_layout, Durham, DurhamTerrainConfig, TerrainRetarget};
use furnishing_layer_model::Furnishing;
use geneva::{install_language_stream, Geneva, LanguageConfig};
use language_layer_model::Language;
use layer_stack::{ActiveGenerationMode, GenerationMode, LayerModeConfig, Scheme};
use maputo::{install_furnishing_stream, Maputo};
use mob_layer_model::Mobs;
use richmond::{install_urbanization_stream, Richmond, RichmondConfig};
use terrain_layer_model::OnTerrain;
use urbanization_layer_model::Urbanization;
use vegetation_layer_model::Vegetation;
use world_player::{ModePlayerPolicies, ModePlayerPolicy};

pub const LABEL: &str = "Discovery";

/// Terrain, vegetation, and urbanization stream while Discovery is in the world shell.
pub fn streams_terrain(session_is_discovery: bool, in_world_shell: bool) -> bool {
	session_is_discovery && in_world_shell
}

/// Playable-world generation. Layers take this as a plugin parameter.
pub struct Discovery;

impl GenerationMode for Discovery {}

impl Scheme<OnTerrain<Durham>> for Discovery {
	fn install(app: &mut App, _config: &DurhamTerrainConfig) {
		app.add_systems(OnEnter(ActiveGenerationMode::of::<Discovery>()), restore_playable_world);
	}
}

impl Scheme<Urbanization<Richmond<OnTerrain<Durham>>>> for Discovery {
	fn install(app: &mut App, _config: &RichmondConfig) {
		install_urbanization_stream::<Discovery, OnTerrain<Durham>>(app);
	}
}

impl Scheme<Vegetation<Chico<Urbanization<Richmond<OnTerrain<Durham>>>>>> for Discovery {
	fn install(app: &mut App, _config: &ChicoConfig) {
		install_vegetation_stream::<Discovery, Chico<Urbanization<Richmond<OnTerrain<Durham>>>>>(
			app,
		);
	}
}

impl Scheme<Mobs<Barking<Vegetation<Chico<Urbanization<Richmond<OnTerrain<Durham>>>>>>>>
	for Discovery
{
	fn install(app: &mut App, _config: &BarkingConfig) {
		install_mob_grid_stream::<Discovery>(app);
	}
}

impl Scheme<Furnishing<Maputo<Urbanization<Richmond<OnTerrain<Durham>>>>>> for Discovery {
	fn install(app: &mut App, _config: &()) {
		install_furnishing_stream::<Discovery, Urbanization<Richmond<OnTerrain<Durham>>>>(app);
	}
}

impl Scheme<Language<Geneva<Vegetation<Chico<Urbanization<Richmond<OnTerrain<Durham>>>>>>>>
	for Discovery
{
	fn install(app: &mut App, _config: &LanguageConfig) {
		install_language_stream::<
			Discovery,
			Vegetation<Chico<Urbanization<Richmond<OnTerrain<Durham>>>>>,
		>(app);
	}
}

/// Discovery's player home is the origin. Waypoints stay; a respawn walks to a POI.
pub struct DiscoveryPlayerPlugin;

impl Plugin for DiscoveryPlayerPlugin {
	fn build(&self, app: &mut App) {
		let mut policies = app.world_mut().get_resource_or_insert_with(ModePlayerPolicies::default);
		policies.register(TypeId::of::<Discovery>(), discovery_player_policy());
	}
}

fn discovery_player_policy() -> ModePlayerPolicy {
	ModePlayerPolicy { home: Vec2::ZERO, keep_waypoints: true, respawn_ends_life: false }
}

fn restore_playable_world(
	config: Res<LayerModeConfig<Discovery, OnTerrain<Durham>>>,
	mut terrain: TerrainRetarget,
) {
	if terrain.coverage() == config.config.coverage {
		return;
	}
	terrain.apply(
		playable_world_cell_layout(),
		config.config.coverage,
		config.config.terrain_radius,
		false,
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use durham::{
		TerrainCellLayout, TerrainConfig, TerrainCoverage, TerrainLayoutPinned,
		TerrainPresentPending, TerrainPresentationAssets, TerrainPresentationDirty,
		WORLD_FINE_HALF_EXTENT_CELLS,
	};

	fn playable_assets() -> TerrainPresentationAssets {
		TerrainPresentationAssets {
			config: TerrainConfig::new(42),
			material: Handle::default(),
			lod_bands: Vec::new(),
			outer_add_walls: true,
			fine_grid_max_radius: Some(WORLD_FINE_HALF_EXTENT_CELLS),
			macro_seam_half_extents: Vec::new(),
			macro_cell_min_size: None,
			macro_res_2: None,
		}
	}

	fn world_with(coverage: TerrainCoverage, layout: TerrainCellLayout) -> World {
		let mut world = World::new();
		world.insert_resource(LayerModeConfig::<Discovery, OnTerrain<Durham>>::new(
			DurhamTerrainConfig::playable_world(),
		));
		world.insert_resource(layout);
		world.insert_resource(coverage);
		world.insert_resource(TerrainLayoutPinned(coverage == TerrainCoverage::FinePatch));
		world.insert_resource(TerrainPresentationDirty(false));
		world.insert_resource(TerrainPresentPending(false));
		world.insert_resource(playable_assets());
		world
	}

	#[test]
	fn already_playable_coverage_is_a_noop() -> anyhow::Result<()> {
		let mut world = world_with(TerrainCoverage::PlayableWorld, playable_world_cell_layout());
		world
			.run_system_once(restore_playable_world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(!world.resource::<TerrainPresentationDirty>().0);
		anyhow::ensure!(!world.resource::<TerrainPresentPending>().0);
		anyhow::ensure!(!world.resource::<TerrainLayoutPinned>().0);
		anyhow::ensure!(*world.resource::<TerrainCellLayout>() == playable_world_cell_layout());
		Ok(())
	}

	#[test]
	fn leaving_a_fine_patch_restores_the_playable_rings() -> anyhow::Result<()> {
		let mut world =
			world_with(TerrainCoverage::FinePatch, durham::fine_patch_cell_layout(2, IVec2::ZERO));
		world
			.run_system_once(restore_playable_world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(*world.resource::<TerrainCellLayout>() == playable_world_cell_layout());
		anyhow::ensure!(*world.resource::<TerrainCoverage>() == TerrainCoverage::PlayableWorld);
		anyhow::ensure!(!world.resource::<TerrainLayoutPinned>().0);
		anyhow::ensure!(world.resource::<TerrainPresentationDirty>().0);
		anyhow::ensure!(world.resource::<TerrainPresentPending>().0);
		Ok(())
	}

	#[test]
	fn discovery_streams_only_inside_the_world_shell() -> anyhow::Result<()> {
		anyhow::ensure!(streams_terrain(true, true));
		anyhow::ensure!(!streams_terrain(true, false));
		anyhow::ensure!(!streams_terrain(false, true));
		Ok(())
	}
}
