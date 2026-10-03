//! [`Maputo<G>`]: furnishing over a ground that exposes [`FurnitureSlots`].

use std::marker::PhantomData;

use bevy::prelude::{App, World};
use furnishing_layer_model::{Furnishing, FurnishingGeneration, FurnishingModel};
use layer_stack::{LayerGenerationCore, RequireLayer};
use terrain_layer_model::TerrainModel;

use crate::host::FurnitureCell;
use crate::index::FurnitureIndex;
use crate::present::{install_maputo_presentation, FurnitureLodChan};
use crate::slots::FurnitureSlots;
use crate::stream::register_furniture_generate;

/// Furnishing model over ground `G`.
pub struct Maputo<G>(PhantomData<fn() -> G>);

impl<G> FurnishingModel for Maputo<G>
where
	G: FurnitureSlots + TerrainModel,
{
	type Ground = G;
	type Cell = FurnitureCell;

	fn require_generation(app: &App) {
		G::require_generation(app);
		app.require_layer::<LayerGenerationCore<Furnishing<Self>>, Furnishing<Self>>();
	}
}

impl<G> FurnishingGeneration for Maputo<G>
where
	G: FurnitureSlots + TerrainModel,
{
	const LABEL: &'static str = "maputo";
	type Config = ();

	fn install_generation(app: &mut App) {
		register_furniture_generate(app);
	}

	fn apply_generation(_world: &mut World, _config: &()) {}

	fn clear_generation(world: &mut World) {
		if let Some(mut index) = world.get_resource_mut::<FurnitureIndex>() {
			index.clear();
		}
	}

	fn install_presentation(app: &mut App) {
		layer_stack::install_lod_present_gate::<Furnishing<Self>, FurnitureLodChan>(app);
		install_maputo_presentation(app);
	}
}
