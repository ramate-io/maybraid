//! Side table of tiles and assigned names, keyed by existing cell ids.

use std::collections::{HashMap, VecDeque};

use bevy::math::bounding::Aabb3d;
use bevy::math::{Rect, Vec2};
use bevy::prelude::Resource;
use durham::GeographicFeatureId;
use lod::gen::Id;
use maybraid_language_core::lexicalizer::mix;

use crate::bundle::LanguageBundle;
use crate::name::{terms_fingerprint, AssignedName, PlaceName};
use crate::tiles::{large_tile_index, large_tile_origin, LargeTile, LARGE_TILE};

/// Configured world seed for language tiles and names.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageWorldSeed(pub u64);

impl Default for LanguageWorldSeed {
	fn default() -> Self {
		Self(LanguageConfig::DEFAULT_SEED)
	}
}

/// World/config contract for Geneva generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageConfig {
	pub seed: u64,
}

impl LanguageConfig {
	pub const DEFAULT_SEED: u64 = 0x6A7B_8A1D_E11E;

	pub fn world_defaults() -> Self {
		Self { seed: Self::DEFAULT_SEED }
	}
}

impl Default for LanguageConfig {
	fn default() -> Self {
		Self::world_defaults()
	}
}

/// Key for a name owned by the language layer.
///
/// Domain is part of the key so the same numeric payload in another domain
/// cannot collide. ECS entity ids and rounded coordinates are not durable keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NameKey {
	Region { ix: i32, iz: i32 },
	Forest(Id),
	Grove(Id),
	Urban(Id),
	UrbanLeaf(Id),
	Geographic(GeographicFeatureId),
	Place { host: Id, local: u32 },
	/// Rounded XZ + label when Richmond has not attached a host identity.
	/// Assignments under this key stay provisional.
	ProvisionalPlace { qx: i32, qz: i32, label: u32 },
}

/// Inputs for one feature or place assignment.
pub struct FeatureAssign<'a> {
	pub world_seed: u64,
	pub key: NameKey,
	pub center: Vec2,
	pub extent: Rect,
	pub english: &'a [String],
	pub revision: u64,
	pub fingerprint: u64,
	pub provisional: bool,
	pub inherit_host: Option<Id>,
}

#[derive(Clone, Debug)]
enum PendingAssign {
	Region {
		ix: i32,
		iz: i32,
		english: Vec<String>,
		fingerprint: u64,
	},
	Feature {
		key: NameKey,
		center: Vec2,
		extent: Rect,
		english: Vec<String>,
		revision: u64,
		fingerprint: u64,
		provisional: bool,
		inherit_host: Option<Id>,
	},
}

/// Tiles and names for the keep ring.
#[derive(Resource, Clone, Debug, Default)]
pub struct LanguageIndex {
	large: HashMap<(i32, i32), LargeTile>,
	names: HashMap<NameKey, AssignedName>,
	/// World XZ for each assigned name. Kept off [`AssignedName`] so that type stays `Eq`.
	anchors: HashMap<NameKey, Vec2>,
	/// XZ extent of the named cell or large tile. Used by the map to label intersections.
	extents: HashMap<NameKey, Rect>,
	host_languages: HashMap<Id, u64>,
	pending: VecDeque<PendingAssign>,
	source_fingerprint: u64,
	/// Increments when assigned names or tiles change.
	pub epoch: u64,
}

impl LanguageIndex {
	pub fn clear(&mut self) {
		self.large.clear();
		self.names.clear();
		self.anchors.clear();
		self.extents.clear();
		self.host_languages.clear();
		self.pending.clear();
		self.source_fingerprint = 0;
		self.epoch = self.epoch.wrapping_add(1);
	}

	/// World XZ stored when the name was assigned.
	pub fn anchor(&self, key: NameKey) -> Option<Vec2> {
		self.anchors.get(&key).copied()
	}

	/// XZ extent stored when the name was assigned, or the large-tile rect for a region.
	pub fn extent(&self, key: NameKey) -> Option<Rect> {
		self.extents.get(&key).copied().or_else(|| match key {
			NameKey::Region { ix, iz } => Some(region_extent(ix, iz)),
			_ => None,
		})
	}

	pub fn large_tile(&self, ix: i32, iz: i32) -> Option<&LargeTile> {
		self.large.get(&(ix, iz))
	}

	pub fn name(&self, key: NameKey) -> Option<&PlaceName> {
		self.names.get(&key).map(|assigned| &assigned.name)
	}

	pub fn assigned(&self, key: NameKey) -> Option<&AssignedName> {
		self.names.get(&key)
	}

	pub fn names(&self) -> impl Iterator<Item = (NameKey, &PlaceName)> {
		self.names.iter().map(|(key, assigned)| (*key, &assigned.name))
	}

	pub fn assigned_names(&self) -> impl Iterator<Item = (NameKey, &AssignedName)> {
		self.names.iter().map(|(key, assigned)| (*key, assigned))
	}

	pub fn large_tiles(&self) -> impl Iterator<Item = &LargeTile> {
		self.large.values()
	}

	pub fn source_fingerprint(&self) -> u64 {
		self.source_fingerprint
	}

	pub fn note_source_fingerprint(&mut self, fingerprint: u64) {
		self.source_fingerprint = fingerprint;
	}

	pub fn ensure_tiles(&mut self, world_seed: u64, region: Aabb3d) {
		let mut added = false;
		for (ix, iz) in large_tiles_overlapping(region) {
			self.large.entry((ix, iz)).or_insert_with(|| {
				added = true;
				LargeTile::generate(world_seed, ix, iz)
			});
		}
		if added {
			self.epoch = self.epoch.wrapping_add(1);
		}
	}

	pub fn queue_keep(
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
			let fingerprint = terms_fingerprint(&english);
			self.pending.push_back(PendingAssign::Region { ix, iz, english, fingerprint });
		}
		for feature in features {
			let center = Vec2::new(
				(feature.bounds.min.x + feature.bounds.max.x) * 0.5,
				(feature.bounds.min.z + feature.bounds.max.z) * 0.5,
			);
			self.pending.push_back(PendingAssign::Feature {
				key: feature.key,
				center,
				extent: xz_extent(feature.bounds),
				english: feature.english.clone(),
				revision: feature.revision,
				fingerprint: feature.fingerprint,
				provisional: feature.provisional,
				inherit_host: None,
			});
		}
		for place in places {
			self.pending.push_back(PendingAssign::Feature {
				key: place.key,
				center: place.xz,
				extent: Rect::from_center_size(place.xz, Vec2::splat(12.0)),
				english: place.english.clone(),
				revision: place.revision,
				fingerprint: place.fingerprint,
				provisional: place.provisional,
				inherit_host: place.host.filter(|_| place.inherit_host_language),
			});
		}
	}

	pub fn assign_keep(
		&mut self,
		world_seed: u64,
		region: Aabb3d,
		features: &[crate::NamedFeature],
		places: &[crate::NamedPlace],
	) {
		self.queue_keep(world_seed, region, features, places);
		self.assign_budgeted(world_seed, usize::MAX);
	}

	pub fn assign_budgeted(&mut self, world_seed: u64, budget: usize) {
		let mut remaining = budget;
		while remaining > 0 {
			let Some(work) = self.pending.pop_front() else {
				break;
			};
			if self.apply_pending(world_seed, work) {
				remaining -= 1;
			}
		}
	}

	fn apply_pending(&mut self, world_seed: u64, work: PendingAssign) -> bool {
		match work {
			PendingAssign::Region { ix, iz, english, fingerprint } => {
				self.assign_region_name(world_seed, ix, iz, &english, fingerprint)
			}
			PendingAssign::Feature {
				key,
				center,
				extent,
				english,
				revision,
				fingerprint,
				provisional,
				inherit_host,
			} => self.assign_feature_name(FeatureAssign {
				world_seed,
				key,
				center,
				extent,
				english: &english,
				revision,
				fingerprint,
				provisional,
				inherit_host,
			}),
		}
	}

	/// Regional names stay provisional. A later, different term set refreshes them.
	/// The first nonempty streamed batch does not freeze a permanent name.
	pub fn assign_region_name(
		&mut self,
		world_seed: u64,
		ix: i32,
		iz: i32,
		english: &[String],
		fingerprint: u64,
	) -> bool {
		let key = NameKey::Region { ix, iz };
		if let Some(existing) = self.names.get(&key) {
			if existing.fingerprint == fingerprint {
				return false;
			}
		}
		let Some(tile) = self.large.get(&(ix, iz)) else {
			return false;
		};
		let Some(bundle) =
			pick_bundle(&tile.languages, mix(world_seed ^ mix(ix as u64) ^ mix(iz as u64)))
		else {
			return false;
		};
		let name = PlaceName::translate(bundle, english, mix(world_seed ^ 0x51A7 ^ mix(ix as u64)));
		if name.surface.is_empty() {
			return false;
		}
		self.names.insert(
			key,
			AssignedName {
				name,
				source_revision: 0,
				fingerprint,
				provisional: true,
			},
		);
		self.anchors.insert(key, region_anchor(ix, iz));
		self.extents.insert(key, region_extent(ix, iz));
		self.epoch = self.epoch.wrapping_add(1);
		true
	}

	pub fn assign_feature_name(&mut self, work: FeatureAssign<'_>) -> bool {
		if let Some(existing) = self.names.get(&work.key) {
			if existing.source_revision == work.revision && existing.fingerprint == work.fingerprint
			{
				return false;
			}
		}
		let feature_key = name_key_salt(work.key);
		let bundle = work
			.inherit_host
			.and_then(|host| self.host_languages.get(&host).copied())
			.and_then(|seed| self.bundle_with_seed(work.center, seed))
			.or_else(|| self.language_at_key(work.center, feature_key));
		let Some(bundle) = bundle else {
			return false;
		};
		let pick = mix(work.world_seed ^ feature_key);
		let name = PlaceName::translate(bundle, work.english, pick);
		if name.surface.is_empty() {
			return false;
		}
		if let NameKey::Place { host, .. } = work.key {
			if !work.provisional {
				self.host_languages.insert(host, name.language_seed);
			}
		}
		self.names.insert(
			work.key,
			AssignedName {
				name,
				source_revision: work.revision,
				fingerprint: work.fingerprint,
				provisional: work.provisional,
			},
		);
		self.anchors.insert(work.key, work.center);
		self.extents.insert(work.key, work.extent);
		self.epoch = self.epoch.wrapping_add(1);
		true
	}

	pub fn language_at(&self, xz: Vec2) -> Option<&LanguageBundle> {
		self.language_at_key(xz, mix(xz.x.to_bits() as u64) ^ mix(xz.y.to_bits() as u64))
	}

	pub fn language_at_key(&self, xz: Vec2, feature_key: u64) -> Option<&LanguageBundle> {
		let ix = large_tile_index(xz.x);
		let iz = large_tile_index(xz.y);
		let tile = self.large.get(&(ix, iz))?;
		if let Some(small) = tile.small_tile_at(xz.x, xz.y) {
			return pick_from_subset(&small.language_ids, tile, feature_key);
		}
		pick_bundle(&tile.languages, mix(ix as u64 ^ (iz as u64).wrapping_mul(13) ^ feature_key))
	}

	fn bundle_with_seed(&self, xz: Vec2, seed: u64) -> Option<&LanguageBundle> {
		let ix = large_tile_index(xz.x);
		let iz = large_tile_index(xz.y);
		let tile = self.large.get(&(ix, iz))?;
		tile.languages.iter().find(|bundle| bundle.seed == seed)
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

fn pick_from_subset<'a>(
	ids: &[u8],
	tile: &'a LargeTile,
	feature_key: u64,
) -> Option<&'a LanguageBundle> {
	if ids.is_empty() {
		return None;
	}
	let id = ids[(mix(feature_key) as usize) % ids.len()];
	tile.language(id)
}

pub fn name_key_salt(key: NameKey) -> u64 {
	let domain = match key {
		NameKey::Region { .. } => 0x01,
		NameKey::Forest(_) => 0x02,
		NameKey::Grove(_) => 0x03,
		NameKey::Urban(_) => 0x04,
		NameKey::UrbanLeaf(_) => 0x05,
		NameKey::Geographic(_) => 0x06,
		NameKey::Place { .. } => 0x07,
		NameKey::ProvisionalPlace { .. } => 0x08,
	};
	match key {
		NameKey::Region { ix, iz } => mix(domain) ^ mix(ix as u64) ^ mix(iz as u64),
		NameKey::Forest(id) | NameKey::Grove(id) | NameKey::Urban(id) | NameKey::UrbanLeaf(id) => {
			mix(domain) ^ mix(id_bits(id))
		}
		NameKey::Geographic(id) => {
			mix(domain)
				^ mix(id.family as u8 as u64)
				^ mix(id.band as u8 as u64)
				^ mix(id_bits(id.source))
		}
		NameKey::Place { host, local } => mix(domain) ^ mix(id_bits(host)) ^ mix(u64::from(local)),
		NameKey::ProvisionalPlace { qx, qz, label } => {
			mix(domain) ^ mix(qx as u64) ^ mix(qz as u64) ^ mix(u64::from(label))
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

fn region_anchor(ix: i32, iz: i32) -> Vec2 {
	region_extent(ix, iz).center()
}

fn region_extent(ix: i32, iz: i32) -> Rect {
	let (ox, oz) = large_tile_origin(ix, iz);
	Rect::from_corners(Vec2::new(ox, oz), Vec2::new(ox + LARGE_TILE, oz + LARGE_TILE))
}

fn xz_extent(bounds: Aabb3d) -> Rect {
	Rect::from_corners(Vec2::new(bounds.min.x, bounds.min.z), Vec2::new(bounds.max.x, bounds.max.z))
}

/// Default keep used when a mode enters and no camera has streamed yet.
pub fn origin_keep() -> Aabb3d {
	Aabb3d::from_min_max(
		bevy::math::Vec3::new(-LARGE_TILE, -1.0, -LARGE_TILE),
		bevy::math::Vec3::new(LARGE_TILE, 1.0, LARGE_TILE),
	)
}
