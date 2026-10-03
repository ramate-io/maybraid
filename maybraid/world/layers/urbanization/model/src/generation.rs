//! Urbanization generation: model hooks plus leftover store / region types.

use bevy::app::App;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::urban::UrbanizationModel;

/// Systems that write urbanization storage.
///
/// Presentation orders `.after(UrbanizationGenerationSystems)` so the split
/// chain matches today's single `.chain()`.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UrbanizationGenerationSystems;

/// A model with its own urbanization generation stack.
pub trait UrbanizationGeneration: UrbanizationModel {
	const LABEL: &'static str;
	type Config: Clone + Send + Sync + 'static;

	fn install_generation(app: &mut App);

	fn apply_generation(world: &mut World, config: &Self::Config);

	fn clear_generation(world: &mut World);

	fn install_presentation(app: &mut App);
}

/// Region schemes write and padding / hosts read.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct UrbanizationLayerRegion {
	pub region: Option<Aabb3d>,
}
