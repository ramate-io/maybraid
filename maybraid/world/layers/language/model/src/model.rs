//! [`Language`] wrapper. Nothing wraps it, so it does not implement [`terrain_layer_model::TerrainModel`].

use std::marker::PhantomData;

use bevy::prelude::{App, Update, World};
use layer_stack::{Layer, LayerPresentation, LayerSystems};
use terrain_layer_model::TerrainModel;

use crate::generation::{LanguageGeneration, LanguageGenerationSystems};

/// Model `L` after language. A sibling over vegetation, not a ground.
pub struct Language<L>(PhantomData<fn() -> L>);

impl<L: LanguageGeneration> Layer for Language<L> {
	const LABEL: &'static str = L::LABEL;
	type Config = L::Config;

	fn install_generation(app: &mut App) {
		L::install_generation(app);
		app.configure_sets(Update, LanguageGenerationSystems);
		app.configure_sets(Update, LayerSystems::<Language<L>>::default());
	}

	fn apply_generation(world: &mut World, config: &Self::Config) {
		L::apply_generation(world, config);
	}

	fn clear_generation(world: &mut World) {
		L::clear_generation(world);
	}

	fn require_lower(app: &App) {
		L::World::require_generation(app);
	}
}

impl<L: LanguageGeneration> LayerPresentation for Language<L> {
	fn install_presentation(app: &mut App) {
		L::install_presentation(app);
	}
}
