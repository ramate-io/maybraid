//! Vegetation generation: model hooks plus leftover system-set labels.

use bevy::app::App;
use bevy::prelude::*;

use crate::vegetation::VegetationModel;

/// Systems that arm forest and bump-out keep regions.
///
/// Callers that edit the layer config order `.before` this set.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct VegetationGenerationSystems;

/// A model with its own vegetation generation stack.
pub trait VegetationGeneration: VegetationModel {
	const LABEL: &'static str;
	type Config: Clone + Send + Sync + 'static;

	fn install_generation(app: &mut App);

	fn apply_generation(world: &mut World, config: &Self::Config);

	fn clear_generation(world: &mut World);

	fn install_presentation(app: &mut App);
}
