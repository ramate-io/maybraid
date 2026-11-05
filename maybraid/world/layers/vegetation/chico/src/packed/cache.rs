//! Resident packed tiles keyed by present id + content revision.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{Id, Version};
use lod::{LodScene, LodSceneLevel};
use vegetation_groves::Orchard;

use crate::host::ChicoGroveHost;
use crate::kind::ForestLayer;
use crate::packed::instances::{pack_orchard_batches, PackedBatch};
use crate::packed::mode::PackMode;

/// Metres added to tile AABBs so wind displacement stays inside the cull hull.
pub const WIND_SLACK_M: f32 = 2.0;

/// Present-cell marker stamped on packed orchard hosts.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PackedGroveCell {
	pub id: Id,
	pub version: Version,
}

/// How a cache probe resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reactivation {
	Fresh,
	Retained,
	Rebuilt,
}

/// One grown orchard tile cached for every LOD band.
#[derive(Clone, Debug)]
pub struct PackedTile {
	pub id: Id,
	pub version: Version,
	pub fingerprint: u64,
	pub bounds: Aabb3d,
	pub center: Vec3,
	pub radius: f32,
	pub lods: HashMap<LodSceneLevel, Vec<PackedBatch>>,
	pub last_seen_frame: u32,
	pub gpu_dirty: bool,
}

impl PackedTile {
	pub fn payload_bytes(&self) -> u64 {
		self.lods.values().flatten().map(PackedBatch::payload_bytes).sum()
	}

	pub fn batches(&self, level: LodSceneLevel) -> &[PackedBatch] {
		self.lods.get(&level).map(Vec::as_slice).unwrap_or(&[])
	}
}

/// Resident visual payloads. Culling does not drop entries.
#[derive(Resource, Debug)]
pub struct PackedGroveCache {
	tiles: HashMap<Id, PackedTile>,
	budget_bytes: u64,
}

impl Default for PackedGroveCache {
	fn default() -> Self {
		Self { tiles: HashMap::new(), budget_bytes: PackMode::budget_bytes_from_env() }
	}
}

impl PackedGroveCache {
	pub fn with_budget_bytes(budget_bytes: u64) -> Self {
		Self { tiles: HashMap::new(), budget_bytes: budget_bytes.max(1) }
	}

	pub fn get(&self, id: Id) -> Option<&PackedTile> {
		self.tiles.get(&id)
	}

	pub fn tiles(&self) -> impl Iterator<Item = &PackedTile> {
		self.tiles.values()
	}

	pub fn len(&self) -> usize {
		self.tiles.len()
	}

	pub fn retained_bytes(&self) -> u64 {
		self.tiles.values().map(PackedTile::payload_bytes).sum()
	}

	/// Insert or reuse a tile. Camera motion that only looks away does not call this.
	pub fn ensure(
		&mut self,
		id: Id,
		version: Version,
		orchard: &Orchard,
		layer: ForestLayer,
		frame: u32,
	) -> Reactivation {
		let fingerprint = ground_fingerprint(version, orchard, layer);
		if let Some(existing) = self.tiles.get_mut(&id) {
			if existing.version == version && existing.fingerprint == fingerprint {
				existing.last_seen_frame = frame;
				return Reactivation::Retained;
			}
			*existing = pack_tile(id, version, fingerprint, orchard, frame);
			existing.gpu_dirty = true;
			return Reactivation::Rebuilt;
		}
		self.tiles.insert(id, pack_tile(id, version, fingerprint, orchard, frame));
		Reactivation::Fresh
	}

	pub fn touch(&mut self, id: Id, frame: u32) {
		if let Some(tile) = self.tiles.get_mut(&id) {
			tile.last_seen_frame = frame;
		}
	}

	pub fn mark_uploaded(&mut self, id: Id) {
		if let Some(tile) = self.tiles.get_mut(&id) {
			tile.gpu_dirty = false;
		}
	}

	pub fn remove(&mut self, id: Id) -> Option<PackedTile> {
		self.tiles.remove(&id)
	}

	/// Drop the oldest untouched tiles until the instance-payload budget fits.
	pub fn evict_to_budget(&mut self) -> usize {
		let mut evicted = 0;
		while self.retained_bytes() > self.budget_bytes && self.tiles.len() > 1 {
			let oldest = self
				.tiles
				.iter()
				.min_by_key(|(_, tile)| tile.last_seen_frame)
				.map(|(id, _)| *id);
			let Some(id) = oldest else {
				break;
			};
			self.tiles.remove(&id);
			evicted += 1;
		}
		evicted
	}
}

/// Content identity: revision, plant count, footprint, and stacking layer.
pub fn ground_fingerprint(version: Version, orchard: &Orchard, layer: ForestLayer) -> u64 {
	let mut hasher = std::collections::hash_map::DefaultHasher::new();
	version.0.hash(&mut hasher);
	orchard.plants.len().hash(&mut hasher);
	for axis in [
		orchard.extent.min().x,
		orchard.extent.min().z,
		orchard.extent.max().x,
		orchard.extent.max().z,
	] {
		axis.to_bits().hash(&mut hasher);
	}
	layer.hash(&mut hasher);
	hasher.finish()
}

fn pack_tile(
	id: Id,
	version: Version,
	fingerprint: u64,
	orchard: &Orchard,
	frame: u32,
) -> PackedTile {
	let mut bounds = expand_wind(orchard.scene_bounds());
	if let Some(cell) = id.origin_cell_bounds() {
		bounds = union_aabb(bounds, expand_wind(cell));
	}
	let mut lods = HashMap::new();
	for level in [LodSceneLevel::High, LodSceneLevel::Medium, LodSceneLevel::Low, LodSceneLevel::UltraLow]
	{
		lods.insert(level, pack_orchard_batches(orchard, level));
	}
	PackedTile {
		id,
		version,
		fingerprint,
		bounds,
		center: orchard.structural_center,
		radius: orchard.footprint_radius,
		lods,
		last_seen_frame: frame,
		gpu_dirty: true,
	}
}

fn expand_wind(bounds: Aabb3d) -> Aabb3d {
	let slack = bevy::math::Vec3A::splat(WIND_SLACK_M);
	Aabb3d { min: bounds.min - slack, max: bounds.max + slack }
}

fn union_aabb(a: Aabb3d, b: Aabb3d) -> Aabb3d {
	Aabb3d { min: a.min.min(b.min), max: a.max.max(b.max) }
}

/// Attach stick capsules on the grove host so packed tiles keep collision in ECS.
pub(crate) fn attach_packed_orchard_physics(
	insert: On<Insert, PackedGroveCell>,
	mut commands: Commands,
) {
	if let Ok(mut entity) = commands.get_entity(insert.entity) {
		entity.insert(crate::stick_physics::StickPhysicsProducer::new(|world, entity, level| {
			let Some(host) = world.get::<ChicoGroveHost>(entity) else {
				return Vec::new();
			};
			let Some(orchard) = host.tile.as_orchard() else {
				return Vec::new();
			};
			orchard
				.plants
				.iter()
				.flat_map(|plant| crate::stick_physics::collider_poses_for(&plant.placed(), level))
				.collect()
		}));
	}
}
