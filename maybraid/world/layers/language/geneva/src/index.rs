//! Side table of tiles and assigned names, keyed by existing cell ids.

use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::math::{Rect, Vec2};
use bevy::prelude::Resource;
use durham::GeographicFeatureId;
use lod::gen::{Id, Version};
use maybraid_language_core::lexicalizer::mix;

use crate::bundle::LanguageBundle;
use crate::english::named_region_english;
use crate::name::{terms_fingerprint, AssignedName, PlaceName};
use crate::sources::{
	feature_center, place_extent, xz_extent, FeatureSnapshot, NamedFeature, NamedPlace,
	PlaceSnapshot, PoseUpdate, SourceRevisions, NAME_WINDOW_QUANT_M,
};
use crate::tiles::{large_tile_index, large_tile_origin, LargeTile, LARGE_TILE};

/// Explicit keep-ring dependency. Stored as a tuple so simultaneous source
/// revisions cannot cancel the way a single XOR fingerprint can.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LanguageSourceDeps {
	pub revisions: SourceRevisions,
	pub seed: u64,
	pub tile_min: (i32, i32),
	pub tile_max: (i32, i32),
	pub naming_origin: (i32, i32),
}

impl LanguageSourceDeps {
	pub fn from_windows(
		revisions: SourceRevisions,
		seed: u64,
		tile_region: Aabb3d,
		naming_xz: Vec2,
	) -> Self {
		Self {
			revisions,
			seed,
			tile_min: (large_tile_index(tile_region.min.x), large_tile_index(tile_region.min.z)),
			tile_max: (
				large_tile_index((tile_region.max.x - 1e-3).max(tile_region.min.x)),
				large_tile_index((tile_region.max.z - 1e-3).max(tile_region.min.z)),
			),
			naming_origin: (
				(naming_xz.x / NAME_WINDOW_QUANT_M).round() as i32,
				(naming_xz.y / NAME_WINDOW_QUANT_M).round() as i32,
			),
		}
	}

	pub fn tiles_match(self, other: Self) -> bool {
		self.seed == other.seed
			&& self.tile_min == other.tile_min
			&& self.tile_max == other.tile_max
			&& self.revisions.tiles == other.revisions.tiles
	}

	pub fn naming_window_match(self, other: Self) -> bool {
		self.seed == other.seed && self.naming_origin == other.naming_origin
	}
}

/// Configured world seed for language tiles and names: Geneva's session root.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageWorldSeed(pub u64);

lod::seeded_root!(LanguageWorldSeed);

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
	Region {
		ix: i32,
		iz: i32,
	},
	Forest(Id),
	Grove(Id),
	Urban(Id),
	UrbanLeaf(Id),
	Geographic(GeographicFeatureId),
	Place {
		host: Id,
		local: u32,
	},
	/// Rounded XZ + label when Richmond has not attached a host identity.
	/// Assignments under this key stay provisional.
	ProvisionalPlace {
		qx: i32,
		qz: i32,
		label: u32,
	},
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct QueuedStamp {
	revision: u64,
	fingerprint: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SourceClass {
	Vegetation,
	Geography,
	Urban,
	Place,
	Region,
}

/// Keys presentation should upsert or drop without cloning the whole index.
#[derive(Clone, Debug, Default)]
pub(crate) struct OverlayDirty {
	pub rebuild_all: bool,
	pub tiles: bool,
	pub keys: HashSet<NameKey>,
}

/// Tiles and names for the keep ring.
///
/// Belongs to one session: the version of the [`LanguageWorldSeed`] root it
/// was filled under. A new root version clears it.
#[derive(Resource, Clone, Debug, Default)]
pub struct LanguageIndex {
	session: Option<Version>,
	large: HashMap<(i32, i32), Arc<LargeTile>>,
	names: HashMap<NameKey, AssignedName>,
	/// World XZ for each assigned name. Kept off [`AssignedName`] so that type stays `Eq`.
	anchors: HashMap<NameKey, Vec2>,
	/// XZ extent of the named cell or large tile. Used by the map to label intersections.
	extents: HashMap<NameKey, Rect>,
	host_languages: HashMap<Id, u64>,
	pending: HashMap<NameKey, PendingAssign>,
	queued: HashMap<NameKey, QueuedStamp>,
	source_deps: Option<LanguageSourceDeps>,
	/// Names currently overlapping the keep ring. Durable records may outlive this.
	active: HashSet<NameKey>,
	overlay_dirty: OverlayDirty,
	/// Increments when assigned names or tiles change.
	pub epoch: u64,
}

impl LanguageIndex {
	pub fn clear(&mut self) {
		self.session = None;
		self.large.clear();
		self.names.clear();
		self.anchors.clear();
		self.extents.clear();
		self.host_languages.clear();
		self.pending.clear();
		self.queued.clear();
		self.source_deps = None;
		self.active.clear();
		self.overlay_dirty = OverlayDirty { rebuild_all: true, tiles: true, keys: HashSet::new() };
		self.epoch = self.epoch.wrapping_add(1);
	}

	/// Version of the root this index was filled under; `None` while idle.
	pub fn session(&self) -> Option<Version> {
		self.session
	}

	/// Starts over under root `version` unless already filled under it.
	pub fn begin_session(&mut self, version: Version) {
		if self.session != Some(version) {
			self.clear();
			self.session = Some(version);
		}
	}

	/// Drops everything once, leaving the index idle.
	pub fn end_session(&mut self) {
		if self.session.is_some() {
			self.clear();
		}
	}

	/// Last queued or assigned stamp matches, so keep must not re-translate.
	pub(crate) fn is_current(&self, key: NameKey, revision: u64, fingerprint: u64) -> bool {
		if self
			.queued
			.get(&key)
			.is_some_and(|stamp| stamp.revision == revision && stamp.fingerprint == fingerprint)
		{
			return true;
		}
		self.names.get(&key).is_some_and(|assigned| {
			assigned.source_revision == revision && assigned.fingerprint == fingerprint
		})
	}

	/// Storage version still matches. Used to skip English rebuilds on snapshot.
	pub(crate) fn is_current_revision(&self, key: NameKey, revision: u64) -> bool {
		self.queued.get(&key).is_some_and(|stamp| stamp.revision == revision)
			|| self
				.names
				.get(&key)
				.is_some_and(|assigned| assigned.source_revision == revision)
	}

	pub(crate) fn take_overlay_dirty(&mut self) -> OverlayDirty {
		std::mem::take(&mut self.overlay_dirty)
	}

	fn mark_overlay_key(&mut self, key: NameKey) {
		self.overlay_dirty.keys.insert(key);
	}

	fn bump_epoch(&mut self) {
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
		self.large.get(&(ix, iz)).map(|tile| &**tile)
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
		self.large.values().map(|tile| &**tile)
	}

	pub fn source_deps(&self) -> Option<LanguageSourceDeps> {
		self.source_deps
	}

	pub fn note_source_deps(&mut self, deps: LanguageSourceDeps) {
		self.source_deps = Some(deps);
	}

	pub fn is_active(&self, key: NameKey) -> bool {
		self.active.contains(&key)
	}

	pub fn pose_matches(&self, key: NameKey, center: Vec2, extent: Rect) -> bool {
		self.anchors.get(&key) == Some(&center) && self.extents.get(&key) == Some(&extent)
	}

	pub fn update_anchor(&mut self, key: NameKey, center: Vec2, extent: Rect) {
		if self.pose_matches(key, center, extent) {
			return;
		}
		if !self.names.contains_key(&key) {
			return;
		}
		self.anchors.insert(key, center);
		self.extents.insert(key, extent);
		self.mark_overlay_key(key);
		self.bump_epoch();
	}

	pub fn apply_pose_updates<I>(&mut self, updates: I)
	where
		I: IntoIterator<Item = PoseUpdate>,
	{
		for update in updates {
			self.update_anchor(update.key, update.center, update.extent);
		}
	}

	/// Admits published `tiles` and names the region of every admitted tile
	/// in `window`. Regions outside `window` leave the overlay; their tiles stay.
	pub fn sync_tiles(
		&mut self,
		world_seed: u64,
		window: &[(i32, i32)],
		tiles: impl IntoIterator<Item = Arc<LargeTile>>,
	) {
		let mut added = false;
		for tile in tiles {
			if let Entry::Vacant(slot) = self.large.entry((tile.ix, tile.iz)) {
				slot.insert(tile);
				added = true;
			}
		}
		if added {
			self.overlay_dirty.tiles = true;
			self.bump_epoch();
		}
		for &(ix, iz) in window {
			if self.large.contains_key(&(ix, iz)) {
				let english = named_region_english(world_seed, ix, iz);
				let fingerprint = terms_fingerprint(&english);
				self.enqueue_region(ix, iz, english, fingerprint);
			}
		}
		let regions = window.iter().map(|&(ix, iz)| NameKey::Region { ix, iz }).collect();
		self.replace_active_class(SourceClass::Region, regions);
	}

	/// Merge one source's snapshot without rebuilding the other domains.
	pub(crate) fn queue_feature_snapshot(&mut self, snapshot: FeatureSnapshot, class: SourceClass) {
		for feature in &snapshot.work {
			self.enqueue_feature(feature);
		}
		self.apply_pose_updates(snapshot.moved);
		self.replace_active_class(class, snapshot.active);
		let active = self.active.clone();
		self.retire_superseded(&active);
		self.cancel_pending_outside_active();
	}

	pub(crate) fn queue_place_snapshot(&mut self, snapshot: PlaceSnapshot) {
		for place in &snapshot.work {
			self.enqueue_place(place);
		}
		self.apply_pose_updates(snapshot.moved);
		self.replace_active_class(SourceClass::Place, snapshot.active);
		let active = self.active.clone();
		self.retire_superseded(&active);
		self.cancel_pending_outside_active();
	}

	pub fn cancel_pending_outside_active(&mut self) {
		let drop: Vec<_> = self
			.pending
			.keys()
			.copied()
			.filter(|key| source_class(*key) != SourceClass::Region && !self.active.contains(key))
			.collect();
		for key in drop {
			self.pending.remove(&key);
			self.queued.remove(&key);
		}
	}

	fn enqueue_region(&mut self, ix: i32, iz: i32, english: Vec<String>, fingerprint: u64) {
		let key = NameKey::Region { ix, iz };
		if self.is_current(key, 0, fingerprint) {
			return;
		}
		self.queued.insert(key, QueuedStamp { revision: 0, fingerprint });
		self.pending.insert(key, PendingAssign::Region { ix, iz, english, fingerprint });
	}

	fn enqueue_feature(&mut self, feature: &NamedFeature) {
		if self.is_current(feature.key, feature.revision, feature.fingerprint) {
			self.update_anchor(
				feature.key,
				feature_center(feature.bounds),
				xz_extent(feature.bounds),
			);
			return;
		}
		self.queued.insert(
			feature.key,
			QueuedStamp { revision: feature.revision, fingerprint: feature.fingerprint },
		);
		self.pending.insert(
			feature.key,
			PendingAssign::Feature {
				key: feature.key,
				center: feature_center(feature.bounds),
				extent: xz_extent(feature.bounds),
				english: feature.english.clone(),
				revision: feature.revision,
				fingerprint: feature.fingerprint,
				provisional: feature.provisional,
				inherit_host: None,
			},
		);
	}

	fn enqueue_place(&mut self, place: &NamedPlace) {
		let inherit_host = place.host.filter(|_| place.inherit_host_language);
		if inherit_host.is_none() && self.is_current(place.key, place.revision, place.fingerprint) {
			self.update_anchor(place.key, place.xz, place_extent(place.xz));
			return;
		}
		self.queued.insert(
			place.key,
			QueuedStamp { revision: place.revision, fingerprint: place.fingerprint },
		);
		self.pending.insert(
			place.key,
			PendingAssign::Feature {
				key: place.key,
				center: place.xz,
				extent: place_extent(place.xz),
				english: place.english.clone(),
				revision: place.revision,
				fingerprint: place.fingerprint,
				provisional: place.provisional,
				inherit_host,
			},
		);
	}

	fn replace_active_class(&mut self, class: SourceClass, next: HashSet<NameKey>) {
		let stale: Vec<_> = self
			.active
			.iter()
			.copied()
			.filter(|key| source_class(*key) == class && !next.contains(key))
			.collect();
		let mut changed = false;
		for key in stale {
			self.active.remove(&key);
			self.queued.remove(&key);
			self.mark_overlay_key(key);
			changed = true;
		}
		for key in next {
			if self.active.insert(key) {
				self.mark_overlay_key(key);
				changed = true;
			}
		}
		if changed {
			self.bump_epoch();
		}
	}

	pub fn assign_budgeted(&mut self, world_seed: u64, budget: usize) {
		if budget == 0 || self.pending.is_empty() {
			return;
		}
		let mut keys: Vec<NameKey> = self.pending.keys().copied().collect();
		keys.sort_by_key(|key| {
			let work = self.pending.get(key);
			(work.map(pending_host_first).unwrap_or(1), name_key_salt(*key))
		});
		for key in keys.into_iter().take(budget) {
			let Some(work) = self.pending.remove(&key) else {
				continue;
			};
			if matches!(self.apply_pending(world_seed, work), AssignResult::Failed) {
				self.queued.remove(&key);
			}
		}
	}

	fn retire_superseded(&mut self, active: &HashSet<NameKey>) {
		let stale: Vec<_> = self
			.names
			.keys()
			.copied()
			.filter(|key| matches!(key, NameKey::ProvisionalPlace { .. }) && !active.contains(key))
			.collect();
		if stale.is_empty() {
			return;
		}
		for key in stale {
			self.names.remove(&key);
			self.anchors.remove(&key);
			self.extents.remove(&key);
			self.queued.remove(&key);
			self.mark_overlay_key(key);
		}
		self.bump_epoch();
	}

	fn apply_pending(&mut self, world_seed: u64, work: PendingAssign) -> AssignResult {
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

	/// Regional names come from seed and tile coordinates, not streamed features.
	pub fn assign_region_name(
		&mut self,
		world_seed: u64,
		ix: i32,
		iz: i32,
		english: &[String],
		fingerprint: u64,
	) -> AssignResult {
		let key = NameKey::Region { ix, iz };
		if let Some(existing) = self.names.get(&key) {
			if existing.fingerprint == fingerprint {
				return AssignResult::Unchanged;
			}
		}
		let Some(tile) = self.large.get(&(ix, iz)) else {
			return AssignResult::Failed;
		};
		let Some(bundle) =
			pick_bundle(&tile.languages, mix(world_seed ^ mix(ix as u64) ^ mix(iz as u64)))
		else {
			return AssignResult::Failed;
		};
		let name = PlaceName::translate(bundle, english, mix(world_seed ^ 0x51A7 ^ mix(ix as u64)));
		if name.surface.is_empty() {
			return AssignResult::Failed;
		}
		self.names.insert(
			key,
			AssignedName {
				name,
				source_revision: 0,
				fingerprint,
				provisional: false,
				inherited_language: None,
			},
		);
		self.anchors.insert(key, region_anchor(ix, iz));
		self.extents.insert(key, region_extent(ix, iz));
		self.mark_overlay_key(key);
		self.bump_epoch();
		AssignResult::Changed
	}

	pub fn assign_feature_name(&mut self, work: FeatureAssign<'_>) -> AssignResult {
		let inherited = work.inherit_host.and_then(|host| self.host_languages.get(&host).copied());
		if let Some(existing) = self.names.get(&work.key) {
			if existing.source_revision == work.revision
				&& existing.fingerprint == work.fingerprint
				&& existing.inherited_language == inherited
			{
				self.update_anchor(work.key, work.center, work.extent);
				return AssignResult::Unchanged;
			}
		}
		let feature_key = name_key_salt(work.key);
		let bundle = inherited
			.and_then(|seed| self.bundle_with_seed(work.center, seed))
			.or_else(|| self.language_at_key(work.center, feature_key));
		let Some(bundle) = bundle else {
			return AssignResult::Failed;
		};
		let name = PlaceName::translate_all(bundle, work.english);
		if name.surface.is_empty() {
			return AssignResult::Failed;
		}
		if host_language_authority(&work) {
			if let NameKey::Place { host, .. } = work.key {
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
				inherited_language: inherited,
			},
		);
		self.anchors.insert(work.key, work.center);
		self.extents.insert(work.key, work.extent);
		self.mark_overlay_key(work.key);
		self.bump_epoch();
		AssignResult::Changed
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

/// Frame-free fills for tests: tiles generate in place instead of on the worker.
#[cfg(test)]
impl LanguageIndex {
	pub fn ensure_tiles(&mut self, world_seed: u64, region: Aabb3d) {
		let window: Vec<_> = large_tiles_overlapping(region).collect();
		let missing: Vec<_> = window
			.iter()
			.filter(|tile| !self.large.contains_key(tile))
			.map(|&(ix, iz)| Arc::new(LargeTile::generate(world_seed, ix, iz)))
			.collect();
		self.sync_tiles(world_seed, &window, missing);
	}

	pub fn queue_keep(
		&mut self,
		world_seed: u64,
		region: Aabb3d,
		features: &[NamedFeature],
		places: &[NamedPlace],
	) {
		self.ensure_tiles(world_seed, region);
		let mut next_features = HashSet::new();
		let mut next_places = HashSet::new();
		for feature in features {
			next_features.insert(feature.key);
			self.enqueue_feature(feature);
		}
		for place in places {
			next_places.insert(place.key);
			self.enqueue_place(place);
		}
		for class in [SourceClass::Vegetation, SourceClass::Geography, SourceClass::Urban] {
			let keys = next_features.iter().copied().filter(|key| source_class(*key) == class);
			self.replace_active_class(class, keys.collect());
		}
		self.replace_active_class(SourceClass::Place, next_places);
		let active = self.active.clone();
		self.retire_superseded(&active);
		self.cancel_pending_outside_active();
	}

	pub fn assign_keep(
		&mut self,
		world_seed: u64,
		region: Aabb3d,
		features: &[NamedFeature],
		places: &[NamedPlace],
	) {
		self.queue_keep(world_seed, region, features, places);
		self.assign_budgeted(world_seed, usize::MAX);
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssignResult {
	Changed,
	Unchanged,
	Failed,
}

pub(crate) fn source_class(key: NameKey) -> SourceClass {
	match key {
		NameKey::Forest(_) | NameKey::Grove(_) => SourceClass::Vegetation,
		NameKey::Geographic(_) => SourceClass::Geography,
		NameKey::Urban(_) | NameKey::UrbanLeaf(_) => SourceClass::Urban,
		NameKey::Place { .. } | NameKey::ProvisionalPlace { .. } => SourceClass::Place,
		NameKey::Region { .. } => SourceClass::Region,
	}
}

fn pending_host_first(work: &PendingAssign) -> u8 {
	match work {
		PendingAssign::Feature {
			key: NameKey::Place { .. },
			provisional: false,
			inherit_host: None,
			..
		} => 0,
		PendingAssign::Feature { inherit_host: Some(_), .. } => 2,
		_ => 1,
	}
}

fn host_language_authority(work: &FeatureAssign<'_>) -> bool {
	matches!(work.key, NameKey::Place { .. }) && !work.provisional && work.inherit_host.is_none()
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
