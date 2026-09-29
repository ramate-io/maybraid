//! [`UrbanizationGenerationPlugin`]: urbanization cells, pads, and padded cells over `M`.

use std::marker::PhantomData;

use bevy::app::{App, Plugin};
use bevy::prelude::*;
use durham_terrain_models::{terrain_streaming_enabled, TerrainColliderSystems};
use lod::{LodGenerateSystems, LodPresentSystems};
use richmond_development_models::RichmondDevelopmentModelsPlugin;
use terrain_layer_model::TerrainModel;

use crate::config::UrbanizationLayerConfig;
use crate::model::Urbanization;
use crate::stream::{
	generate_urbanization_developments, generate_urbanization_padded_terrain,
	register_urbanization_lod_generate, stream_urbanization, sync_urbanization_pin,
	urbanization_streaming_enabled, UrbanizationStreamingEnabled,
};

/// Systems that write urbanization / development / padded-cell storage.
///
/// Presentation orders `.after(UrbanizationGenerationSystems)` so the split
/// chain matches today's single `.chain()`.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UrbanizationGenerationSystems;

/// Generates urbanization over model `M` and makes `Urbanization<M>` available.
///
/// Reads `M` (pad heights sample the inner surface, never pads). Writes
/// urbanization cells, development cells, built developments, and padded cells.
pub struct UrbanizationGenerationPlugin<M> {
	pub config: UrbanizationLayerConfig,
	_marker: PhantomData<fn() -> M>,
}

impl<M> UrbanizationGenerationPlugin<M> {
	pub fn new(config: UrbanizationLayerConfig) -> Self {
		Self { config, _marker: PhantomData }
	}
}

impl<M> Default for UrbanizationGenerationPlugin<M> {
	fn default() -> Self {
		Self::new(UrbanizationLayerConfig::default())
	}
}

impl<M> Plugin for UrbanizationGenerationPlugin<M>
where
	M: TerrainModel,
	Urbanization<M>: TerrainModel,
{
	fn build(&self, app: &mut App) {
		let development_config = self.config.development_config();
		app.add_plugins(RichmondDevelopmentModelsPlugin)
			.insert_resource(self.config.clone())
			.insert_resource(development_config);
		if !app.world().contains_resource::<UrbanizationStreamingEnabled>() {
			app.init_resource::<UrbanizationStreamingEnabled>();
		}
		register_urbanization_lod_generate(app, self.config.generate_budget);

		// The pin stays ungated: other readers of `UrbanizationIndex` (world mobs)
		// select cells before terrain streaming starts, and must see the spec noise.
		app.add_systems(
			Update,
			sync_urbanization_pin
				.in_set(UrbanizationGenerationSystems)
				.before(stream_urbanization)
				.before(LodGenerateSystems::Produce),
		);
		// Stream still runs while urbanization is off so a session that turns
		// it off (Training) tears generate state down instead of freezing it.
		app.add_systems(
			Update,
			(
				stream_urbanization.before(LodGenerateSystems::Produce),
				(
					generate_urbanization_developments.after(LodGenerateSystems::Drain),
					generate_urbanization_padded_terrain,
				)
					.chain()
					.run_if(urbanization_streaming_enabled),
			)
				.chain()
				.in_set(UrbanizationGenerationSystems)
				.run_if(terrain_streaming_enabled)
				.before(LodPresentSystems::Produce)
				.before(TerrainColliderSystems::QueueMeshes),
		);
	}

	fn finish(&self, app: &mut App) {
		M::require_generation(app);
	}
}
