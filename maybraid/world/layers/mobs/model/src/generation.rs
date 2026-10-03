//! Mob generation: model hooks plus leftover system-set labels.

use bevy::app::App;
use bevy::prelude::*;

use crate::mob::MobModel;

/// Systems that arm mob keep regions and sync selection models.
///
/// Scheme writes order `.before` LOD generate / present produce through this set.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MobGenerationSystems;

/// A model with its own mob generation stack.
pub trait MobGeneration: MobModel {
	const LABEL: &'static str;
	type Config: Clone + Send + Sync + 'static;

	fn install_generation(app: &mut App);

	fn apply_generation(world: &mut World, config: &Self::Config);

	fn clear_generation(world: &mut World);

	fn install_presentation(app: &mut App);
}
