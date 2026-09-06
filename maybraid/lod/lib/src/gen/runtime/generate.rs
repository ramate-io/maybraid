//! Generate-region production and budgeted index insert.

use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use bevy::ecs::component::Mutable;
use bevy::ecs::query::QueryFilter;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::gen::{
	entering_keep_regions, expand_keep_xz, id_lives_in_keep, id_xz_distance2, keep_region_changed,
	GeneratingSpatialIndex, GenerationScheme, Id, MaterializeStatus, StorageStatus,
	QUEUE_KEEP_SLACK_XZ,
};
use crate::lod_ref::{
	collect_node_snapshots, lod_refs_from_snapshots, snapshot_node, LodNode, LodNodeBounds,
	LodNodePlugin, LodNodePose, LodNodeSystems,
};
use crate::scene::{arm_keep_if_empty, LodRefreshRegions, LodRefreshRegionsStatus};

/// Impulse: generate ids originating in `region` (channel `M`).
#[derive(Message, Debug, Clone)]
pub struct LodGenerateRegion<M: Send + Sync + 'static> {
	pub region: Aabb3d,
	pub _marker: PhantomData<M>,
}

impl<M: Send + Sync + 'static> LodGenerateRegion<M> {
	pub fn new(region: Aabb3d) -> Self {
		Self { region, _marker: PhantomData }
	}
}

/// One origin id inserted by generate.
///
/// Present consumes this impulse instead of broadphasing the unchanged keep
/// ring every frame.
#[derive(Message, Debug, Clone, Copy)]
pub struct LodGenerated<T: Send + Sync + 'static> {
	pub id: Id,
	pub _marker: PhantomData<T>,
}

impl<T: Send + Sync + 'static> LodGenerated<T> {
	pub fn new(id: Id) -> Self {
		Self { id, _marker: PhantomData }
	}
}

/// How many origin ids each generate drain may materialize per frame.
///
/// Independent of scene and presentation.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LodGenerateBudget {
	pub ids_per_frame: u32,
}

impl Default for LodGenerateBudget {
	fn default() -> Self {
		Self { ids_per_frame: 1 }
	}
}

/// Independent wall-clock guard applied by each generate drain.
///
/// [`LodGenerateBudget`] remains an ID ceiling. The drain stops before its next
/// region or ID quantum after this duration has elapsed.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LodGenerateTimeBudget {
	/// Per-drain wall time. Zero disables the time limit.
	pub time_per_frame: Duration,
	/// Warn when one non-interruptible region/ID quantum exceeds this duration.
	/// Zero disables warnings.
	pub max_atomic_cost: Duration,
}

impl Default for LodGenerateTimeBudget {
	fn default() -> Self {
		Self { time_per_frame: Duration::from_millis(2), max_atomic_cost: Duration::from_millis(3) }
	}
}

/// Pending origin ids for type `T`.
#[derive(Resource, Debug)]
pub struct LodGenerateQueue<T> {
	pending: VecDeque<Id>,
	pending_ids: HashSet<Id>,
	scan_regions: VecDeque<Aabb3d>,
	reset_scan: bool,
	_marker: PhantomData<T>,
}

impl<T> Default for LodGenerateQueue<T> {
	fn default() -> Self {
		Self {
			pending: VecDeque::new(),
			pending_ids: HashSet::new(),
			scan_regions: VecDeque::new(),
			reset_scan: true,
			_marker: PhantomData,
		}
	}
}

impl<T> LodGenerateQueue<T> {
	pub fn is_empty(&self) -> bool {
		self.pending.is_empty()
	}

	pub fn contains(&self, id: &Id) -> bool {
		self.pending_ids.contains(id)
	}

	pub fn clear(&mut self) {
		self.pending.clear();
		self.pending_ids.clear();
		self.scan_regions.clear();
		self.reset_scan = true;
	}

	pub fn enqueue(&mut self, id: Id) -> bool {
		if !self.pending_ids.insert(id) {
			return false;
		}
		self.pending.push_back(id);
		true
	}

	fn pop_front(&mut self) -> Option<Id> {
		let id = self.pending.pop_front()?;
		self.pending_ids.remove(&id);
		Some(id)
	}

	fn enqueue_scan(&mut self, region: Aabb3d) {
		if !self.scan_regions.iter().any(|queued| regions_match(*queued, region)) {
			self.scan_regions.push_back(region);
		}
	}

	fn take_scan_reset(&mut self) -> bool {
		std::mem::take(&mut self.reset_scan)
	}

	fn expire_outside_keep(&mut self, keep: Option<Aabb3d>, slack: f32) {
		let Some(keep) = keep else {
			return;
		};
		self.pending.retain(|id| id_lives_in_keep(*id, keep, slack));
		self.pending_ids.clear();
		self.pending_ids.extend(self.pending.iter().copied());
		let live = expand_keep_xz(keep, slack);
		self.scan_regions.retain(|region| regions_overlap_xz(*region, live));
	}
}

/// Last generate-ring AABB for this channel.
///
/// [`Self::slack_xz`] is the live margin around [`Self::region`] (queue expire).
#[derive(Resource, Debug)]
pub struct LodGenerateKeepRegion<M: Send + Sync + 'static> {
	pub region: Option<Aabb3d>,
	/// XZ expand of [`Self::region`] for pending-id expiry. World-overridable.
	pub slack_xz: f32,
	pub _marker: PhantomData<M>,
}

impl<M: Send + Sync + 'static> Default for LodGenerateKeepRegion<M> {
	fn default() -> Self {
		Self { region: None, slack_xz: QUEUE_KEEP_SLACK_XZ, _marker: PhantomData }
	}
}

impl<M: Send + Sync + 'static> LodGenerateKeepRegion<M> {
	/// Keep AABB expanded by [`Self::slack_xz`] on XZ. `None` before the first produce.
	pub fn live_region(&self) -> Option<Aabb3d> {
		self.region.map(|region| expand_keep_xz(region, self.slack_xz))
	}
}

/// Ordering inside the generate layer (not the scene stack).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodGenerateSystems {
	Produce,
	Drain,
}

pub(super) fn ensure_generate_sets(app: &mut App) {
	if app.is_plugin_added::<LodGenerateSetsPlugin>() {
		return;
	}
	app.add_plugins(LodGenerateSetsPlugin);
}

struct LodGenerateSetsPlugin;

impl Plugin for LodGenerateSetsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<LodNodePlugin>() {
			app.add_plugins(LodNodePlugin);
		}
		app.init_resource::<LodGenerateBudget>()
			.init_resource::<LodGenerateTimeBudget>()
			.configure_sets(
				Update,
				(LodGenerateSystems::Produce, LodGenerateSystems::Drain)
					.chain()
					.after(LodNodeSystems::Track),
			);
	}
}

/// Read `F`-filtered [`LodNode`]s. Empty keep is armed from the current lattice
/// disk without a region impulse. Tile-cross / translation-cross still emits
/// newly entered [`LodGenerateRegion<M>`] strips.
pub fn produce_lod_generate_regions<P, F, M>(
	producer: Res<P>,
	nodes: Query<(Entity, Ref<LodNodePose>, Option<&LodNodeBounds>), (With<LodNode>, F)>,
	mut writer: MessageWriter<LodGenerateRegion<M>>,
	mut keep: ResMut<LodGenerateKeepRegion<M>>,
	mut previous: Local<Option<Aabb3d>>,
) where
	P: Resource + LodRefreshRegions,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	let any_changed = nodes.iter().any(|(_, pose, _)| pose.is_changed());
	if keep.region.is_some() && !any_changed {
		return;
	}
	if nodes.is_empty() {
		return;
	}
	let snapshots: Vec<_> = nodes
		.iter()
		.map(|(entity, pose, bounds)| snapshot_node(entity, &pose, bounds))
		.collect();
	let refs = lod_refs_from_snapshots(&snapshots);
	arm_keep_if_empty(&*producer, &refs, &mut keep.region);
	if keep.region.is_some() && previous.is_none() {
		*previous = keep.region;
	}
	if !any_changed {
		return;
	}
	let changed: Vec<_> = nodes
		.iter()
		.filter(|(_, pose, _)| pose.is_changed())
		.map(|(entity, pose, bounds)| snapshot_node(entity, &pose, bounds))
		.collect();
	let changed_refs = lod_refs_from_snapshots(&changed);
	let Ok(LodRefreshRegionsStatus::Changed(region)) =
		producer.lod_refresh_regions_for(&changed_refs)
	else {
		return;
	};
	let baseline = (*previous).or(keep.region);
	keep.region = Some(region);
	for entered in entering_keep_regions(baseline, region) {
		writer.write(LodGenerateRegion::<M>::new(entered));
	}
	*previous = Some(region);
}

/// Enqueue fresh origin ids from generate regions, then materialize a budgeted slice.
pub fn drain_lod_generate<T, S, M, F>(
	mut index: ResMut<S>,
	mut queue: ResMut<LodGenerateQueue<T>>,
	budget: Res<LodGenerateBudget>,
	time_budget: Res<LodGenerateTimeBudget>,
	mut regions: MessageReader<LodGenerateRegion<M>>,
	keep: Res<LodGenerateKeepRegion<M>>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, F)>,
	mut last_keep: Local<Option<Aabb3d>>,
	mut scan_initialized: Local<bool>,
	mut generated: MessageWriter<LodGenerated<T>>,
) where
	T: GenerationScheme<S> + Send + Sync + 'static,
	S: Resource<Mutability = Mutable> + GeneratingSpatialIndex<T>,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	let started = Instant::now();
	let scan_was_reset = queue.take_scan_reset();
	if scan_was_reset {
		*scan_initialized = false;
	}
	if keep_region_changed(*last_keep, keep.region) {
		*last_keep = keep.region;
		queue.expire_outside_keep(keep.region, keep.slack_xz);
	}

	let mut received_region = false;
	for message in regions.read() {
		received_region = true;
		queue.enqueue_scan(message.region);
	}
	if scan_was_reset {
		if let Some(region) = keep.region {
			queue.scan_regions.clear();
			queue.enqueue_scan(region);
			*scan_initialized = true;
		} else if received_region {
			*scan_initialized = true;
		}
	} else if received_region {
		*scan_initialized = true;
	} else if !*scan_initialized {
		if let Some(region) = keep.region {
			queue.enqueue_scan(region);
			*scan_initialized = true;
		}
	}

	let mut scanned = false;
	while !time_up(started, time_budget.time_per_frame) {
		let Some(region) = queue.scan_regions.pop_front() else {
			break;
		};
		let quantum = Instant::now();
		for original in T::original_ids_for(&mut *index, region) {
			if index.storage_status(original.0) != StorageStatus::NotTracked {
				continue;
			}
			if keep
				.region
				.is_some_and(|region| !id_lives_in_keep(original.0, region, keep.slack_xz))
			{
				continue;
			}
			queue.enqueue(original.0);
		}
		warn_atomic_overrun("generate region scan", quantum.elapsed(), time_budget.max_atomic_cost);
		scanned = true;
	}

	if queue.pending.is_empty() {
		return;
	}

	let snapshots = collect_node_snapshots(&nodes);
	let refs = lod_refs_from_snapshots(&snapshots);
	let Some(lod_ref) = refs.first() else {
		return;
	};

	let origin = lod_ref.current_transform.translation;
	if scanned {
		let quantum = Instant::now();
		queue.pending.make_contiguous().sort_by(|a, b| {
			id_xz_distance2(*a, origin)
				.partial_cmp(&id_xz_distance2(*b, origin))
				.unwrap_or(std::cmp::Ordering::Equal)
		});
		warn_atomic_overrun(
			"generate queue ordering",
			quantum.elapsed(),
			time_budget.max_atomic_cost,
		);
	}

	let n = budget.ids_per_frame as usize;
	for _ in 0..n {
		if time_up(started, time_budget.time_per_frame) {
			break;
		}
		let Some(id) = queue.pop_front() else {
			break;
		};
		let quantum = Instant::now();
		if index.get_or_generate(id, lod_ref) == Some(MaterializeStatus::Created) {
			generated.write(LodGenerated::new(id));
		}
		warn_atomic_overrun("generate ID", quantum.elapsed(), time_budget.max_atomic_cost);
	}
}

fn time_up(started: Instant, budget: Duration) -> bool {
	!budget.is_zero() && started.elapsed() >= budget
}

fn warn_atomic_overrun(stage: &'static str, elapsed: Duration, maximum: Duration) {
	if maximum.is_zero() || elapsed <= maximum {
		return;
	}
	warn!(
		stage,
		elapsed_us = elapsed.as_micros(),
		max_us = maximum.as_micros(),
		"LOD generate quantum exceeded max_atomic_cost"
	);
}

fn regions_match(a: Aabb3d, b: Aabb3d) -> bool {
	(a.min.x - b.min.x).abs() < 1e-3
		&& (a.max.x - b.max.x).abs() < 1e-3
		&& (a.min.z - b.min.z).abs() < 1e-3
		&& (a.max.z - b.max.z).abs() < 1e-3
}

fn regions_overlap_xz(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.z <= b.max.z && a.max.z >= b.min.z
}

/// Produce [`LodGenerateRegion<M>`] from `F`-filtered [`LodNode`]s via strategy `P`.
pub struct LodGenerateRegionPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	_marker: PhantomData<fn() -> (P, F, M)>,
}

impl<P, F, M> Default for LodGenerateRegionPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<P, F, M> Plugin for LodGenerateRegionPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_generate_sets(app);
		app.init_resource::<P>()
			.init_resource::<LodGenerateKeepRegion<M>>()
			.add_message::<LodGenerateRegion<M>>()
			.add_systems(
				Update,
				produce_lod_generate_regions::<P, F, M>.in_set(LodGenerateSystems::Produce),
			);
	}
}

/// Budgeted `get_or_generate` for `T` on resource index `S`.
pub struct LodGeneratePlugin<T, S, M, F = ()>
where
	T: GenerationScheme<S> + Send + Sync + 'static,
	S: Resource<Mutability = Mutable> + GeneratingSpatialIndex<T>,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, S, M, F)>,
}

impl<T, S, M, F> Default for LodGeneratePlugin<T, S, M, F>
where
	T: GenerationScheme<S> + Send + Sync + 'static,
	S: Resource<Mutability = Mutable> + GeneratingSpatialIndex<T>,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<T, S, M, F> Plugin for LodGeneratePlugin<T, S, M, F>
where
	T: GenerationScheme<S> + Send + Sync + 'static,
	S: Resource<Mutability = Mutable> + GeneratingSpatialIndex<T>,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_generate_sets(app);
		app.init_resource::<LodGenerateBudget>()
			.init_resource::<LodGenerateTimeBudget>()
			.init_resource::<LodGenerateQueue<T>>()
			.init_resource::<LodGenerateKeepRegion<M>>()
			.add_message::<LodGenerateRegion<M>>()
			.add_message::<LodGenerated<T>>()
			.add_systems(
				Update,
				drain_lod_generate::<T, S, M, F>.in_set(LodGenerateSystems::Drain),
			);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn region(x0: f32, z0: f32, x1: f32, z1: f32) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(x0, -1.0, z0), Vec3::new(x1, 1.0, z1))
	}

	fn xz_area(region: Aabb3d) -> f32 {
		(region.max.x - region.min.x) * (region.max.z - region.min.z)
	}

	#[test]
	fn zero_duration_disables_time_limit() {
		assert!(!time_up(Instant::now(), Duration::ZERO));
	}

	#[test]
	fn elapsed_duration_stops_the_next_quantum() {
		let started = Instant::now() - Duration::from_millis(2);
		assert!(time_up(started, Duration::from_millis(1)));
	}

	#[test]
	fn moving_keep_expires_stale_scan_regions() {
		let mut queue = LodGenerateQueue::<()>::default();
		queue.enqueue_scan(region(0.0, 0.0, 1.0, 1.0));
		queue.enqueue_scan(region(100.0, 0.0, 101.0, 1.0));
		queue.expire_outside_keep(Some(region(0.0, 0.0, 1.0, 1.0)), 0.0);
		assert_eq!(queue.scan_regions.len(), 1);
		assert!(regions_match(queue.scan_regions[0], region(0.0, 0.0, 1.0, 1.0)));
	}

	#[test]
	fn entering_regions_cover_only_moved_strip() {
		let strips =
			entering_keep_regions(Some(region(0.0, 0.0, 10.0, 10.0)), region(2.0, 0.0, 12.0, 10.0));
		assert_eq!(strips.len(), 1);
		assert_eq!(xz_area(strips[0]), 20.0);
	}

	#[test]
	fn entering_regions_cover_expansion_without_overlap() {
		let strips = entering_keep_regions(
			Some(region(0.0, 0.0, 10.0, 10.0)),
			region(-2.0, -2.0, 12.0, 12.0),
		);
		let area: f32 = strips.into_iter().map(xz_area).sum();
		assert_eq!(area, 96.0);
	}
}
