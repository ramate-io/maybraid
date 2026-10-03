//! Language generation: model hooks plus leftover system-set labels.

use bevy::app::App;
use bevy::prelude::*;

use crate::language::LanguageModel;

/// Systems that keep language tiles and assign names in the keep ring.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LanguageGenerationSystems;

/// A model with its own language generation stack.
pub trait LanguageGeneration: LanguageModel {
	const LABEL: &'static str;
	type Config: Clone + Send + Sync + 'static;

	fn install_generation(app: &mut App);

	fn apply_generation(world: &mut World, config: &Self::Config);

	fn clear_generation(world: &mut World);

	fn install_presentation(app: &mut App);
}
