//! LodScene refresh for character hosts (High-only bands for now).
//!
//! Nested [`RigNode`] / [`PartNode`] are registered once. Each species host is
//! `ComponentsOnly<Clothed<T>>` via [`add_character_components_host`].

use bevy::prelude::*;
use crozon_characters::{
	add_character_components_host,
	species::{braidman::bsn::Braidman, brodler::bsn::Brodler},
	CharacterComponentsPlugin, Clothed, PartNode, RigNode,
};
use lod::{add_lod_refresh_chunk_for, LodRefreshCorePlugin};
use scene_ref::SceneRefPlugin;

/// Chunk fulfill + scene-ref + character socket/skin plugin for LodScene hosts.
pub struct CharacterLodPlugin;

impl Plugin for CharacterLodPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<SceneRefPlugin>() {
			app.add_plugins(SceneRefPlugin);
		}
		if !app.is_plugin_added::<LodRefreshCorePlugin>() {
			app.add_plugins(LodRefreshCorePlugin);
		}
		if !app.is_plugin_added::<CharacterComponentsPlugin>() {
			app.add_plugins(CharacterComponentsPlugin);
		}
		add_lod_refresh_chunk_for::<RigNode>(app);
		add_lod_refresh_chunk_for::<PartNode>(app);
		add_character_components_host::<Clothed<Braidman>>(app);
		add_character_components_host::<Clothed<Brodler>>(app);
	}
}
