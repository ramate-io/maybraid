//! Present-region production, budgeted handle, and hide-then-despawn cull.

use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use super::RegionPresenter;
use crate::gen::{
	entering_keep_regions, expand_keep_xz, id_lives_in_keep, id_xz_distance2, keep_region_changed,
	Id, LodGenerated, SpatialIndex, QUEUE_KEEP_SLACK_XZ,
};
use crate::lod_ref::{
	collect_node_snapshots, lod_refs_from_snapshots, LodNode, LodNodeBounds, LodNodePlugin,
	LodNodePose, LodNodeSystems,
};
use crate::scene::{
	LodCullRegionCursor, LodCullRegions, LodCullRegionsStatus, LodRefreshRegions,
	LodRefreshRegionsStatus,
};

/// Impulse: present ids tracked in `region` (channel `M`).
#[derive(Message, Debug, Clone)]
pub struct LodPresentRegion<M: Send + Sync + 'static> {
	pub region: Aabb3d,
	pub _marker: PhantomData<M>,
}

impl<M: Send + Sync + 'static> LodPresentRegion<M> {
	pub fn new(region: Aabb3d) -> Self {
		Self { region, _marker: PhantomData }
	}
}

/// Impulse: optional lattice tile for present cull. Drain does not read
/// these; hide / despawn is keep-set membership.
#[derive(Message, Debug, Clone)]
pub struct LodPresentCullRegion<M: Send + Sync + 'static> {
	pub region: Aabb3d,
	pub _marker: PhantomData<M>,
}

impl<M: Send + Sync + 'static> LodPresentCullRegion<M> {
	pub fn new(region: Aabb3d) -> Self {
		Self { region, _marker: PhantomData }
	}
}

/// How many ids each present drain may handle per frame.
///
/// Independent of generation and scene fulfillment.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LodPresentBudget {
	pub ids_per_frame: u32,
}

impl Default for LodPresentBudget {
	fn default() -> Self {
		Self { ids_per_frame: 1 }
	}
}

/// Independent wall-clock guard applied by each present drain.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LodPresentTimeBudget {
	/// Per-drain wall time. Zero disables the time limit.
	pub time_per_frame: Duration,
	/// Warn when one non-interruptible region/ID quantum exceeds this duration.
	/// Zero disables warnings.
	pub max_atomic_cost: Duration,
}

impl Default for LodPresentTimeBudget {
	fn default() -> Self {
		Self { time_per_frame: Duration::from_millis(2), max_atomic_cost: Duration::from_millis(3) }
	}
}

/// How many leaving present ids each cull drain may recursive-despawn per frame.
///
/// Hide is uncapped (cheap `Visibility::Hidden`). Despawn is one grove id
/// (all of its host entities) per slot — the parent hitch.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LodPresentCullBudget {
	pub despawns_per_frame: u32,
}

impl Default for LodPresentCullBudget {
	fn default() -> Self {
		Self { despawns_per_frame: 1 }
	}
}

/// Pending present ids for type `T`.
#[derive(Resource, Debug)]
pub struct LodPresentQueue<T> {
	pending: VecDeque<Id>,
	pending_ids: HashSet<Id>,
	scan_regions: VecDeque<Aabb3d>,
	reset_scan: bool,
	_marker: PhantomData<T>,
}

impl<T> Default for LodPresentQueue<T> {
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

impl<T> LodPresentQueue<T> {
	pub fn is_empty(&self) -> bool {
		self.pending.is_empty()
	}

	pub fn contains(&self, id: &Id) -> bool {
		self.pending_ids.contains(id)
	}

	pub fn enqueue(&mut self, id: Id) -> bool {
		self.enqueue_back(id)
	}

	pub fn clear(&mut self) {
		self.pending.clear();
		self.pending_ids.clear();
		self.scan_regions.clear();
		self.reset_scan = true;
	}

	fn enqueue_back(&mut self, id: Id) -> bool {
		if !self.pending_ids.insert(id) {
			return false;
		}
		self.pending.push_back(id);
		true
	}

	fn enqueue_front(&mut self, id: Id) -> bool {
		if !self.pending_ids.insert(id) {
			return false;
		}
		self.pending.push_front(id);
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

/// Last present-ring AABB for this channel (cull `keep` set).
///
/// [`Self::slack_xz`] is the live margin around [`Self::region`] (queue expire
/// and present-cull keep ids).
#[derive(Resource, Debug)]
pub struct LodPresentKeepRegion<M: Send + Sync + 'static> {
	pub region: Option<Aabb3d>,
	/// XZ expand of [`Self::region`] for pending-id expiry and cull keep.
	pub slack_xz: f32,
	_marker: PhantomData<M>,
}

impl<M: Send + Sync + 'static> Default for LodPresentKeepRegion<M> {
	fn default() -> Self {
		Self { region: None, slack_xz: QUEUE_KEEP_SLACK_XZ, _marker: PhantomData }
	}
}

impl<M: Send + Sync + 'static> LodPresentKeepRegion<M> {
	/// Keep AABB expanded by [`Self::slack_xz`] on XZ. `None` before the first produce.
	pub fn live_region(&self) -> Option<Aabb3d> {
		self.region.map(|region| expand_keep_xz(region, self.slack_xz))
	}
}

/// Present-layer cull cursor (not the scene [`LodCullRegionCursor`] resource).
#[derive(Resource, Debug, Clone, Default)]
pub struct LodPresentCullCursor(pub LodCullRegionCursor);

/// Ordering inside the present layer (not the scene stack).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodPresentSystems {
	Produce,
	Drain,
	ProduceCull,
	Cull,
}

pub(super) fn ensure_present_sets(app: &mut App) {
	if app.is_plugin_added::<LodPresentSetsPlugin>() {
		return;
	}
	app.add_plugins(LodPresentSetsPlugin);
}

struct LodPresentSetsPlugin;

impl Plugin for LodPresentSetsPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<LodNodePlugin>() {
			app.add_plugins(LodNodePlugin);
		}
		app.init_resource::<LodPresentBudget>()
			.init_resource::<LodPresentTimeBudget>()
			.init_resource::<LodPresentCullBudget>()
			.configure_sets(
				Update,
				(
					LodPresentSystems::Produce,
					LodPresentSystems::Drain,
					LodPresentSystems::ProduceCull,
					LodPresentSystems::Cull,
				)
					.chain()
					.after(LodNodeSystems::Track),
			);
	}
}

/// Read pose-changed drivers, emit newly entered [`LodPresentRegion<M>`]
/// strips, and record the full keep AABB.
pub fn produce_lod_present_regions<P, F, M>(
	producer: Res<P>,
	nodes: Query<
		(Entity, &LodNodePose, Option<&LodNodeBounds>),
		(With<LodNode>, Changed<LodNodePose>, F),
	>,
	mut writer: MessageWriter<LodPresentRegion<M>>,
	mut keep: ResMut<LodPresentKeepRegion<M>>,
	mut previous: Local<Option<Aabb3d>>,
) where
	P: Resource + LodRefreshRegions,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	if nodes.is_empty() {
		return;
	}
	let snapshots = collect_node_snapshots(&nodes);
	let refs = lod_refs_from_snapshots(&snapshots);
	let Ok(LodRefreshRegionsStatus::Changed(region)) = producer.lod_refresh_regions_for(&refs)
	else {
		return;
	};
	let baseline = (*previous).or(keep.region);
	keep.region = Some(region);
	for entered in entering_keep_regions(baseline, region) {
		writer.write(LodPresentRegion::<M>::new(entered));
	}
	*previous = Some(region);
}

/// Enqueue tracked ids that need handle, then present a budgeted slice.
pub fn drain_lod_present<T, S, Pr, M, F>(
	presenter: StaticSystemParam<Pr>,
	index: Res<S>,
	mut queue: ResMut<LodPresentQueue<T>>,
	budget: Res<LodPresentBudget>,
	time_budget: Res<LodPresentTimeBudget>,
	mut regions: MessageReader<LodPresentRegion<M>>,
	mut generated: MessageReader<LodGenerated<T>>,
	keep: Res<LodPresentKeepRegion<M>>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, F)>,
	mut last_keep: Local<Option<Aabb3d>>,
	mut scan_initialized: Local<bool>,
) where
	T: Send + Sync + 'static,
	S: Resource + SpatialIndex<T>,
	Pr: SystemParam + 'static,
	for<'w, 's> Pr::Item<'w, 's>: RegionPresenter<T, S>,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	let started = Instant::now();
	let mut presenter = presenter.into_inner();
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

	let mut reorder_pending = false;
	for message in generated.read() {
		if keep
			.region
			.is_some_and(|region| !id_lives_in_keep(message.id, region, keep.slack_xz))
		{
			continue;
		}
		if queue.enqueue_back(message.id) {
			reorder_pending = true;
		}
	}

	while !time_up(started, time_budget.time_per_frame) {
		let Some(region) = queue.scan_regions.pop_front() else {
			break;
		};
		let quantum = Instant::now();
		for tracked in index.tracked_ids_for(region) {
			let Some(version) = index.version(tracked.0) else {
				continue;
			};
			let needs = presenter
				.presented_version(tracked.0)
				.is_none_or(|presented| presented < version);
			if !needs
				&& !presenter.needs_repair(
					Aabb3d::from_min_max(Vec3::ZERO, Vec3::ZERO),
					tracked.0,
					version,
				) {
				continue;
			}
			if queue.enqueue_back(tracked.0) {
				reorder_pending = true;
			}
		}
		warn_atomic_overrun("present region scan", quantum.elapsed(), time_budget.max_atomic_cost);
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
	if reorder_pending {
		let quantum = Instant::now();
		queue.pending.make_contiguous().sort_by(|a, b| {
			id_xz_distance2(*a, origin)
				.partial_cmp(&id_xz_distance2(*b, origin))
				.unwrap_or(std::cmp::Ordering::Equal)
		});
		warn_atomic_overrun(
			"present queue ordering",
			quantum.elapsed(),
			time_budget.max_atomic_cost,
		);
	}

	let n = budget.ids_per_frame as usize;
	let mut handled = 0;
	while handled < n && !time_up(started, time_budget.time_per_frame) {
		let Some(id) = queue.pop_front() else {
			break;
		};
		handled += 1;
		let Some(version) = index.version(id) else {
			continue;
		};
		let needs = presenter.presented_version(id).is_none_or(|presented| presented < version);
		if !needs
			&& !presenter.needs_repair(Aabb3d::from_min_max(Vec3::ZERO, Vec3::ZERO), id, version)
		{
			continue;
		}
		let Some(value) = index.get(id) else {
			continue;
		};
		let quantum = Instant::now();
		presenter.handle(id, version, value, lod_ref);
		warn_atomic_overrun("present ID", quantum.elapsed(), time_budget.max_atomic_cost);
		// Grow-then-spawn presenters may consume a slot without stamping
		// `presented_version`. Re-queue so the next slot can finish without a
		// keep rescan.
		let still_needs =
			presenter.presented_version(id).is_none_or(|presented| presented < version);
		if still_needs {
			queue.enqueue_front(id);
		}
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
		"LOD presentation quantum exceeded max_atomic_cost"
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

/// Emit optional [`LodPresentCullRegion<M>`] tiles via strategy `P`. Drain ignores them.
pub fn produce_lod_present_cull_regions<P, F, M>(
	producer: Res<P>,
	mut cursor: ResMut<LodPresentCullCursor>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, F)>,
	mut writer: MessageWriter<LodPresentCullRegion<M>>,
) where
	P: Resource + LodCullRegions,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	if nodes.is_empty() {
		return;
	}
	if producer.is_changed() {
		cursor.0.invalidate_cells();
	}
	let snapshots = collect_node_snapshots(&nodes);
	let refs = lod_refs_from_snapshots(&snapshots);
	let LodCullRegionsStatus::Changed(regions) = producer.lod_cull_regions(&refs, &mut cursor.0)
	else {
		return;
	};
	for region in regions {
		writer.write(LodPresentCullRegion::<M>::new(region));
	}
}

/// Hide, then budget-despawn, presented ids outside the keep ring.
///
/// Runs whenever keep is live. Does not wait for lattice tiles.
pub fn drain_lod_present_cull<T, S, Pr, M>(
	presenter: StaticSystemParam<Pr>,
	index: Res<S>,
	keep: Res<LodPresentKeepRegion<M>>,
	budget: Res<LodPresentCullBudget>,
) where
	T: Send + Sync + 'static,
	S: Resource + SpatialIndex<T>,
	Pr: SystemParam + 'static,
	for<'w, 's> Pr::Item<'w, 's>: RegionPresenter<T, S>,
	M: Send + Sync + 'static,
{
	let Some(keep_region) = keep.live_region() else {
		return;
	};
	let keep_ids: HashSet<Id> = index
		.tracked_ids_for(keep_region)
		.into_iter()
		.map(|tracked| tracked.0)
		.collect();
	let mut presenter = presenter.into_inner();
	presenter.cull(&*index, &keep_ids, budget.despawns_per_frame);
}

/// Produce [`LodPresentRegion<M>`] from `F`-filtered [`LodNode`]s via strategy `P`.
pub struct LodPresentRegionPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	_marker: PhantomData<fn() -> (P, F, M)>,
}

impl<P, F, M> Default for LodPresentRegionPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<P, F, M> Plugin for LodPresentRegionPlugin<P, F, M>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_present_sets(app);
		app.init_resource::<P>()
			.init_resource::<LodPresentKeepRegion<M>>()
			.add_message::<LodPresentRegion<M>>()
			.add_systems(
				Update,
				produce_lod_present_regions::<P, F, M>.in_set(LodPresentSystems::Produce),
			);
	}
}

/// Budgeted present drain for `T` via presenter `Pr`.
pub struct LodPresentPlugin<T, S, Pr, M, F = ()>
where
	T: Send + Sync + 'static,
	S: Resource + SpatialIndex<T>,
	Pr: SystemParam + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, S, Pr, M, F)>,
}

impl<T, S, Pr, M, F> Default for LodPresentPlugin<T, S, Pr, M, F>
where
	T: Send + Sync + 'static,
	S: Resource + SpatialIndex<T>,
	Pr: SystemParam + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<T, S, Pr, M, F> Plugin for LodPresentPlugin<T, S, Pr, M, F>
where
	T: Send + Sync + 'static,
	S: Resource + SpatialIndex<T>,
	Pr: SystemParam + 'static,
	for<'w, 's> Pr::Item<'w, 's>: RegionPresenter<T, S>,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_present_sets(app);
		app.init_resource::<LodPresentBudget>()
			.init_resource::<LodPresentTimeBudget>()
			.init_resource::<LodPresentQueue<T>>()
			.init_resource::<LodPresentKeepRegion<M>>()
			.add_message::<LodPresentRegion<M>>()
			.add_message::<LodGenerated<T>>()
			.add_systems(
				Update,
				drain_lod_present::<T, S, Pr, M, F>.in_set(LodPresentSystems::Drain),
			);
	}
}

/// Optional lattice tiles. [`drain_lod_present_cull`] does not require them.
pub struct LodPresentCullRegionPlugin<P, F, M>
where
	P: Resource + LodCullRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	_marker: PhantomData<fn() -> (P, F, M)>,
}

impl<P, F, M> Default for LodPresentCullRegionPlugin<P, F, M>
where
	P: Resource + LodCullRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<P, F, M> Plugin for LodPresentCullRegionPlugin<P, F, M>
where
	P: Resource + LodCullRegions + Default,
	F: QueryFilter + 'static,
	M: Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_present_sets(app);
		app.init_resource::<P>()
			.init_resource::<LodPresentCullCursor>()
			.add_message::<LodPresentCullRegion<M>>()
			.add_systems(
				Update,
				produce_lod_present_cull_regions::<P, F, M>.in_set(LodPresentSystems::ProduceCull),
			);
	}
}

/// Cull drain for `T` via presenter `Pr`.
pub struct LodPresentCullPlugin<T, S, Pr, M>
where
	T: Send + Sync + 'static,
	S: Resource + SpatialIndex<T>,
	Pr: SystemParam + 'static,
	M: Send + Sync + 'static,
{
	_marker: PhantomData<fn() -> (T, S, Pr, M)>,
}

impl<T, S, Pr, M> Default for LodPresentCullPlugin<T, S, Pr, M>
where
	T: Send + Sync + 'static,
	S: Resource + SpatialIndex<T>,
	Pr: SystemParam + 'static,
	M: Send + Sync + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<T, S, Pr, M> Plugin for LodPresentCullPlugin<T, S, Pr, M>
where
	T: Send + Sync + 'static,
	S: Resource + SpatialIndex<T>,
	Pr: SystemParam + 'static,
	for<'w, 's> Pr::Item<'w, 's>: RegionPresenter<T, S>,
	M: Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_present_sets(app);
		app.init_resource::<LodPresentCullBudget>()
			.init_resource::<LodPresentKeepRegion<M>>()
			.add_systems(
				Update,
				drain_lod_present_cull::<T, S, Pr, M>.in_set(LodPresentSystems::Cull),
			);
	}
}
