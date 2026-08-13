//! LodScene refresh for character hosts (High-only bands for now).
//!
//! Nested [`RigNode`] / [`PartNode`] are registered once. Each species host is
//! `ComponentsOnly<Clothed<T>>` via [`add_character_components_host`].

use bevy::prelude::*;
use crozon_characters::{
	add_character_components_host,
	species::{
		braidman::bsn::Braidman, brenal::bsn::Brenal, brodler::bsn::Brodler, brokker::bsn::Brokker,
		caole::bsn::Caole, chupri::bsn::Chupri, claber::bsn::Claber, croconot::bsn::Croconot,
		dui::bsn::Dui, epiphant::bsn::Epiphant, grener::bsn::Grener, hars::bsn::Hars,
		kaller::bsn::Kaller, kappler::bsn::Kappler, kispar::bsn::Kispar, lero::bsn::Lero,
		lidder::bsn::Lidder, mistler::bsn::Mistler, mygr::bsn::Mygr, sonyak::bsn::Sonyak,
		spibmom::bsn::Spibmom, tapp::bsn::Tapp, thumplus::bsn::Thumplus, tipple::bsn::Tipple,
		topple::bsn::Topple, tuberwaber::bsn::Tuberwaber, wumbus::bsn::Wumbus, ylter::bsn::Yilter,
	},
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
		add_character_components_host::<Clothed<Brenal>>(app);
		add_character_components_host::<Clothed<Brodler>>(app);
		add_character_components_host::<Clothed<Brokker>>(app);
		add_character_components_host::<Clothed<Caole>>(app);
		add_character_components_host::<Clothed<Chupri>>(app);
		add_character_components_host::<Clothed<Claber>>(app);
		add_character_components_host::<Clothed<Croconot>>(app);
		add_character_components_host::<Clothed<Dui>>(app);
		add_character_components_host::<Clothed<Epiphant>>(app);
		add_character_components_host::<Clothed<Grener>>(app);
		add_character_components_host::<Clothed<Hars>>(app);
		add_character_components_host::<Clothed<Kaller>>(app);
		add_character_components_host::<Clothed<Kappler>>(app);
		add_character_components_host::<Clothed<Kispar>>(app);
		add_character_components_host::<Clothed<Lero>>(app);
		add_character_components_host::<Clothed<Lidder>>(app);
		add_character_components_host::<Clothed<Mistler>>(app);
		add_character_components_host::<Clothed<Mygr>>(app);
		add_character_components_host::<Clothed<Sonyak>>(app);
		add_character_components_host::<Clothed<Spibmom>>(app);
		add_character_components_host::<Clothed<Tapp>>(app);
		add_character_components_host::<Clothed<Thumplus>>(app);
		add_character_components_host::<Clothed<Tipple>>(app);
		add_character_components_host::<Clothed<Topple>>(app);
		add_character_components_host::<Clothed<Tuberwaber>>(app);
		add_character_components_host::<Clothed<Wumbus>>(app);
		add_character_components_host::<Clothed<Yilter>>(app);
	}
}
