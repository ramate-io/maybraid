//! Revision-owned resident payloads. Visibility never owns cache residency.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::Arc;

use bevy::prelude::*;
use bevy::shape::Aabb3d;
use lod::LodSceneLevel;

use crate::packed::instances::{pack_grove_band, PackedBatch};
use crate::ChicoGroveHost;

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

#[derive(Clone)]
pub struct TilePayload {
	#[cfg(feature = "packed-diagnostics")]
	pub diagnostic_family: &'static str,
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

		Ok(Self {
			#[cfg(feature = "packed-diagnostics")]
			diagnostic_family: host.tile.family_name(),
			revision,
			proxy_shadows,
			// Provisional indexing bounds; replaced by geometric bounds before any proxy exists.
			bounds,
			center: structural_lod.center,
			radius: structural_lod.tree_radius,
			structural_lod,
			plant_count: plants.len(),
			lods,
			cpu_payload_bytes,
			gpu_payload_bytes: instances * 176,
		})
	}

	/// Union every rendered LOD and shadow kit in world space. Authored LOD
	/// bands stay separate: structural footprint metadata is not mesh bounds.
	pub fn geometric_bounds(&self, kits: &crate::packed::kit::PackedKitMeshes) -> Option<Aabb3d> {
		let _span = bevy::log::tracing::trace_span!(target:"maybraid.packed.profile", "packed_geometry_bounds").entered();
		let mut union: Option<(Vec3, Vec3)> = None;
		for batch in self.lods.iter().flat_map(|batches| batches.iter()) {
			let local = kits.ready_bounds(batch.key.kit)?;
			let center = Vec3::from((local.min + local.max) * 0.5);
			let half = Vec3::from((local.max - local.min) * 0.5);
			for instance in batch.instances.iter() {
				let model = instance.world_from_local;
				let center = model.transform_point3(center);
				let extent = model.x_axis.truncate().abs() * half.x
					+ model.y_axis.truncate().abs() * half.y
					+ model.z_axis.truncate().abs() * half.z;
				let wind = if batch.key.material_kind == lod::VisualMaterialKind::Leaf {
					let scale = if instance.merged_leaf {
						instance
							.collection_root
							.x_axis
							.truncate()
							.length()
							.max(instance.collection_root.y_axis.truncate().length())
							.max(instance.collection_root.z_axis.truncate().length())
							* instance.part_scale.abs()
					} else {
						model
							.x_axis
							.truncate()
							.length()
							.max(model.y_axis.truncate().length())
							.max(model.z_axis.truncate().length())
					};
					0.108 * scale
				} else {
					0.0
				};
				let extent = extent + Vec3::splat(wind);
				let (min, max) = (center - extent, center + extent);
				if !min.is_finite() || !max.is_finite() {
					return None;
				}
				union = Some(match union {
					Some((old_min, old_max)) => (old_min.min(min), old_max.max(max)),
					None => (min, max),
				});
			}
		}
		union.map(|(min, max)| Aabb3d::from_min_max(min, max))
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

/// Active sources are pinned. A denied source retries only after an admission
/// epoch changes (retirement/budget/source change), never because a camera moves.
#[derive(Resource)]
pub struct PackedGroveCache {
	pub tiles: HashMap<Entity, ResidentTile>,
	pub denied: HashMap<Entity, u64>,
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
