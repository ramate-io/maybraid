//! Shared quantized animation samples.
//!
//! Cache identity is clip + interned parameters + compatible rig + quantized clip time.
//! Playback time stays continuous; misses evaluate at the bin's canonical time.
//! Crossfade, aiming, IK, and terrain correction stay outside this cache.

use std::collections::{HashMap, VecDeque};
use std::mem::size_of;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::RwLock;

use bevy::prelude::*;
use character_animations::animations::{Idle, Run, Walk};
use character_animations::{finite_parameter_bits, Effects, DEFAULT_SAMPLE_INTERVAL_US};
use character_rigs::authoring::{humanoid_write_mask, HumanoidPose};

use crate::clip::{AnimClip, AnimId};
use crate::rig::RigSkeletonKind;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

/// Clip-content generation included in [`ClipVariantKey`]. Bump to drop stale tables.
pub const CURRENT_CLIP_REVISION: u64 = 0;

/// Default FIFO cap across all variants.
pub const DEFAULT_MAX_SAMPLES: usize = 4096;
/// Default distinct clip-variant cap.
pub const DEFAULT_MAX_VARIANTS: usize = 64;
/// Per-variant cap so unbounded idle cannot flush walk/run tables.
pub const DEFAULT_MAX_SAMPLES_PER_VARIANT: usize = 256;

const IDLE_BONES: &[&str] = &[
	"lower_neck",
	"upper_neck",
	"pelvis.L",
	"pelvis.R",
	"shoulder.L",
	"shoulder.R",
	"humerus.L",
	"humerus.R",
	"forearm.L",
	"forearm.R",
];

const WALK_BONES: &[&str] = &[
	"root",
	"pelvis.L",
	"pelvis.R",
	"femur.L",
	"femur.R",
	"shin.L",
	"shin.R",
	"shoulder.L",
	"shoulder.R",
	"humerus.L",
	"humerus.R",
	"forearm.L",
	"forearm.R",
];

const RUN_BONES: &[&str] = &[
	"pelvis.L",
	"pelvis.R",
	"femur.L",
	"femur.R",
	"shin.L",
	"shin.R",
	"shoulder.L",
	"shoulder.R",
	"humerus.L",
	"humerus.R",
	"forearm.L",
	"forearm.R",
];

/// Compatible-rig identity. Humanoid V0 is the first cached family.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RigVariantId(pub u16);

impl RigVariantId {
	pub const HUMANOID_V0: Self = Self(1);
	pub const QUADRUPED_V0: Self = Self(2);
	pub const FORELIMBED_V0: Self = Self(3);
}

impl RigSkeletonKind {
	/// Rig variant used in cache keys. `None` for families without a cached sampler.
	pub const fn sample_rig_variant(self) -> Option<RigVariantId> {
		match self {
			Self::Humanoid => Some(RigVariantId::HUMANOID_V0),
			Self::Quadruped => Some(RigVariantId::QUADRUPED_V0),
			Self::Forelimbed => Some(RigVariantId::FORELIMBED_V0),
			Self::Neck => None,
		}
	}
}

/// Interned complete parameter set. Equality is by stored values, not hash alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClipParametersId(pub u32);

/// Pose-affecting knobs after signed-zero normalization.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ClipParametersKey {
	Idle { arm_hang: u32, elbow_hang: u32, arm_sway: u32, neck_roll: u32, hip_shift: u32 },
	Walk { stride: u32, bounce: u32, rotation: u32 },
	Run { stride: u32, bounce: u32, rotation: u32 },
}

impl ClipParametersKey {
	pub fn idle(idle: &Idle) -> Option<Self> {
		Some(Self::Idle {
			arm_hang: finite_parameter_bits(idle.arm_hang)?,
			elbow_hang: finite_parameter_bits(idle.elbow_hang)?,
			arm_sway: finite_parameter_bits(idle.arm_sway)?,
			neck_roll: finite_parameter_bits(idle.neck_roll)?,
			hip_shift: finite_parameter_bits(idle.hip_shift)?,
		})
	}

	pub fn walk(walk: &Walk) -> Option<Self> {
		Some(Self::Walk {
			stride: finite_parameter_bits(walk.stride)?,
			bounce: finite_parameter_bits(walk.bounce)?,
			rotation: finite_parameter_bits(walk.rotation)?,
		})
	}

	pub fn run(run: &Run) -> Option<Self> {
		Some(Self::Run {
			stride: finite_parameter_bits(run.stride)?,
			bounce: finite_parameter_bits(run.bounce)?,
			rotation: finite_parameter_bits(run.rotation)?,
		})
	}
}

/// Shared sample table for one clip / parameter / rig / interval combination.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ClipVariantKey {
	pub clip_id: AnimId,
	pub clip_revision: u64,
	pub parameters_id: ClipParametersId,
	pub rig_variant: RigVariantId,
	pub sample_interval_us: u32,
}

/// One quantized sample inside a variant table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SampleKey {
	pub variant: ClipVariantKey,
	pub sample_index: u32,
}

/// Dense authored channels plus the bones this clip is allowed to write.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AuthoredSample {
	pub pose: HumanoidPose,
	pub bone_mask: u32,
	pub effects: Effects,
}

/// Runtime cache bounds and the uncached comparison switch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimSampleCacheSettings {
	pub enabled: bool,
	pub sample_interval_us: u32,
	pub interpolate: bool,
	pub max_samples: usize,
	pub max_variants: usize,
	pub max_samples_per_variant: usize,
}

impl Default for AnimSampleCacheSettings {
	fn default() -> Self {
		Self {
			enabled: true,
			sample_interval_us: DEFAULT_SAMPLE_INTERVAL_US,
			interpolate: false,
			max_samples: DEFAULT_MAX_SAMPLES,
			max_variants: DEFAULT_MAX_VARIANTS,
			max_samples_per_variant: DEFAULT_MAX_SAMPLES_PER_VARIANT,
		}
	}
}

/// Snapshot of cache traffic and retained size.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AnimSampleCacheStats {
	pub hits: u64,
	pub misses: u64,
	pub inserts: u64,
	pub evictions: u64,
	pub evaluations: u64,
	pub samples: usize,
	pub variants: usize,
	pub interned_parameters: usize,
}

struct ParameterRegistry {
	by_value: HashMap<ClipParametersKey, ClipParametersId>,
	by_id: Vec<ClipParametersKey>,
}

impl ParameterRegistry {
	fn new() -> Self {
		Self { by_value: HashMap::new(), by_id: Vec::new() }
	}

	fn intern(&mut self, key: ClipParametersKey) -> ClipParametersId {
		if let Some(&id) = self.by_value.get(&key) {
			return id;
		}
		let id = ClipParametersId(self.by_id.len() as u32);
		self.by_value.insert(key, id);
		self.by_id.push(key);
		id
	}

	fn get(&self, key: &ClipParametersKey) -> Option<ClipParametersId> {
		self.by_value.get(key).copied()
	}

	fn len(&self) -> usize {
		self.by_id.len()
	}

	fn stored_eq(&self, id: ClipParametersId, key: &ClipParametersKey) -> bool {
		self.by_id.get(id.0 as usize).is_some_and(|stored| stored == key)
	}
}

struct CacheInner {
	samples: HashMap<SampleKey, AuthoredSample>,
	order: VecDeque<SampleKey>,
	variant_order: VecDeque<ClipVariantKey>,
	variant_counts: HashMap<ClipVariantKey, usize>,
	params: ParameterRegistry,
}

impl CacheInner {
	fn new() -> Self {
		Self {
			samples: HashMap::new(),
			order: VecDeque::new(),
			variant_order: VecDeque::new(),
			variant_counts: HashMap::new(),
			params: ParameterRegistry::new(),
		}
	}

	fn get(&self, key: &SampleKey) -> Option<AuthoredSample> {
		self.samples.get(key).copied()
	}

	fn insert(
		&mut self,
		key: SampleKey,
		sample: AuthoredSample,
		settings: &AnimSampleCacheSettings,
	) {
		if self.samples.contains_key(&key) {
			self.samples.insert(key, sample);
			return;
		}
		if !self.variant_counts.contains_key(&key.variant)
			&& self.variant_counts.len() >= settings.max_variants
		{
			self.evict_oldest_variant();
		}
		self.samples.insert(key, sample);
		self.order.push_back(key);
		*self.variant_counts.entry(key.variant).or_insert(0) += 1;
		if !self.variant_order.contains(&key.variant) {
			self.variant_order.push_back(key.variant);
		}
		self.enforce_variant_cap(key.variant, settings.max_samples_per_variant);
		while self.samples.len() > settings.max_samples {
			self.evict_oldest_sample();
		}
	}

	fn enforce_variant_cap(&mut self, variant: ClipVariantKey, max_per_variant: usize) {
		while self.variant_counts.get(&variant).copied().unwrap_or(0) > max_per_variant {
			if !self.evict_oldest_of(variant) {
				break;
			}
		}
	}

	fn evict_oldest_of(&mut self, variant: ClipVariantKey) -> bool {
		let Some(position) = self.order.iter().position(|key| key.variant == variant) else {
			return false;
		};
		if let Some(key) = self.order.remove(position) {
			self.remove_sample(key);
			return true;
		}
		false
	}

	fn evict_oldest_sample(&mut self) {
		if let Some(key) = self.order.pop_front() {
			self.remove_sample(key);
		}
	}

	fn evict_oldest_variant(&mut self) {
		let Some(variant) = self.variant_order.pop_front() else {
			return;
		};
		self.invalidate_variant(variant);
	}

	fn remove_sample(&mut self, key: SampleKey) {
		if self.samples.remove(&key).is_some() {
			if let Some(count) = self.variant_counts.get_mut(&key.variant) {
				*count = count.saturating_sub(1);
				if *count == 0 {
					self.variant_counts.remove(&key.variant);
					if let Some(position) =
						self.variant_order.iter().position(|item| *item == key.variant)
					{
						self.variant_order.remove(position);
					}
				}
			}
		}
	}

	fn invalidate_variant(&mut self, variant: ClipVariantKey) {
		self.order.retain(|key| key.variant != variant);
		self.samples.retain(|key, _| key.variant != variant);
		self.variant_counts.remove(&variant);
	}

	fn invalidate_clip(&mut self, clip_id: AnimId) {
		let variants: Vec<ClipVariantKey> = self
			.variant_counts
			.keys()
			.copied()
			.filter(|variant| variant.clip_id == clip_id)
			.collect();
		for variant in variants {
			self.invalidate_variant(variant);
		}
	}

	fn invalidate_revision(&mut self, clip_id: AnimId, revision: u64) {
		let variants: Vec<ClipVariantKey> = self
			.variant_counts
			.keys()
			.copied()
			.filter(|variant| variant.clip_id == clip_id && variant.clip_revision == revision)
			.collect();
		for variant in variants {
			self.invalidate_variant(variant);
		}
	}

	fn invalidate_parameters(&mut self, parameters_id: ClipParametersId) {
		let variants: Vec<ClipVariantKey> = self
			.variant_counts
			.keys()
			.copied()
			.filter(|variant| variant.parameters_id == parameters_id)
			.collect();
		for variant in variants {
			self.invalidate_variant(variant);
		}
	}

	fn invalidate_rig(&mut self, rig_variant: RigVariantId) {
		let variants: Vec<ClipVariantKey> = self
			.variant_counts
			.keys()
			.copied()
			.filter(|variant| variant.rig_variant == rig_variant)
			.collect();
		for variant in variants {
			self.invalidate_variant(variant);
		}
	}

	fn clear(&mut self) {
		self.samples.clear();
		self.order.clear();
		self.variant_order.clear();
		self.variant_counts.clear();
	}
}

/// Shared evaluated samples. Interior-mutable so mailbox apply can run in parallel.
#[derive(Resource)]
pub struct AnimSampleCache {
	pub settings: AnimSampleCacheSettings,
	inner: RwLock<CacheInner>,
	hits: AtomicU64,
	misses: AtomicU64,
	inserts: AtomicU64,
	evictions: AtomicU64,
	evaluations: AtomicU64,
}

impl Default for AnimSampleCache {
	fn default() -> Self {
		Self {
			settings: AnimSampleCacheSettings::default(),
			inner: RwLock::new(CacheInner::new()),
			hits: AtomicU64::new(0),
			misses: AtomicU64::new(0),
			inserts: AtomicU64::new(0),
			evictions: AtomicU64::new(0),
			evaluations: AtomicU64::new(0),
		}
	}
}

impl AnimSampleCache {
	pub fn with_settings(settings: AnimSampleCacheSettings) -> Self {
		Self { settings, ..Self::default() }
	}

	pub fn uncached() -> Self {
		Self::with_settings(AnimSampleCacheSettings { enabled: false, ..Default::default() })
	}

	fn lock_read(&self) -> std::sync::RwLockReadGuard<'_, CacheInner> {
		self.inner.read().unwrap_or_else(|poisoned| poisoned.into_inner())
	}

	fn lock_write(&self) -> std::sync::RwLockWriteGuard<'_, CacheInner> {
		self.inner.write().unwrap_or_else(|poisoned| poisoned.into_inner())
	}

	/// Intern by complete value comparison. A raw hash never establishes identity.
	pub fn intern(&self, key: ClipParametersKey) -> ClipParametersId {
		{
			let inner = self.lock_read();
			if let Some(id) = inner.params.get(&key) {
				return id;
			}
		}
		let mut inner = self.lock_write();
		inner.params.intern(key)
	}

	pub fn parameters_eq(&self, id: ClipParametersId, key: &ClipParametersKey) -> bool {
		self.lock_read().params.stored_eq(id, key)
	}

	pub fn get(&self, key: &SampleKey) -> Option<AuthoredSample> {
		self.lock_read().get(key)
	}

	pub fn get_or_insert_with(
		&self,
		key: SampleKey,
		build: impl FnOnce() -> AuthoredSample,
	) -> AuthoredSample {
		{
			let inner = self.lock_read();
			if let Some(sample) = inner.get(&key) {
				self.hits.fetch_add(1, Ordering::Relaxed);
				return sample;
			}
		}
		self.evaluations.fetch_add(1, Ordering::Relaxed);
		let sample = build();
		let mut inner = self.lock_write();
		if let Some(existing) = inner.get(&key) {
			self.hits.fetch_add(1, Ordering::Relaxed);
			return existing;
		}
		let before = inner.samples.len();
		inner.insert(key, sample, &self.settings);
		let evicted = before.saturating_add(1).saturating_sub(inner.samples.len());
		self.misses.fetch_add(1, Ordering::Relaxed);
		self.inserts.fetch_add(1, Ordering::Relaxed);
		if evicted > 0 {
			self.evictions.fetch_add(evicted as u64, Ordering::Relaxed);
		}
		sample
	}

	pub fn stats(&self) -> AnimSampleCacheStats {
		let inner = self.lock_read();
		AnimSampleCacheStats {
			hits: self.hits.load(Ordering::Relaxed),
			misses: self.misses.load(Ordering::Relaxed),
			inserts: self.inserts.load(Ordering::Relaxed),
			evictions: self.evictions.load(Ordering::Relaxed),
			evaluations: self.evaluations.load(Ordering::Relaxed),
			samples: inner.samples.len(),
			variants: inner.variant_counts.len(),
			interned_parameters: inner.params.len(),
		}
	}

	pub fn retained_bytes(&self) -> usize {
		let inner = self.lock_read();
		inner.samples.len() * size_of::<AuthoredSample>()
			+ inner.samples.len() * size_of::<SampleKey>()
	}

	pub fn invalidate_clip(&self, clip_id: AnimId) {
		self.lock_write().invalidate_clip(clip_id);
	}

	pub fn invalidate_revision(&self, clip_id: AnimId, revision: u64) {
		self.lock_write().invalidate_revision(clip_id, revision);
	}

	pub fn invalidate_parameters(&self, parameters_id: ClipParametersId) {
		self.lock_write().invalidate_parameters(parameters_id);
	}

	pub fn invalidate_rig(&self, rig_variant: RigVariantId) {
		self.lock_write().invalidate_rig(rig_variant);
	}

	pub fn clear(&self) {
		self.lock_write().clear();
	}

	/// Sample a cacheable humanoid clip. `None` means the caller should use the uncached path.
	pub fn sample_humanoid(
		&self,
		clip: AnimClip,
		rig: &mut HumanoidV0Rig,
		progress: f32,
		write_bones: bool,
		write_effects: bool,
	) -> Option<Effects> {
		if !self.settings.enabled || self.settings.interpolate || !clip.is_sample_cacheable() {
			return None;
		}
		let parameters = parameters_key(clip)?;
		let parameters_id = self.intern(parameters);
		let interval = self.settings.sample_interval_us;
		let variant = ClipVariantKey {
			clip_id: clip.id(),
			clip_revision: clip.revision(),
			parameters_id,
			rig_variant: RigVariantId::HUMANOID_V0,
			sample_interval_us: interval,
		};
		let address = clip.sample_address(progress, interval);
		let key = SampleKey { variant, sample_index: address.index };
		let sample = self.get_or_insert_with(key, || {
			authored_sample(clip, address.canonical_time).unwrap_or(AuthoredSample {
				pose: HumanoidPose::default(),
				bone_mask: 0,
				effects: Effects::IDENTITY,
			})
		});
		if write_bones {
			rig.apply_masked_pose(&sample.pose, sample.bone_mask);
		}
		Some(if write_effects { sample.effects } else { Effects::IDENTITY })
	}
}

/// Pose-affecting knobs for a cacheable clip. `None` bypasses the cache.
pub fn parameters_key(clip: AnimClip) -> Option<ClipParametersKey> {
	match clip {
		AnimClip::Still => ClipParametersKey::idle(&Idle::default()),
		AnimClip::Walk(walk) => ClipParametersKey::walk(&walk),
		AnimClip::Run(run) => ClipParametersKey::run(&run),
		_ => None,
	}
}

/// Bones this cacheable clip is allowed to write.
pub fn clip_bone_mask(clip: AnimClip) -> Option<u32> {
	match clip {
		AnimClip::Still => Some(humanoid_write_mask(IDLE_BONES)),
		AnimClip::Walk(_) => Some(humanoid_write_mask(WALK_BONES)),
		AnimClip::Run(_) => Some(humanoid_write_mask(RUN_BONES)),
		_ => None,
	}
}

/// Evaluate authored channels at `canonical_time`. Never uses the requester's exact time.
pub fn authored_sample(clip: AnimClip, canonical_time: f32) -> Option<AuthoredSample> {
	let bone_mask = clip_bone_mask(clip)?;
	let pose = match clip {
		AnimClip::Still => Idle::default().sample_pose(canonical_time),
		AnimClip::Walk(walk) => walk.sample_pose(canonical_time),
		AnimClip::Run(run) => run.sample_pose(canonical_time),
		_ => return None,
	};
	Some(AuthoredSample { pose, bone_mask, effects: Effects::IDENTITY })
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_animations::{Animation, DEFAULT_SAMPLE_INTERVAL_US};
	use character_rigs::authoring::humanoid_bone_bit;
	use std::time::Instant;

	fn variant(
		cache: &AnimSampleCache,
		clip: AnimClip,
		rig: RigVariantId,
	) -> anyhow::Result<ClipVariantKey> {
		let parameters = parameters_key(clip).ok_or_else(|| anyhow::anyhow!("uncacheable clip"))?;
		Ok(ClipVariantKey {
			clip_id: clip.id(),
			clip_revision: clip.revision(),
			parameters_id: cache.intern(parameters),
			rig_variant: rig,
			sample_interval_us: cache.settings.sample_interval_us,
		})
	}

	fn lookup(
		cache: &AnimSampleCache,
		clip: AnimClip,
		clip_time: f32,
	) -> anyhow::Result<AuthoredSample> {
		let variant = variant(cache, clip, RigVariantId::HUMANOID_V0)?;
		let address = clip.sample_address(clip_time, cache.settings.sample_interval_us);
		let key = SampleKey { variant, sample_index: address.index };
		Ok(cache.get_or_insert_with(key, || {
			authored_sample(clip, address.canonical_time).unwrap_or(AuthoredSample {
				pose: HumanoidPose::default(),
				bone_mask: 0,
				effects: Effects::IDENTITY,
			})
		}))
	}

	#[test]
	fn intern_compares_complete_values_and_signed_zero() -> anyhow::Result<()> {
		let cache = AnimSampleCache::default();
		let walk = Walk { stride: 0.0, bounce: 1.0, rotation: 1.0 };
		let signed_zero = Walk { stride: -0.0, bounce: 1.0, rotation: 1.0 };
		let other = Walk { stride: 0.8, bounce: 1.0, rotation: 1.0 };
		let a = ClipParametersKey::walk(&walk).ok_or_else(|| anyhow::anyhow!("finite walk"))?;
		let z =
			ClipParametersKey::walk(&signed_zero).ok_or_else(|| anyhow::anyhow!("signed zero"))?;
		let b = ClipParametersKey::walk(&other).ok_or_else(|| anyhow::anyhow!("other walk"))?;
		let id_a = cache.intern(a);
		let id_z = cache.intern(z);
		let id_b = cache.intern(b);
		if id_a != id_z {
			anyhow::bail!("signed zero must intern with +0");
		}
		if id_a == id_b {
			anyhow::bail!("different strides must not share a parameters id");
		}
		if !cache.parameters_eq(id_a, &a) || cache.parameters_eq(id_a, &b) {
			anyhow::bail!("intern identity must compare stored values, not hash alone");
		}
		Ok(())
	}

	#[test]
	fn non_finite_parameters_bypass_the_cache() {
		let walk = Walk { stride: f32::NAN, bounce: 1.0, rotation: 1.0 };
		assert!(ClipParametersKey::walk(&walk).is_none());
		assert!(parameters_key(AnimClip::jab()).is_none());
	}

	#[test]
	fn characters_share_a_canonical_bin() -> anyhow::Result<()> {
		let cache = AnimSampleCache::default();
		let clip = AnimClip::walk();
		let first = lookup(&cache, clip, 0.121)?;
		let second = lookup(&cache, clip, 0.123)?;
		let stats = cache.stats();
		if stats.evaluations != 1 || stats.hits != 1 || stats.misses != 1 {
			anyhow::bail!("expected one miss and one hit, got {stats:?}");
		}
		if first.pose != second.pose {
			anyhow::bail!("shared bin must reuse the same authored pose");
		}
		let canonical = clip.sample_address(0.121, DEFAULT_SAMPLE_INTERVAL_US).canonical_time;
		let expected = Walk::default().sample_pose(canonical);
		if first.pose != expected {
			anyhow::bail!(
				"miss must evaluate at canonical time {canonical}, not the requester time"
			);
		}
		if first.pose == Walk::default().sample_pose(0.121) && (canonical - 0.121).abs() > 1e-6 {
			anyhow::bail!("cached pose must not be the first requester's exact time");
		}
		Ok(())
	}

	#[test]
	fn different_parameters_revisions_and_rigs_separate() -> anyhow::Result<()> {
		let cache = AnimSampleCache::default();
		let walk = AnimClip::walk();
		let long = AnimClip::Walk(Walk { stride: 0.9, bounce: 1.0, rotation: 1.0 });
		lookup(&cache, walk, 0.2)?;
		lookup(&cache, long, 0.2)?;
		lookup(&cache, AnimClip::run(), 0.2)?;
		lookup(&cache, AnimClip::still(), 0.2)?;
		let walk_variant = variant(&cache, walk, RigVariantId::HUMANOID_V0)?;
		let other_rig = ClipVariantKey { rig_variant: RigVariantId::QUADRUPED_V0, ..walk_variant };
		let address = walk.sample_address(0.2, DEFAULT_SAMPLE_INTERVAL_US);
		let Some(sample) = authored_sample(walk, address.canonical_time) else {
			anyhow::bail!("walk sample");
		};
		cache.get_or_insert_with(
			SampleKey { variant: other_rig, sample_index: address.index },
			|| sample,
		);
		let revised = ClipVariantKey { clip_revision: 7, ..walk_variant };
		cache.get_or_insert_with(
			SampleKey { variant: revised, sample_index: address.index },
			|| sample,
		);
		let stats = cache.stats();
		if stats.variants < 5 {
			anyhow::bail!("params, clip, rig, and revision must not share tables, got {stats:?}");
		}
		Ok(())
	}

	#[test]
	fn warm_hit_skips_evaluation() -> anyhow::Result<()> {
		let cache = AnimSampleCache::default();
		lookup(&cache, AnimClip::walk(), 0.41)?;
		lookup(&cache, AnimClip::walk(), 0.41)?;
		lookup(&cache, AnimClip::walk(), 0.409)?;
		if cache.stats().evaluations != 1 {
			anyhow::bail!("warm hits must not evaluate the clip, got {:?}", cache.stats());
		}
		Ok(())
	}

	#[test]
	fn idle_mask_leaves_femur_to_the_rest_baseline() -> anyhow::Result<()> {
		let Some(mask) = clip_bone_mask(AnimClip::still()) else {
			anyhow::bail!("idle mask");
		};
		let Some(femur) = humanoid_bone_bit("femur.L") else {
			anyhow::bail!("femur bit");
		};
		if mask & femur != 0 {
			anyhow::bail!("idle must not claim the femur channel");
		}
		let Some(walk_mask) = clip_bone_mask(AnimClip::walk()) else {
			anyhow::bail!("walk mask");
		};
		if walk_mask & femur == 0 {
			anyhow::bail!("walk must write the femur");
		}
		let mut leftover = HumanoidV0Rig::imported();
		Walk::default().apply_for(&mut leftover, 0.2);
		let posed = leftover.posed_angle("femur.L");
		if posed < 0.05 {
			anyhow::bail!("setup walk should pose the femur");
		}
		let Some(sample) = authored_sample(AnimClip::still(), 0.25) else {
			anyhow::bail!("idle sample");
		};
		leftover.apply_masked_pose(&sample.pose, sample.bone_mask);
		if leftover.posed_angle("femur.L") > 1e-5 {
			anyhow::bail!("masked idle apply uses rest as baseline, femur should return to rest");
		}
		if leftover.posed_angle("humerus.L") < 0.2 {
			anyhow::bail!("idle should still hang the arms");
		}
		Ok(())
	}

	#[test]
	fn walk_loop_seam_reuses_bin_zero() -> anyhow::Result<()> {
		let cache = AnimSampleCache::default();
		lookup(&cache, AnimClip::walk(), 0.996)?;
		lookup(&cache, AnimClip::walk(), 1.004)?;
		lookup(&cache, AnimClip::walk(), 0.0)?;
		if cache.stats().evaluations != 1 {
			anyhow::bail!("loop seam and 0 must share bin 0, got {:?}", cache.stats());
		}
		Ok(())
	}

	#[test]
	fn invalidation_drops_stale_variants() -> anyhow::Result<()> {
		let cache = AnimSampleCache::default();
		lookup(&cache, AnimClip::walk(), 0.3)?;
		lookup(&cache, AnimClip::run(), 0.3)?;
		cache.invalidate_clip(AnimId::Walk);
		if cache.stats().variants != 1 {
			anyhow::bail!("walk invalidation should leave run, got {:?}", cache.stats());
		}
		lookup(&cache, AnimClip::walk(), 0.3)?;
		if cache.stats().evaluations != 3 {
			anyhow::bail!("invalidated walk must miss again, got {:?}", cache.stats());
		}
		let walk_params =
			parameters_key(AnimClip::walk()).ok_or_else(|| anyhow::anyhow!("walk"))?;
		cache.invalidate_parameters(cache.intern(walk_params));
		cache.invalidate_rig(RigVariantId::HUMANOID_V0);
		cache.invalidate_revision(AnimId::Run, CURRENT_CLIP_REVISION);
		if cache.stats().samples != 0 {
			anyhow::bail!("rig/parameter/revision invalidation should empty the cache");
		}
		Ok(())
	}

	#[test]
	fn bounds_evict_oldest_and_cap_unbounded_idle() -> anyhow::Result<()> {
		let cache = AnimSampleCache::with_settings(AnimSampleCacheSettings {
			max_samples: 8,
			max_variants: 2,
			max_samples_per_variant: 3,
			..AnimSampleCacheSettings::default()
		});
		for index in 0..8 {
			lookup(&cache, AnimClip::still(), index as f32 * 0.02)?;
		}
		if cache.stats().samples > 3 {
			anyhow::bail!("idle must respect the per-variant cap, got {:?}", cache.stats());
		}
		lookup(&cache, AnimClip::walk(), 0.1)?;
		lookup(&cache, AnimClip::run(), 0.1)?;
		if cache.stats().variants > 2 {
			anyhow::bail!("variant cap should evict the oldest table, got {:?}", cache.stats());
		}
		Ok(())
	}

	#[test]
	fn disabled_cache_uses_the_uncached_path() -> anyhow::Result<()> {
		let cache = AnimSampleCache::uncached();
		let mut rig = HumanoidV0Rig::imported();
		if cache.sample_humanoid(AnimClip::walk(), &mut rig, 0.25, true, true).is_some() {
			anyhow::bail!("disabled cache must return None so mailbox can sample live");
		}
		Ok(())
	}

	#[test]
	fn playback_speeds_share_bins_and_stay_near_live_pose() -> anyhow::Result<()> {
		let cache = AnimSampleCache::default();
		let clip = AnimClip::walk();
		let speeds = [1.08, 0.3, 2.4];
		let mut live = HumanoidV0Rig::imported();
		let mut quantized = HumanoidV0Rig::imported();
		for speed in speeds {
			let mut clip_time = 0.0;
			for _ in 0..40 {
				clip_time += 1.0 / 60.0 * speed;
				let address = clip.sample_address(clip_time, DEFAULT_SAMPLE_INTERVAL_US);
				lookup(&cache, clip, clip_time)?;
				Walk::default().apply_for(&mut live, clip_time);
				Walk::default().apply_for(&mut quantized, address.canonical_time);
				let delta = live.posed_angle("femur.L") - quantized.posed_angle("femur.L");
				if delta.abs() > 0.08 {
					anyhow::bail!(
						"10 ms bins should stay close to live walk at speed {speed}, delta {delta}"
					);
				}
			}
		}
		if cache.stats().hits == 0 {
			anyhow::bail!("repeated speeds should reuse walk bins");
		}
		Ok(())
	}

	#[test]
	fn different_rest_shares_authored_channels() -> anyhow::Result<()> {
		let cache = AnimSampleCache::default();
		let clip = AnimClip::walk();
		let mut a = HumanoidV0Rig::imported();
		let mut b = HumanoidV0Rig::imported();
		b.seed_rest("femur.L", Transform::from_rotation(Quat::from_rotation_x(0.15)));
		if cache.sample_humanoid(clip, &mut a, 0.2, true, true).is_none() {
			anyhow::bail!("cached walk on default rest");
		}
		if cache.sample_humanoid(clip, &mut b, 0.204, true, true).is_none() {
			anyhow::bail!("cached walk on edited rest");
		}
		if cache.stats().evaluations != 1 {
			anyhow::bail!("compatible humanoid V0 rests must share the authored sample");
		}
		if a.rotation("femur.L") == b.rotation("femur.L") {
			anyhow::bail!("masked apply must compose onto each character's rest");
		}
		Ok(())
	}

	#[test]
	fn benchmark_idle_walk_run_against_uncached() -> anyhow::Result<()> {
		const CHARACTERS: usize = 64;
		const FRAMES: usize = 90;
		let dt = 1.0 / 60.0;
		let clips = [AnimClip::still(), AnimClip::walk(), AnimClip::run()];
		let speeds = [0.2, 1.08, 1.68];

		let uncached_start = Instant::now();
		let mut uncached_allocs = 0usize;
		for (clip, speed) in clips.into_iter().zip(speeds) {
			for character in 0..CHARACTERS {
				let mut rig = HumanoidV0Rig::imported();
				uncached_allocs += 1;
				let mut time = character as f32 * 0.01;
				for _ in 0..FRAMES {
					time += dt * speed;
					match clip {
						AnimClip::Still => Idle::default().apply_for(&mut rig, time),
						AnimClip::Walk(walk) => walk.apply_for(&mut rig, time),
						AnimClip::Run(run) => run.apply_for(&mut rig, time),
						_ => {}
					}
				}
			}
		}
		let uncached_ms = uncached_start.elapsed().as_secs_f64() * 1_000.0;

		let cache = AnimSampleCache::default();
		let cold_start = Instant::now();
		for (clip, speed) in clips.into_iter().zip(speeds) {
			let mut rig = HumanoidV0Rig::imported();
			if cache.sample_humanoid(clip, &mut rig, 0.0, true, true).is_none() {
				anyhow::bail!("cold sample");
			}
			let _ = speed;
		}
		let cold_ms = cold_start.elapsed().as_secs_f64() * 1_000.0;

		let mut replay = Vec::new();
		for (clip, speed) in clips.into_iter().zip(speeds) {
			for character in 0..CHARACTERS {
				let mut rig = HumanoidV0Rig::imported();
				let mut time = character as f32 * 0.01;
				for _ in 0..FRAMES {
					time += dt * speed;
					replay.push((clip, time));
					if cache.sample_humanoid(clip, &mut rig, time, true, true).is_none() {
						anyhow::bail!("fill sample");
					}
				}
			}
		}
		let mut warm_rig = HumanoidV0Rig::imported();
		let warm_start = Instant::now();
		for (clip, time) in &replay {
			if cache.sample_humanoid(*clip, &mut warm_rig, *time, true, true).is_none() {
				anyhow::bail!("warm sample");
			}
		}
		let warm_ms = warm_start.elapsed().as_secs_f64() * 1_000.0;
		let lockstep_start = Instant::now();
		let lockstep_clip = AnimClip::walk();
		for _ in 0..CHARACTERS {
			let mut rig = HumanoidV0Rig::imported();
			for frame in 0..FRAMES {
				let time = frame as f32 * dt * 1.08;
				if cache.sample_humanoid(lockstep_clip, &mut rig, time, true, true).is_none() {
					anyhow::bail!("lockstep sample");
				}
			}
		}
		let lockstep_ms = lockstep_start.elapsed().as_secs_f64() * 1_000.0;
		let stats = cache.stats();
		if stats.hits == 0 || stats.samples == 0 {
			anyhow::bail!("warm playback should reuse samples, got {stats:?}");
		}

		eprintln!(
			"anim sample cache {CHARACTERS} chars × {FRAMES} frames × 3 clips: \
			uncached {uncached_ms:.2}ms ({uncached_allocs} rigs), \
			cold construct {cold_ms:.3}ms, \
			warm replay {warm_ms:.2}ms, \
			lockstep walk {lockstep_ms:.2}ms, hits {}, misses {}, evals {}, \
			retained {} samples / {} variants / {} bytes",
			stats.hits,
			stats.misses,
			stats.evaluations,
			stats.samples,
			stats.variants,
			cache.retained_bytes()
		);
		Ok(())
	}
}
