//! Training Ground is a seeded FinePatch of the Maybraid world.
//!
//! The shell requests this mode on [`layer_stack::ActiveGenerationMode`].
//! Terrain layout is [`Scheme`](layer_stack::Scheme)`<OnTerrain<Durham>>`
//! on [`TrainingGround`].

use bevy::prelude::*;
use chico::{install_vegetation_stream, Chico, ChicoConfig};
use durham::{
	Durham, DurhamTerrainConfig, TerrainColliderSystems, TerrainFillSystems, TerrainRetarget,
};
use furnishing_layer_model::Furnishing;
use geneva::{install_language_stream, Geneva};
use language_layer_model::Language;
use layer_stack::{
	in_generation_mode, ActiveGenerationMode, GenerationMode, GenerationModeSystems,
	LayerModeConfig, Scheme,
};
use maputo::{install_furnishing_stream, Maputo};
use richmond::Richmond;
use terrain_layer_model::OnTerrain;
use urbanization_layer_model::Urbanization;
use vegetation_layer_model::Vegetation;
use world_player::{PlayerLifeEnded, PlayerLifeSet};

mod arena;
mod markers;
mod mobs;
mod plaza;
mod round;
mod session;
mod urbanization;

pub use arena::TrainingArena;
pub use markers::TrainingEnemyMarkersEnabled;
pub use mobs::TrainingBrawler;
pub use round::{TrainingMap, TrainingRound, TRAINING_FINE_HALF_EXTENT_CELLS};
pub use session::{training_trainee, TrainingLifeEnded, TrainingSessionSet, TrainingTrainee};
pub use urbanization::{
	pad_influence_region, terrain_ids_under_pads, training_development_cell, TrainingPlazaStamped,
	TrainingStampSettled, TRAINING_ARENA_MARGIN_M, TRAINING_ARENA_MAX_HALF_M,
	TRAINING_COURTYARD_EASE_M, TRAINING_COURTYARD_OVERHANG_M,
};

/// Pinned FinePatch generation. Layers take this as a plugin parameter.
pub struct TrainingGround;

impl GenerationMode for TrainingGround {}

impl Scheme<Vegetation<Chico<Urbanization<Richmond<OnTerrain<Durham>>>>>> for TrainingGround {
	fn install(app: &mut App, _config: &ChicoConfig) {
		install_vegetation_stream::<TrainingGround, Chico<Urbanization<Richmond<OnTerrain<Durham>>>>>(
			app,
		);
	}
}

impl Scheme<Furnishing<Maputo<Urbanization<Richmond<OnTerrain<Durham>>>>>> for TrainingGround {
	fn install(app: &mut App, _config: &()) {
		install_furnishing_stream::<TrainingGround, Urbanization<Richmond<OnTerrain<Durham>>>>(app);
	}
}

impl Scheme<Language<Geneva<Vegetation<Chico<Urbanization<Richmond<OnTerrain<Durham>>>>>>>>
	for TrainingGround
{
	fn install(app: &mut App, _config: &()) {
		install_language_stream::<
			TrainingGround,
			Vegetation<Chico<Urbanization<Richmond<OnTerrain<Durham>>>>>,
		>(app);
	}
}

impl Scheme<OnTerrain<Durham>> for TrainingGround {
	fn install(app: &mut App, _config: &DurhamTerrainConfig) {
		app.add_systems(
			Update,
			apply_training_patch
				.in_set(GenerationModeSystems::<TrainingGround>::default())
				.before(TerrainFillSystems::Generate),
		);
	}
}

fn apply_training_patch(
	round: Option<Res<TrainingRound>>,
	config: Res<LayerModeConfig<TrainingGround, OnTerrain<Durham>>>,
	mut terrain: TerrainRetarget,
) {
	let Some(round) = round else {
		return;
	};
	if terrain.coverage() == config.config.coverage && terrain.layout() == &round.layout() {
		return;
	}
	terrain.apply(round.layout(), config.config.coverage, config.config.terrain_radius, true);
}

/// Plaza, score, markers, and the trainee roll for a Training session.
pub struct TrainingGroundPlugin;

impl Plugin for TrainingGroundPlugin {
	fn build(&self, app: &mut App) {
		session::register_training_policy(app);
		app.init_resource::<TrainingRound>()
			.init_resource::<TrainingEnemyMarkersEnabled>()
			.add_message::<TrainingLifeEnded>()
			.add_message::<PlayerLifeEnded>()
			.configure_sets(Update, PlayerLifeSet::Resolve)
			.configure_sets(Update, TrainingSessionSet::LifeEnded.after(PlayerLifeSet::Resolve))
			.add_systems(
				OnEnter(ActiveGenerationMode::of::<TrainingGround>()),
				session::open_training_score,
			)
			.add_systems(
				OnExit(ActiveGenerationMode::of::<TrainingGround>()),
				(
					session::close_training_score,
					session::clear_training_live_enemies,
					plaza::request_training_leave,
				),
			)
			.add_systems(
				Update,
				(
					(
						plaza::park_on_training_site,
						plaza::mount_training_plaza,
						plaza::promote_training_plaza.after(TerrainColliderSystems::QueueMeshes),
						plaza::reseat_training_life,
						session::count_training_enemies,
						markers::sync_training_enemy_markers,
					)
						.run_if(in_generation_mode::<TrainingGround>()),
					session::note_training_life_ended
						.in_set(TrainingSessionSet::LifeEnded)
						.run_if(in_generation_mode::<TrainingGround>()),
				),
			)
			// Wall fixtures and the player anchor tear down in Last so combat
			// commands queued through PostUpdate still find their targets.
			.add_systems(
				Last,
				(
					plaza::clear_stale_training_plaza
						.run_if(in_generation_mode::<TrainingGround>()),
					(plaza::finish_training_leave, markers::despawn_training_enemy_markers)
						.run_if(resource_exists::<plaza::TrainingLeave>),
				),
			);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use durham::{
		playable_world_cell_layout, TerrainCellLayout, TerrainConfig, TerrainCoverage,
		TerrainLayoutPinned, TerrainPresentPending, TerrainPresentationAssets,
		TerrainPresentationDirty, WORLD_FINE_HALF_EXTENT_CELLS,
	};

	fn assets() -> TerrainPresentationAssets {
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

	fn playable_world(round: TrainingRound) -> World {
		let mut world = World::new();
		world.insert_resource(round);
		world.insert_resource(LayerModeConfig::<TrainingGround, OnTerrain<Durham>>::new(
			DurhamTerrainConfig::fine_patch(TRAINING_FINE_HALF_EXTENT_CELLS),
		));
		world.insert_resource(playable_world_cell_layout());
		world.insert_resource(TerrainCoverage::PlayableWorld);
		world.insert_resource(TerrainLayoutPinned(false));
		world.insert_resource(TerrainPresentationDirty(false));
		world.insert_resource(TerrainPresentPending(false));
		world.insert_resource(assets());
		world
	}

	#[test]
	fn entering_pins_the_round_site() -> anyhow::Result<()> {
		let round = TrainingRound::new(7);
		let mut world = playable_world(round);
		world
			.run_system_once(apply_training_patch)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(*world.resource::<TerrainCellLayout>() == round.layout());
		anyhow::ensure!(*world.resource::<TerrainCoverage>() == TerrainCoverage::FinePatch);
		anyhow::ensure!(world.resource::<TerrainLayoutPinned>().0);
		anyhow::ensure!(world.resource::<TerrainPresentationDirty>().0);
		anyhow::ensure!(world.resource::<TerrainPresentPending>().0);
		Ok(())
	}

	#[test]
	fn a_new_map_moves_the_patch_and_a_new_life_does_not() -> anyhow::Result<()> {
		let round = TrainingRound::new(7);
		let mut world = playable_world(round);
		world
			.run_system_once(apply_training_patch)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.resource_mut::<TerrainPresentationDirty>().0 = false;

		world
			.run_system_once(apply_training_patch)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(!world.resource::<TerrainPresentationDirty>().0, "same round stays put");

		world.insert_resource(round.next_life());
		world
			.run_system_once(apply_training_patch)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(
			!world.resource::<TerrainPresentationDirty>().0,
			"a new life keeps the map"
		);
		anyhow::ensure!(*world.resource::<TerrainCellLayout>() == round.layout());

		world.insert_resource(round.next());
		world
			.run_system_once(apply_training_patch)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(*world.resource::<TerrainCellLayout>() == round.next().layout());
		anyhow::ensure!(world.resource::<TerrainPresentationDirty>().0);
		Ok(())
	}

	#[test]
	fn the_round_layout_is_a_pinned_fine_patch() -> anyhow::Result<()> {
		let round = TrainingRound::new(3);
		let layout = round.layout();
		let site = round.site();
		let center = layout.region_center_xz();
		let cell = layout.cell_size;
		anyhow::ensure!((center.x - site.x as f32 * cell).abs() < 1e-3);
		anyhow::ensure!((center.z - site.y as f32 * cell).abs() < 1e-3);
		let side = (2 * TRAINING_FINE_HALF_EXTENT_CELLS) as u32;
		anyhow::ensure!(layout.extents == bevy::math::UVec2::new(side, side));
		Ok(())
	}
}
