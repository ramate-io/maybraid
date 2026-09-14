//! Furniture shaders: named [`MaterialRef`] surface recipes.
//!
//! Parallel to Chico vegetation-shaders / Richmond urban-surface: one PBR
//! look graph keyed by recipe (`furniture_wood`, `furniture_cloth`,
//! `furniture_soft`, `furniture_marble`, `furniture_ornate`). Grain, veins,
//! and gold inlay are procedural — BotW-warm, not urban-mud.

use bevy::prelude::*;

mod furniture_surface;
mod material_lib;

pub use furniture_surface::{
	is_furniture_surface_recipe, FurnitureSurfaceKind, FurnitureSurfaceMaterial,
	FurnitureSurfaceMaterialPlugin, FurnitureSurfaceUniform, KIND_CLOTH, KIND_MARBLE, KIND_ORNATE,
	KIND_SOFT, KIND_WOOD, RECIPE_FURNITURE_CLOTH, RECIPE_FURNITURE_MARBLE, RECIPE_FURNITURE_ORNATE,
	RECIPE_FURNITURE_SOFT, RECIPE_FURNITURE_WOOD,
};
pub use material_lib::{
	init_furniture_material_caches, FurnitureMaterialLib, FurnitureMaterialRefPlugin,
	FurnitureStandaloneMaterialLib, FurnitureSurfaceMaterialRefCache,
};

/// Registers furniture-surface materials used by furniture [`material_ref::MaterialLib`]s.
pub struct FurnitureShadersPlugin;

impl Plugin for FurnitureShadersPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(FurnitureSurfaceMaterialPlugin);
	}
}
