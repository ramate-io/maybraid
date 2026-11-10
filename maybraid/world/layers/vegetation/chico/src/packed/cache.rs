//! Revision-owned resident payloads. Visibility never owns cache residency.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

use bevy::prelude::*;
use bevy::shape::Aabb3d;
use lod::LodSceneLevel;

use crate::ChicoGroveHost;
use crate::packed::instances::{PackedBatch, pack_grove_band};

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

/// Optional diagnostic snapshot: unique coarse batches, not issued draw calls.
#[cfg(feature = "packed-diagnostics")]
#[derive(Default)]
pub struct PackedReadinessSnapshot {
	pub proxy_tiles: usize,
	pub fully_ready_tiles: usize,
	pub batches: u64,
	/// Main / prepass / shadow / buffer bits respectively.
	pub required: [u64; 4],
	pub acknowledged: [u64; 4],
	pub missing_masks: [u64; 16],
}
#[cfg(feature = "packed-diagnostics")]
impl PackedReadinessSnapshot {
	pub fn add_signal(&mut self, signal: &TileSignal) {
		for (ready, required) in signal.ready.iter().zip(&signal.required) {
			let ready = ready.load(Ordering::Acquire);
			self.batches += 1;
			for (index, bit) in
				[READY_MAIN, READY_PREPASS, READY_SHADOW, READY_BUFFER].into_iter().enumerate()
			{
				self.required[index] += u64::from(required & bit != 0);
				self.acknowledged[index] += u64::from(ready & bit != 0);
			}
			self.missing_masks[usize::from(required & !ready)] += 1;
		}
	}
}

/// All bands are packed once from the same grown source. Only Arc references
/// cross extraction; camera motion never visits individual instances.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AdmissionReason {
	EmptyContent,
	ModeOff,
	UnselectedFamily,
	UnsupportedAdapter,
	UnsupportedGeometry,
	UnsupportedMaterial,
	UnsupportedPose,
	UnsupportedKit,
	ViewPolicy,
	ShadowPolicy,
	Budget,
	PendingAssets,
	PendingGpuReadiness,
	Active,
	RestoringOrdinary,
}
impl AdmissionReason {
	pub fn label(self) -> &'static str {
		match self {
			Self::EmptyContent => "empty_content",
			Self::ModeOff => "mode_off",
			Self::UnselectedFamily => "unselected_family",
			Self::UnsupportedAdapter => "unsupported_adapter",
			Self::UnsupportedGeometry => "unsupported_geometry",
			Self::UnsupportedMaterial => "unsupported_material",
			Self::UnsupportedPose => "unsupported_pose",
			Self::UnsupportedKit => "unsupported_kit",
			Self::ViewPolicy => "view_policy",
			Self::ShadowPolicy => "shadow_policy",
			Self::Budget => "budget",
			Self::PendingAssets => "pending_assets",
			Self::PendingGpuReadiness => "pending_gpu_readiness",
			Self::Active => "active",
			Self::RestoringOrdinary => "restoring_ordinary",
		}
	}
}

pub struct TilePayload {
	pub revision: u64,
	pub proxy_shadows: bool,
	pub bounds: Aabb3d,
	pub center: Vec3,
	pub radius: f32,
	pub structural_lod: vegetation_components::StructuralLod,
	pub plant_count: usize,
	pub lods: [Arc<[PackedBatch]>; 4],
	pub cpu_payload_bytes: u64,
	pub gpu_payload_bytes: u64,
}

impl TilePayload {
	pub fn from_host(
		host: &ChicoGroveHost,
		revision: u64,
		proxy_shadows: bool,
	) -> Result<Self, AdmissionReason> {
		let plants = crate::stick_physics::packed_plants(&host.tile)
			.ok_or(AdmissionReason::UnsupportedAdapter)?;
		let structural_lod =
			host.tile.structural_lod().ok_or(AdmissionReason::UnsupportedAdapter)?;
		let drop = host.drop_out();
		let mut lods: [Arc<[PackedBatch]>; 4] = LEVELS.map(|_| Arc::from([]));
		for (index, level) in LEVELS.into_iter().enumerate() {
			// Validate all bands atomically even if the current stacking layer drops one.
			let batches = pack_grove_band(&host.tile, &plants, level, proxy_shadows)?;
			if !drop.omits(level) {
				lods[index] = Arc::from(batches);
			}
		}

		if lods.iter().all(|batches| batches.is_empty()) {
			return Err(AdmissionReason::EmptyContent);
		}

		if lods
			.iter()
			.flat_map(|batches| batches.iter())
			.flat_map(|batch| batch.instances.iter())
			.any(|instance| {
				!instance.world_from_local.is_finite()
					|| instance.world_from_local.determinant() < 1e-12
			}) {
			return Err(AdmissionReason::UnsupportedPose);
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
		let bounds = host.tile.scene_bounds();
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
		Ok(Self {
			revision,
			proxy_shadows,
			bounds: Aabb3d { min: bounds.min - slack, max: bounds.max + slack },
			center: structural_lod.center,
			radius: structural_lod.tree_radius,
			structural_lod,
			plant_count: plants.len(),
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
	pub(crate) readiness_policy: super::view_policy::ReadinessPolicy,
	pub original: ChicoGroveHost,
	pub payload: Arc<TilePayload>,
	pub signal: Arc<TileSignal>,
	pub proxies: Vec<Entity>,
	pub last_kit_epoch: u64,
}

/// Metadata-only identity for a rejected payload. The change tick conservatively
/// invalidates even an equal replacement and prevents plant-Arc address reuse
/// from making a different source look unchanged. No grown recipe/IR is retained.
#[derive(Clone, Debug, PartialEq)]
pub struct AdmissionKey {
	source_change_tick: u32,
	layer: crate::ForestLayer,
	family: &'static str,
	plant_identity: Option<usize>,
	structural_lod: Option<vegetation_components::StructuralLod>,
	bounds: Aabb3d,
	proxy_shadows: bool,
}

impl AdmissionKey {
	pub fn from_host(host: &ChicoGroveHost, source_change_tick: u32, proxy_shadows: bool) -> Self {
		Self {
			source_change_tick,
			layer: host.layer,
			family: host.tile.family_name(),
			plant_identity: host.tile.plant_list_identity(),
			structural_lod: host.tile.structural_lod(),
			bounds: host.tile.scene_bounds(),
			proxy_shadows,
		}
	}
}

pub struct DeniedAdmission {
	key: AdmissionKey,
	required_gpu_bytes: u64,
}

/// All resident sources stay charged until retirement, including preparing and
/// restoring tiles. Unchanged denials retry only when their known size fits.
#[derive(Resource)]
pub struct PackedGroveCache {
	pub tiles: HashMap<Entity, ResidentTile>,
	pub denied: HashMap<Entity, DeniedAdmission>,
	pub unsupported: HashMap<Entity, (u64, AdmissionReason)>,
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

	pub fn available_gpu_bytes(&self) -> u64 {
		self.budget_bytes.saturating_sub(self.retained_gpu_bytes())
	}

	/// `available` is a current capacity snapshot, rechecked before assembly
	/// after earlier candidates have admitted. Camera movement is not an input.
	pub fn should_build_payload(&self, entity: Entity, key: &AdmissionKey, available: u64) -> bool {
		self.denied
			.get(&entity)
			.is_none_or(|denial| denial.key != *key || denial.required_gpu_bytes <= available)
	}

	pub fn admit(&mut self, entity: Entity, key: AdmissionKey, bytes: u64) -> bool {
		// A changed source may now be smaller even without any retirement.
		// Always decide from its actual new size, never an old denial epoch.
		if bytes > self.available_gpu_bytes() {
			self.denied.insert(entity, DeniedAdmission { key, required_gpu_bytes: bytes });
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
	fn admission_key() -> AdmissionKey {
		let host = ChicoGroveHost::new(
			crate::ForestGroveTile::Orchard(vegetation_groves::Orchard {
				plants: Arc::from([]),
				structural_center: Vec3::ZERO,
				footprint_radius: 1.0,
				extent: vegetation_groves::GroveExtent::new(Vec3::ZERO, Vec3::splat(10.0)),
			}),
			crate::ForestLayer::UpperCanopy,
		);
		AdmissionKey::from_host(&host, 1, false)
	}

	#[test]
	fn unchanged_denial_waits_for_sufficient_capacity_not_epoch_changes() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let key = admission_key();
		let mut cache = PackedGroveCache::with_budget(128);
		assert!(!cache.admit(entity, key.clone(), 256));
		cache.epoch += 1;
		assert!(!cache.should_build_payload(entity, &key, cache.available_gpu_bytes()));
		// A tiny release/budget increase still cannot fit the unchanged payload.
		cache.budget_bytes = 255;
		assert!(!cache.should_build_payload(entity, &key, cache.available_gpu_bytes()));
		assert_eq!(cache.budget_denials, 1);
		cache.budget_bytes = 256;
		assert!(cache.should_build_payload(entity, &key, cache.available_gpu_bytes()));
		assert!(cache.admit(entity, key, 256));
		assert!(!cache.denied.contains_key(&entity));
	}

	#[test]
	fn changed_smaller_source_admits_without_a_retirement() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let key = admission_key();
		let mut cache = PackedGroveCache::with_budget(128);
		assert!(!cache.admit(entity, key.clone(), 256));
		let mut changed = key;
		changed.source_change_tick += 1;
		assert!(cache.should_build_payload(entity, &changed, cache.available_gpu_bytes()));
		assert!(cache.admit(entity, changed, 64));
		assert_eq!(cache.epoch, 0);
	}

	#[test]
	fn source_metadata_policy_and_entity_generation_invalidate_known_size() {
		let mut world = World::new();
		let entity = world.spawn_empty().id();
		let key = admission_key();
		let mut cache = PackedGroveCache::with_budget(128);
		assert!(!cache.admit(entity, key.clone(), 256));
		for changed in [
			AdmissionKey { proxy_shadows: true, ..key.clone() },
			AdmissionKey { source_change_tick: 2, ..key.clone() },
			AdmissionKey { layer: crate::ForestLayer::Understory, ..key.clone() },
			AdmissionKey { family: "replacement", ..key.clone() },
			AdmissionKey { plant_identity: Some(42), ..key.clone() },
			AdmissionKey {
				structural_lod: Some(vegetation_components::StructuralLod::new(Vec3::X, 2.0)),
				..key.clone()
			},
			AdmissionKey {
				bounds: Aabb3d { min: Vec3::splat(-20.0).into(), max: Vec3::splat(20.0).into() },
				..key.clone()
			},
		] {
			assert!(cache.should_build_payload(entity, &changed, cache.available_gpu_bytes()));
		}
		world.despawn(entity);
		let replacement = world.spawn_empty().id();
		assert_ne!(entity, replacement);
		assert!(cache.should_build_payload(replacement, &key, cache.available_gpu_bytes()));
	}

	#[cfg(feature = "packed-diagnostics")]
	#[test]
	fn readiness_snapshot_distinguishes_required_acknowledged_and_missing_passes() {
		let signal = TileSignal::new([
			READY_MAIN | READY_BUFFER,
			READY_SHADOW | READY_BUFFER,
			READY_MAIN | READY_PREPASS | READY_BUFFER,
		]);
		signal.acknowledge(0, READY_MAIN | READY_BUFFER);
		signal.acknowledge(1, READY_BUFFER);
		signal.acknowledge(2, READY_MAIN | READY_BUFFER);
		let mut snapshot = PackedReadinessSnapshot::default();
		snapshot.add_signal(&signal);
		assert_eq!(snapshot.batches, 3);
		assert_eq!(snapshot.required, [2, 1, 1, 3]);
		assert_eq!(snapshot.acknowledged, [2, 0, 0, 3]);
		assert_eq!(snapshot.missing_masks[0], 1);
		assert_eq!(snapshot.missing_masks[usize::from(READY_SHADOW)], 1);
		assert_eq!(snapshot.missing_masks[usize::from(READY_PREPASS)], 1);
		assert!(!signal.is_ready());
	}
}
