//! Revision-owned resident payloads. Visibility never owns cache residency.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use bevy::prelude::*;
use bevy::shape::Aabb3d;
use lod::{LodScene, LodSceneLevel};

use crate::packed::instances::{pack_orchard_batches, pack_orchard_shadows, PackedBatch};
use crate::{ChicoGroveHost, ForestLayer};

pub const LEVELS: [LodSceneLevel; 4] =
	[LodSceneLevel::High, LodSceneLevel::Medium, LodSceneLevel::Low, LodSceneLevel::UltraLow];
pub const PREPARING: u8 = 0;
pub const ACTIVE: u8 = 1;
pub const RETIRED: u8 = 2;
pub const READY_MAIN: u8 = 1;
pub const READY_PREPASS: u8 = 2;
pub const READY_SHADOW: u8 = 4;
pub const READY_BUFFER: u8 = 8;

/// Shared readiness belongs to this revision, so stale renderer acknowledgements
/// cannot activate a replacement or a cancelled host.
pub struct TileSignal {
	pub phase: AtomicU8,
	ready: Box<[AtomicU8]>,
	required: Box<[u8]>,
}

impl TileSignal {
	pub fn new(required: impl IntoIterator<Item = u8>) -> Self {
		let required: Box<[u8]> = required.into_iter().collect();
		let ready = required.iter().map(|_| AtomicU8::new(0)).collect();
		Self { phase: AtomicU8::new(PREPARING), ready, required }
	}

	pub fn acknowledge(&self, batch: usize, flags: u8) {
		if self.phase.load(Ordering::Acquire) != RETIRED {
			if let Some(ready) = self.ready.get(batch) {
				ready.fetch_or(flags, Ordering::Release);
			}
		}
	}

	pub fn is_ready(&self) -> bool {
		self.phase.load(Ordering::Acquire) != RETIRED
			&& self
				.ready
				.iter()
				.zip(&self.required)
				.all(|(ready, required)| ready.load(Ordering::Acquire) & required == *required)
	}

	pub fn activate(&self) -> bool {
		self.is_ready()
			&& self
				.phase
				.compare_exchange(PREPARING, ACTIVE, Ordering::AcqRel, Ordering::Acquire)
				.is_ok()
	}

	pub fn retire(&self) {
		self.phase.store(RETIRED, Ordering::Release);
	}
}

/// All bands are packed once from the same grown source. Only Arc references
/// cross extraction; camera motion never visits individual instances.
pub struct TilePayload {
	pub revision: u64,
	pub proxy_shadows: bool,
	pub bounds: Aabb3d,
	pub center: Vec3,
	pub radius: f32,
	pub lods: [Arc<[PackedBatch]>; 4],
	pub cpu_payload_bytes: u64,
	pub gpu_payload_bytes: u64,
}

impl TilePayload {
	pub fn from_host(host: &ChicoGroveHost, revision: u64, proxy_shadows: bool) -> Option<Self> {
		let orchard = host.tile.as_orchard()?;
		let drop = crate::LayerDropOut::for_stacked(host.layer, false);
		let lods: [Arc<[PackedBatch]>; 4] = LEVELS.map(|level| {
			if drop.omits(level) {
				return Arc::from([]);
			}
			let mut batches = pack_orchard_batches(orchard, level);
			if proxy_shadows {
				batches.extend(pack_orchard_shadows(orchard, level));
			}
			Arc::from(batches)
		});
		if lods
			.iter()
			.flat_map(|batches| batches.iter())
			.flat_map(|batch| batch.instances.iter())
			.any(|instance| {
				!instance.world_from_local.is_finite()
					|| instance.world_from_local.determinant() < 1e-12
			}) {
			return None;
		}
		let cpu_payload_bytes = lods
			.iter()
			.flat_map(|batches| batches.iter())
			.map(PackedBatch::payload_bytes)
			.sum();
		let instances = lods
			.iter()
			.flat_map(|batches| batches.iter())
			.map(|batch| batch.instances.len() as u64)
			.sum::<u64>();
		let bounds = orchard.scene_bounds();
		// Original leaf sway is bounded by 0.108 times the part radius.
		let max_scale = lods
			.iter()
			.flat_map(|batches| batches.iter())
			.flat_map(|batch| batch.instances.iter())
			.map(|instance| {
				instance
					.world_from_local
					.x_axis
					.truncate()
					.length()
					.max(instance.world_from_local.y_axis.truncate().length())
					.max(instance.world_from_local.z_axis.truncate().length())
			})
			.fold(1.0_f32, f32::max);
		let slack = bevy::math::Vec3A::splat(2.0 + max_scale * 0.108);
		Some(Self {
			revision,
			proxy_shadows,
			bounds: Aabb3d { min: bounds.min - slack, max: bounds.max + slack },
			center: orchard.structural_center,
			radius: orchard.footprint_radius,
			lods,
			cpu_payload_bytes,
			gpu_payload_bytes: instances * 176,
		})
	}

	pub fn batch_count(&self) -> usize {
		self.lods.iter().map(|batches| batches.len()).sum()
	}

	pub fn instances(&self) -> usize {
		self.lods
			.iter()
			.flat_map(|batches| batches.iter())
			.map(|batch| batch.instances.len())
			.sum()
	}

	pub fn level_index(level: LodSceneLevel) -> usize {
		match level {
			LodSceneLevel::High => 0,
			LodSceneLevel::Medium => 1,
			LodSceneLevel::Low => 2,
			_ => 3,
		}
	}
}

pub struct ResidentTile {
	pub original: ChicoGroveHost,
	pub payload: Arc<TilePayload>,
	pub signal: Arc<TileSignal>,
	pub proxies: Vec<Entity>,
	pub layer: ForestLayer,
	pub last_kit_epoch: u64,
}

/// Active sources are pinned. A denied source retries only after an admission
/// epoch changes (retirement/budget/source change), never because a camera moves.
#[derive(Resource)]
pub struct PackedGroveCache {
	pub tiles: HashMap<Entity, ResidentTile>,
	pub denied: HashMap<Entity, u64>,
	pub unsupported: HashMap<Entity, u64>,
	pub epoch: u64,
	pub revision: u64,
	pub budget_bytes: u64,
	pub builds: u64,
	pub budget_denials: u64,
}

impl Default for PackedGroveCache {
	fn default() -> Self {
		Self::with_budget(crate::packed::mode::PackMode::budget_bytes_from_env())
	}
}

impl PackedGroveCache {
	pub fn with_budget(budget_bytes: u64) -> Self {
		Self {
			tiles: HashMap::new(),
			denied: HashMap::new(),
			unsupported: HashMap::new(),
			epoch: 0,
			revision: 0,
			budget_bytes,
			builds: 0,
			budget_denials: 0,
		}
	}

	pub fn retained_gpu_bytes(&self) -> u64 {
		self.tiles.values().map(|tile| tile.payload.gpu_payload_bytes).sum()
	}

	pub fn retained_cpu_bytes(&self) -> u64 {
		self.tiles.values().map(|tile| tile.payload.cpu_payload_bytes).sum()
	}

	pub fn admit(&mut self, entity: Entity, bytes: u64) -> bool {
		if self.denied.get(&entity) == Some(&self.epoch) {
			return false;
		}
		if bytes > self.budget_bytes.saturating_sub(self.retained_gpu_bytes()) {
			self.denied.insert(entity, self.epoch);
			self.budget_denials += 1;
			return false;
		}
		self.denied.remove(&entity);
		true
	}

	pub fn remove(&mut self, entity: Entity) -> Option<ResidentTile> {
		let tile = self.tiles.remove(&entity)?;
		tile.signal.retire();
		self.epoch = self.epoch.wrapping_add(1);
		Some(tile)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn readiness_is_per_revision_and_all_required_passes() {
		let old = TileSignal::new([READY_MAIN | READY_BUFFER, READY_SHADOW | READY_BUFFER]);
		old.acknowledge(0, READY_MAIN | READY_BUFFER);
		old.acknowledge(1, READY_BUFFER);
		assert!(!old.activate());
		old.acknowledge(1, READY_SHADOW);
		assert!(old.activate());
		old.retire();
		old.acknowledge(1, u8::MAX);
		assert!(!old.activate());
		let next = TileSignal::new([READY_MAIN | READY_BUFFER]);
		assert!(!next.is_ready());
	}
	#[test]
	fn denied_admission_does_not_retry_on_camera_motion() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let mut cache = PackedGroveCache::with_budget(128);
		assert!(!cache.admit(entity, 256));
		assert!(!cache.admit(entity, 64));
		assert_eq!(cache.budget_denials, 1);
		cache.epoch += 1;
		assert!(cache.admit(entity, 64));
	}
}
