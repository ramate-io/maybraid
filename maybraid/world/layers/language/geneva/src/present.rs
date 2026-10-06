//! Debug overlay of tile bounds and assigned names. Not a mesh stream.

use bevy::math::{Rect, Vec2};
use bevy::prelude::{App, IntoScheduleConfigs, Local, Res, ResMut, Resource, Update};
use language_layer_model::Language;
use lod::lod_present_gate_open;
use terrain_layer_model::TerrainModel;

use crate::index::{LanguageIndex, NameKey};
use crate::name::AssignedName;
use crate::sources::NamedWorld;
use crate::Geneva;

/// Channel marker for language generate / present keep.
#[derive(Debug, Clone, Copy, Default)]
pub struct LanguageLodChan;

/// Tile bounds and names for a debug overlay.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct LanguageOverlay {
	pub large_tiles: Vec<LargeTileOverlay>,
	pub names: Vec<NamedOverlay>,
}

/// One large tile in the overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LargeTileOverlay {
	pub ix: i32,
	pub iz: i32,
	pub language_count: usize,
	pub small_count: usize,
}

/// One assigned name in the overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct NamedOverlay {
	pub key: NameKey,
	pub surface: String,
	pub english: Vec<String>,
	pub provisional: bool,
	pub xz: Vec2,
	pub extent: Rect,
}

impl LanguageOverlay {
	pub fn from_index(index: &LanguageIndex) -> Self {
		let large_tiles = index
			.large_tiles()
			.map(|tile| LargeTileOverlay {
				ix: tile.ix,
				iz: tile.iz,
				language_count: tile.languages.len(),
				small_count: tile.small.len(),
			})
			.collect();
		let names = index
			.assigned_names()
			.filter_map(|(key, assigned): (NameKey, &AssignedName)| {
				if !index.is_active(key) {
					return None;
				}
				Some(NamedOverlay {
					key,
					surface: assigned.name.surface.clone(),
					english: assigned.name.english.clone(),
					provisional: assigned.provisional,
					xz: index.anchor(key)?,
					extent: index.extent(key)?,
				})
			})
			.collect();
		Self { large_tiles, names }
	}
}

fn present_language_overlay(
	index: Res<LanguageIndex>,
	mut overlay: ResMut<LanguageOverlay>,
	mut last_epoch: Local<u64>,
) {
	if index.epoch == *last_epoch {
		return;
	}
	*overlay = LanguageOverlay::from_index(&index);
	*last_epoch = index.epoch;
}

pub(crate) fn install_geneva_presentation<W>(app: &mut App)
where
	W: NamedWorld + TerrainModel,
{
	app.init_resource::<LanguageOverlay>().add_systems(
		Update,
		present_language_overlay.run_if(lod_present_gate_open::<Language<Geneva<W>>>),
	);
}
