//! [`Furnishing`] wrapper. Nothing wraps it, so it does not implement [`TerrainModel`].

use std::marker::PhantomData;

use bevy::prelude::{App, Update, World};
use layer_stack::{Layer, LayerPresentation, LayerSystems};

use terrain_layer_model::TerrainModel;

use crate::generation::{FurnishingGeneration, FurnishingGenerationSystems};

/// Model `F` after furnishing. A sibling over urbanization, not a ground.
pub struct Furnishing<F>(PhantomData<fn() -> F>);

impl<F: FurnishingGeneration> Layer for Furnishing<F> {
	const LABEL: &'static str = F::LABEL;
	type Config = F::Config;

	fn install_generation(app: &mut App) {
		F::install_generation(app);
		app.configure_sets(Update, FurnishingGenerationSystems);
		app.configure_sets(Update, LayerSystems::<Furnishing<F>>::default());
	}

	fn apply_generation(world: &mut World, config: &Self::Config) {
		F::apply_generation(world, config);
	}

	fn clear_generation(world: &mut World) {
		F::clear_generation(world);
	}

	fn require_lower(app: &App) {
		F::Ground::require_generation(app);
	}
}

impl<F: FurnishingGeneration> LayerPresentation for Furnishing<F> {
	fn install_presentation(app: &mut App) {
		F::install_presentation(app);
	}
}
