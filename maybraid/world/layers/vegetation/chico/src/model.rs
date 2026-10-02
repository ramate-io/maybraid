//! [`Chico<G>`]: vegetation over ground `G`.

use std::marker::PhantomData;

use bevy::prelude::{App, World};
use layer_stack::RequireLayer;
use lod::gen::LodGenerateBudget;
use vegetation_layer_model::{
	Vegetation, VegetationGeneration, VegetationGenerationCore, VegetationModel,
};

use crate::config::ChicoConfig;
use crate::generation::{BumpOutLodChan, ForestLodChan, MediumBumpOutLodChan};
use crate::ground::ChicoGround;
use crate::layer_stream::{
	clear_vegetation_stream_world, register_bump_out_generate, register_forest_generate,
};

/// Vegetation model over ground `G`.
pub struct Chico<G>(PhantomData<fn() -> G>);

impl<G: ChicoGround> VegetationModel for Chico<G> {
	type Ground = G;

	fn require_generation(app: &App) {
		app.require_layer::<VegetationGenerationCore<Self>, Vegetation<Self>>();
	}
}

impl<G: ChicoGround> VegetationGeneration for Chico<G> {
	type Config = ChicoConfig;

	fn install_generation(app: &mut App) {
		register_forest_generate(app);
		register_bump_out_generate(app);
	}

	fn apply_generation(world: &mut World, config: &ChicoConfig) {
		*world.resource_mut::<LodGenerateBudget<ForestLodChan>>() =
			LodGenerateBudget::new(config.forest_budget);
		*world.resource_mut::<LodGenerateBudget<BumpOutLodChan>>() =
			LodGenerateBudget::new(config.bump_out_budget);
		*world.resource_mut::<LodGenerateBudget<MediumBumpOutLodChan>>() =
			LodGenerateBudget::new(config.medium_bump_out_budget);
	}

	fn clear_generation(world: &mut World) {
		clear_vegetation_stream_world(world);
	}
}
