//! Richmond building shaders: named [`MaterialRef`] urban-surface recipes.

use bevy::prelude::*;

mod material_lib;
mod urban_surface;

pub use material_lib::{
	init_richmond_urban_material_caches, RichmondUrbanMaterialLib, RichmondUrbanMaterialRefPlugin,
	UrbanSurfaceMaterialLib, UrbanSurfaceMaterialRefCache,
};
pub use urban_surface::{
	is_urban_surface_recipe, UrbanSurfaceKind, UrbanSurfaceMaterial, UrbanSurfaceMaterialPlugin,
	UrbanSurfaceUniform, KIND_HAY, KIND_IRON, KIND_STUCCO, KIND_TERRACOTTA, KIND_WOOD, RECIPE_HAY,
	RECIPE_IRON, RECIPE_STUCCO, RECIPE_TERRACOTTA, RECIPE_WOOD,
};

/// Registers urban-surface materials used by Richmond [`material_ref::MaterialLib`]s.
pub struct RichmondBuildingShadersPlugin;

impl Plugin for RichmondBuildingShadersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(UrbanSurfaceMaterialPlugin);
	}
}
