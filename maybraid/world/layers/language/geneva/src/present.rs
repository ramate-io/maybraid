//! Debug overlay of tile bounds and assigned names. Not a mesh stream.

use bevy::math::{Rect, Vec2};
use bevy::prelude::{Local, ResMut, Resource};

use crate::index::{LanguageIndex, NameKey, OverlayDirty};
use crate::name::AssignedName;

/// Tile bounds and names for a debug overlay.
///
/// `epoch` follows [`LanguageIndex::epoch`], so readers can skip rebuilds
/// while it holds.
#[derive(Resource, Clone, Debug, Default, PartialEq)]
pub struct LanguageOverlay {
	pub epoch: u64,
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
		let large_tiles = overlay_tiles(index);
		let names = index
			.assigned_names()
			.filter_map(|(key, _)| named_from_index(index, key))
			.collect();
		Self { epoch: index.epoch, large_tiles, names }
	}

	pub(crate) fn apply_dirty(&mut self, index: &LanguageIndex, dirty: OverlayDirty) {
		if dirty.rebuild_all {
			*self = Self::from_index(index);
			return;
		}
		if dirty.tiles {
			self.large_tiles = overlay_tiles(index);
		}
		for key in dirty.keys {
			self.names.retain(|name| name.key != key);
			if let Some(row) = named_from_index(index, key) {
				self.names.push(row);
			}
		}
		self.epoch = index.epoch;
	}
}

fn overlay_tiles(index: &LanguageIndex) -> Vec<LargeTileOverlay> {
	index
		.large_tiles()
		.map(|tile| LargeTileOverlay {
			ix: tile.ix,
			iz: tile.iz,
			language_count: tile.languages.len(),
			small_count: tile.small.len(),
		})
		.collect()
}

fn named_from_index(index: &LanguageIndex, key: NameKey) -> Option<NamedOverlay> {
	if !index.is_active(key) {
		return None;
	}
	let assigned: &AssignedName = index.assigned(key)?;
	Some(NamedOverlay {
		key,
		surface: assigned.name.surface.clone(),
		english: assigned.name.english.clone(),
		provisional: assigned.provisional,
		xz: index.anchor(key)?,
		extent: index.extent(key)?,
	})
}

/// Upserts the keys the index marked since the last epoch it presented.
pub(crate) fn present_language_overlay(
	mut index: ResMut<LanguageIndex>,
	mut overlay: ResMut<LanguageOverlay>,
	mut last_epoch: Local<u64>,
) {
	if index.epoch == *last_epoch {
		return;
	}
	let dirty = index.take_overlay_dirty();
	overlay.apply_dirty(&index, dirty);
	*last_epoch = index.epoch;
}
