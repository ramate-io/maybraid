//! Furnishing generation: model hooks plus leftover system-set labels.

use bevy::app::App;
use bevy::prelude::*;

use crate::furnishing::FurnishingModel;

/// Systems that keep the furniture neighborhood and bin occupied cells.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct FurnishingGenerationSystems;

/// A model with its own furnishing generation stack.
pub trait FurnishingGeneration: FurnishingModel {
	const LABEL: &'static str;
	type Config: Clone + Send + Sync + 'static;

	fn install_generation(app: &mut App);

	fn apply_generation(world: &mut World, config: &Self::Config);

	fn clear_generation(world: &mut World);

	fn install_presentation(app: &mut App);
}
