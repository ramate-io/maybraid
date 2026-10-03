//! Side table of tiles and assigned names, keyed by existing cell ids.

use std::collections::HashMap;

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use bevy::prelude::Resource;
use lod::gen::Id;
use maybraid_language_core::lexicalizer::mix;

use crate::bundle::LanguageBundle;
use crate::name::PlaceName;
use crate::tiles::{large_tile_index, LargeTile, LARGE_TILE};

/// World seed for language tiles and names.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageWorldSeed(pub u64);

impl Default for LanguageWorldSeed {
	fn default() -> Self {
		Self(0x6A7B_8A1D_E11E)
	}
}

/// Key for a name owned by the language layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NameKey {
	Region { ix: i32, iz: i32 },
	Forest(Id),
	Grove(Id),
	Urban(Id),
	UrbanLeaf(Id),
	Geographic(u64),
	Place { qx: i32, qz: i32, label: u32 },
}

/// Tiles and names for the keep ring.
#[derive(Resource, Clone, Debug, Default)]
pub struct LanguageIndex {
	large: HashMap<(i32, i32), LargeTile>,
	names: HashMap<NameKey, PlaceName>,
}

impl LanguageIndex {
	pub fn clear(&mut self) {
		self.large.clear();
		self.names.clear();
	}

	pub fn large_tile(&self, ix: i32, iz: i32) -> Option<&LargeTile> {
		self.large.get(&(ix, iz))
	}

	pub fn name(&self, key: NameKey) -> Option<&PlaceName> {
		self.names.get(&key)
	}

	pub fn names(&self) -> impl Iterator<Item = (NameKey, &PlaceName)> {
		self.names.iter().map(|(key, name)| (*key, name))
	}

	pub fn large_tiles(&self) -> impl Iterator<Item = &LargeTile> {
		self.large.values()
	}

	pub fn ensure_tiles(&mut self, world_seed: u64, region: Aabb3d) {
		for (ix, iz) in large_tiles_overlapping(region) {
			self.large.entry((ix, iz)).or_insert_with(|| LargeTile::generate(world_seed, ix, iz));
		}
	}

	pub fn assign_region_name(&mut self, world_seed: u64, ix: i32, iz: i32, english: &[String]) {
		let key = NameKey::Region { ix, iz };
		if self.names.contains_key(&key) {
			return;
		}
		let Some(tile) = self.large.get(&(ix, iz)) else {
			return;
		};
		let Some(bundle) = pick_bundle(&tile.languages, mix(world_seed ^ mix(ix as u64) ^ mix(iz as u64)))
		else {
			return;
		};
		let name = PlaceName::translate(bundle, english, mix(world_seed ^ 0x51A7 ^ mix(ix as u64)));
		if !name.surface.is_empty() {
			self.names.insert(key, name);
		}
	}

	pub fn assign_feature_name(
		&mut self,
		world_seed: u64,
		key: NameKey,
		center: Vec2,
		english: &[String],
	) {
		if self.names.contains_key(&key) {
			return;
		}
		let Some(bundle) = self.language_at(center) else {
			return;
		};
		let pick = mix(world_seed ^ name_key_salt(key));
		let name = PlaceName::translate(bundle, english, pick);
		if !name.surface.is_empty() {
			self.names.insert(key, name);
		}
	}

	pub fn language_at(&self, xz: Vec2) -> Option<&LanguageBundle> {
		let ix = large_tile_index(xz.x);
		let iz = large_tile_index(xz.y);
		let tile = self.large.get(&(ix, iz))?;
		if let Some(small) = tile.small_tile_at(xz.x, xz.y) {
			let id = small.language_ids.first().copied()?;
			return tile.language(id);
		}
		pick_bundle(&tile.languages, mix(ix as u64 ^ (iz as u64).wrapping_mul(13)))
	}

	pub fn assign_keep(
		&mut self,
		world_seed: u64,
		region: Aabb3d,
		features: &[crate::NamedFeature],
		places: &[crate::NamedPlace],
	) {
		self.ensure_tiles(world_seed, region);
		for (ix, iz) in large_tiles_overlapping(region) {
			let tile_bounds = large_tile_aabb(ix, iz);
			let mut english = Vec::new();
			for feature in features {
				if intersects_xz(tile_bounds, feature.bounds) {
					english.extend(feature.english.iter().cloned());
				}
			}
			self.assign_region_name(world_seed, ix, iz, &english);
		}
		for feature in features {
			let center = Vec2::new(
				(feature.bounds.min.x + feature.bounds.max.x) * 0.5,
				(feature.bounds.min.z + feature.bounds.max.z) * 0.5,
			);
			self.assign_feature_name(world_seed, feature.key, center, &feature.english);
		}
		for place in places {
			self.assign_feature_name(world_seed, place.key, place.xz, &place.english);
		}
	}
}

fn large_tile_aabb(ix: i32, iz: i32) -> Aabb3d {
	let origin = crate::large_tile_origin(ix, iz);
	Aabb3d::from_min_max(
		bevy::math::Vec3::new(origin.0, -1.0, origin.1),
		bevy::math::Vec3::new(origin.0 + LARGE_TILE, 1.0, origin.1 + LARGE_TILE),
	)
}

fn intersects_xz(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x < b.max.x && a.max.x > b.min.x && a.min.z < b.max.z && a.max.z > b.min.z
}

pub fn large_tiles_overlapping(region: Aabb3d) -> impl Iterator<Item = (i32, i32)> {
	let min_x = large_tile_index(region.min.x);
	let max_x = large_tile_index((region.max.x - 1e-3).max(region.min.x));
	let min_z = large_tile_index(region.min.z);
	let max_z = large_tile_index((region.max.z - 1e-3).max(region.min.z));
	(min_x..=max_x).flat_map(move |ix| (min_z..=max_z).map(move |iz| (ix, iz)))
}

fn pick_bundle(languages: &[LanguageBundle], seed: u64) -> Option<&LanguageBundle> {
	if languages.is_empty() {
		return None;
	}
	languages.get((mix(seed) as usize) % languages.len())
}

fn name_key_salt(key: NameKey) -> u64 {
	match key {
		NameKey::Region { ix, iz } => mix(ix as u64) ^ mix(iz as u64),
		NameKey::Forest(id) | NameKey::Grove(id) | NameKey::Urban(id) | NameKey::UrbanLeaf(id) => {
			mix(id_bits(id))
		}
		NameKey::Geographic(tag) => mix(tag),
		NameKey::Place { qx, qz, label } => {
			mix(qx as u64) ^ mix(qz as u64) ^ mix(u64::from(label))
		}
	}
}

fn id_bits(id: Id) -> u64 {
	match id {
		Id::Universal => 1,
		Id::Bytes(bytes) => {
			let mut out = 0u64;
			for chunk in bytes.0.chunks(8) {
				let mut buf = [0u8; 8];
				buf[..chunk.len()].copy_from_slice(chunk);
				out ^= u64::from_le_bytes(buf);
			}
			out
		}
		Id::OriginCell(cell) => {
			let bounds = cell.0 .0;
			u64::from(bounds.min.x.to_bits())
				^ u64::from(bounds.min.z.to_bits()).wrapping_shl(1)
				^ u64::from(bounds.max.x.to_bits()).wrapping_shl(2)
		}
	}
}

/// Default keep used when a mode enters and no camera has streamed yet.
pub fn origin_keep() -> Aabb3d {
	Aabb3d::from_min_max(
		bevy::math::Vec3::new(-LARGE_TILE, -1.0, -LARGE_TILE),
		bevy::math::Vec3::new(LARGE_TILE, 1.0, LARGE_TILE),
	)
}
