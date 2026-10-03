//! [`Geneva<W>`]: language over a vegetated world that exposes [`NamedWorld`].

use std::marker::PhantomData;

use bevy::prelude::{App, World};
use language_layer_model::{Language, LanguageGeneration, LanguageModel};
use layer_stack::{LayerGenerationCore, RequireLayer};
use terrain_layer_model::TerrainModel;

use crate::index::LanguageIndex;
use crate::present::install_geneva_presentation;
use crate::sources::NamedWorld;
use crate::stream::{apply_origin_tiles, register_language_generate};
use crate::tiles::LargeTile;

/// Language model over vegetated world `W`.
pub struct Geneva<W>(PhantomData<fn() -> W>);

impl<W> LanguageModel for Geneva<W>
where
	W: NamedWorld + TerrainModel,
{
	type World = W;
	type Cell = LargeTile;

	fn require_generation(app: &App) {
		W::require_generation(app);
		app.require_layer::<LayerGenerationCore<Language<Self>>, Language<Self>>();
	}
}

impl<W> LanguageGeneration for Geneva<W>
where
	W: NamedWorld + TerrainModel,
{
	const LABEL: &'static str = "geneva";
	type Config = ();

	fn install_generation(app: &mut App) {
		register_language_generate(app);
	}

	fn apply_generation(world: &mut World, _config: &()) {
		apply_origin_tiles(world);
	}

	fn clear_generation(world: &mut World) {
		if let Some(mut index) = world.get_resource_mut::<LanguageIndex>() {
			index.clear();
		}
	}

	fn install_presentation(app: &mut App) {
		install_geneva_presentation(app);
	}
}
