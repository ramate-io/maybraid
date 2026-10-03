//! Debug overlay of tile bounds and assigned names. Not a mesh stream.

use bevy::prelude::Resource;

use crate::index::{LanguageIndex, NameKey};

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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedOverlay {
	pub key: NameKey,
	pub surface: String,
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
			.names()
			.map(|(key, name)| NamedOverlay { key, surface: name.surface.clone() })
			.collect();
		Self { large_tiles, names }
	}
}

pub(crate) fn install_geneva_presentation(_app: &mut bevy::prelude::App) {}
