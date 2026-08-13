//! Nested [`LodScene`] host registration for character recipes.

use bevy::prelude::*;
use lod::{add_lod_refresh_chunk_for, LodScene};

use crate::components::{CharacterComponents, ComponentsOnly};

/// Register chunk fulfill for a structural [`ComponentsOnly<C>`] host.
///
/// [`crate::RigNode`] / [`crate::PartNode`] stamp socket, skin, pose, and material
/// refs from [`lod::LodScene::host`] / [`lod::LodScene::scene_with_level`]. Each
/// species only adds its recipe type here (typically [`crate::Clothed<T>`]).
pub fn add_character_components_host<C>(app: &mut App)
where
	C: CharacterComponents + Send + Sync + 'static,
	ComponentsOnly<C>: Component + LodScene,
{
	add_lod_refresh_chunk_for::<ComponentsOnly<C>>(app);
}

/// Nested character hosts are spawned as LodScene; no post-fulfill prepare pass.
pub struct CharacterComponentsPlugin;

impl Plugin for CharacterComponentsPlugin {
	fn build(&self, _app: &mut App) {}
}
