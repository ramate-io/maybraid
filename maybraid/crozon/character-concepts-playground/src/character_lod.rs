//! LodScene refresh for Braidman character hosts (High-only bands for now).

use bevy::prelude::*;
use crozon_characters::{
	species::braidman::bsn::Braidman, CharacterComponentsPlugin, Clothed, ComponentsOnly, PartNode,
	RigNode,
};
use lod::{add_lod_refresh_chunk_for, LodRefreshCorePlugin};
use scene_ref::SceneRefPlugin;

/// Structural host type spawned for Braidman previews.
pub type BraidmanHost = ComponentsOnly<Clothed<Braidman>>;

/// Chunk fulfill + scene-ref + character socket/skin plugin for Braidman LodScene hosts.
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
		add_lod_refresh_chunk_for::<BraidmanHost>(app);
		add_lod_refresh_chunk_for::<RigNode>(app);
		add_lod_refresh_chunk_for::<PartNode>(app);
	}
}
