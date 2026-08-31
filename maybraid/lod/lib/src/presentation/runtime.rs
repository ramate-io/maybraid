//! Present-region production, budgeted handle, and hide-then-despawn cull.

use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;

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

/// How many ids to handle per frame. Independent of generate / scene.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LodPresentBudget {
	pub ids_per_frame: u32,
}

impl Default for LodPresentBudget {
	fn default() -> Self {
		Self { ids_per_frame: 1 }
	}
}

/// Shared present allowance with rotating remainder slots across drains.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LodPresentBudgetClock(FairDrainClock);

impl LodPresentBudgetClock {
	fn reset(&mut self, total: u32, drain_count: u32) {
		self.0.reset(total, drain_count);
	}

	fn quota(&self, drain_id: u32) -> u32 {
		self.0.quota(drain_id)
	}

	fn finish_drain(&mut self, consumed: u32) {
		self.0.finish_drain(consumed);
	}
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct FairDrainClock {
	remaining: u32,
	total: u32,
	drain_count: u32,
	extra_start: u32,
	next_extra_start: u32,
}

impl FairDrainClock {
	fn reset(&mut self, total: u32, drain_count: u32) {
		if self.drain_count != drain_count {
			self.next_extra_start = 0;
		}
		self.remaining = total;
		self.total = total;
		self.drain_count = drain_count;
		self.extra_start = self.next_extra_start;
		let extra = if drain_count == 0 { 0 } else { total % drain_count };
		if drain_count > 0 {
			self.next_extra_start = (self.extra_start + extra) % drain_count;
		}
	}

	fn quota(&self, drain_id: u32) -> u32 {
		if self.drain_count == 0 || drain_id >= self.drain_count {
			return 0;
		}
		let base = self.total / self.drain_count;
		let extra = self.total % self.drain_count;
		let offset = (drain_id + self.drain_count - self.extra_start) % self.drain_count;
		base + u32::from(offset < extra)
	}

	fn finish_drain(&mut self, consumed: u32) {
		self.remaining = self.remaining.saturating_sub(consumed);
	}
}

#[derive(Resource, Debug, Clone, Copy, Default)]
struct LodPresentDrainCount(u32);

#[derive(Resource)]
#[doc(hidden)]
pub struct LodPresentDrainId<T, S, Pr, M, F> {
	id: u32,
	_marker: PhantomData<fn() -> (T, S, Pr, M, F)>,
}

/// How many leaving present ids may recursive-despawn per frame.
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

/// Shared cull allowance with rotating remainder slots across drains.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LodPresentCullBudgetClock(FairDrainClock);

impl LodPresentCullBudgetClock {
	fn reset(&mut self, total: u32, drain_count: u32) {
		self.0.reset(total, drain_count);
	}

	fn quota(&self, drain_id: u32) -> u32 {
		self.0.quota(drain_id)
	}

	fn finish_drain(&mut self, consumed: u32) {
		self.0.finish_drain(consumed);
	}
}

#[derive(Resource, Debug, Clone, Copy, Default)]
struct LodPresentCullDrainCount(u32);

#[derive(Resource)]
#[doc(hidden)]
pub struct LodPresentCullDrainId<T, S, Pr, M> {
	id: u32,
	_marker: PhantomData<fn() -> (T, S, Pr, M)>,
}

/// Pending present ids for type `T`.
#[derive(Resource, Debug)]
pub struct LodPresentQueue<T> {
	pending: VecDeque<Id>,
	pending_ids: HashSet<Id>,
	scan_regions: VecDeque<Aabb3d>,
	_marker: PhantomData<T>,
}

impl<T> Default for LodPresentQueue<T> {
	fn default() -> Self {
		Self {
			pending: VecDeque::new(),
			pending_ids: HashSet::new(),
			scan_regions: VecDeque::new(),
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

	fn expire_outside_keep(&mut self, keep: Option<Aabb3d>, slack: f32) {
		let Some(keep) = keep else {
			return;
		};
		self.pending.retain(|id| id_lives_in_keep(*id, keep, slack));
		self.pending_ids.clear();
		self.pending_ids.extend(self.pending.iter().copied());
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
			.init_resource::<LodPresentBudgetClock>()
			.init_resource::<LodPresentDrainCount>()
			.init_resource::<LodPresentCullBudget>()
			.init_resource::<LodPresentCullBudgetClock>()
			.init_resource::<LodPresentCullDrainCount>()
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
			)
			.add_systems(Update, reset_lod_present_budgets.in_set(LodPresentSystems::Produce));
	}
}

fn reset_lod_present_budgets(
	present_budget: Res<LodPresentBudget>,
	present_drains: Res<LodPresentDrainCount>,
	cull_budget: Res<LodPresentCullBudget>,
	cull_drains: Res<LodPresentCullDrainCount>,
	mut present_clock: ResMut<LodPresentBudgetClock>,
	mut cull_clock: ResMut<LodPresentCullBudgetClock>,
) {
	present_clock.reset(present_budget.ids_per_frame, present_drains.0);
	cull_clock.reset(cull_budget.despawns_per_frame, cull_drains.0);
}

/// Read pose-changed drivers, emit [`LodPresentRegion<M>`], record keep AABB.
pub fn produce_lod_present_regions<P, F, M>(
	producer: Res<P>,
	nodes: Query<
		(Entity, &LodNodePose, Option<&LodNodeBounds>),
		(With<LodNode>, Changed<LodNodePose>, F),
	>,
	mut writer: MessageWriter<LodPresentRegion<M>>,
	mut keep: ResMut<LodPresentKeepRegion<M>>,
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
	keep.region = Some(region);
	writer.write(LodPresentRegion::<M>::new(region));
}

/// Enqueue tracked ids that need handle, then present a budgeted slice.
pub fn drain_lod_present<T, S, Pr, M, F>(
	presenter: StaticSystemParam<Pr>,
	index: Res<S>,
	mut queue: ResMut<LodPresentQueue<T>>,
	mut budget: ResMut<LodPresentBudgetClock>,
	drain_id: Res<LodPresentDrainId<T, S, Pr, M, F>>,
	mut regions: MessageReader<LodPresentRegion<M>>,
	mut generated: MessageReader<LodGenerated<T>>,
	keep: Res<LodPresentKeepRegion<M>>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, F)>,
	mut last_keep: Local<Option<Aabb3d>>,
) where
	T: Send + Sync + 'static,
	S: Resource + SpatialIndex<T>,
	Pr: SystemParam + 'static,
	for<'w, 's> Pr::Item<'w, 's>: RegionPresenter<T, S>,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	let mut presenter = presenter.into_inner();
	let mut scan: Vec<Aabb3d> = regions.read().map(|message| message.region).collect();
	if keep_region_changed(*last_keep, keep.region) {
		let previous = *last_keep;
		*last_keep = keep.region;
		queue.expire_outside_keep(keep.region, keep.slack_xz);
		push_incremental_keep_regions(&mut scan, previous, keep.region);
	}
	for region in scan {
		queue.enqueue_scan(region);
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

	let quota = budget.quota(drain_id.id);
	if quota == 0 {
		budget.finish_drain(0);
		return;
	}

	if let Some(region) = queue.scan_regions.pop_front() {
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
	}

	if queue.pending.is_empty() {
		budget.finish_drain(0);
		return;
	}

	let snapshots = collect_node_snapshots(&nodes);
	let refs = lod_refs_from_snapshots(&snapshots);
	let Some(lod_ref) = refs.first() else {
		budget.finish_drain(0);
		return;
	};

	let origin = lod_ref.current_transform.translation;
	if reorder_pending {
		queue.pending.make_contiguous().sort_by(|a, b| {
			id_xz_distance2(*a, origin)
				.partial_cmp(&id_xz_distance2(*b, origin))
				.unwrap_or(std::cmp::Ordering::Equal)
		});
	}

	let n = quota as usize;
	let mut handled = 0;
	while handled < n {
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
		presenter.handle(id, version, value, lod_ref);
		// Grow-then-spawn presenters may consume a slot without stamping
		// `presented_version`. Re-queue so the next slot can finish without a
		// keep rescan.
		let still_needs =
			presenter.presented_version(id).is_none_or(|presented| presented < version);
		if still_needs {
			queue.enqueue_front(id);
		}
	}
	budget.finish_drain(handled as u32);
}

fn push_incremental_keep_regions(
	scan: &mut Vec<Aabb3d>,
	previous: Option<Aabb3d>,
	keep: Option<Aabb3d>,
) {
	let Some(keep_region) = keep else {
		return;
	};
	if let Some(index) = scan.iter().position(|region| regions_match(*region, keep_region)) {
		scan.remove(index);
	}
	for region in entering_keep_regions(previous, keep_region) {
		if !scan.iter().any(|candidate| regions_match(*candidate, region)) {
			scan.push(region);
		}
	}
}

fn regions_match(a: Aabb3d, b: Aabb3d) -> bool {
	(a.min.x - b.min.x).abs() < 1e-3
		&& (a.max.x - b.max.x).abs() < 1e-3
		&& (a.min.z - b.min.z).abs() < 1e-3
		&& (a.max.z - b.max.z).abs() < 1e-3
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
	mut budget: ResMut<LodPresentCullBudgetClock>,
	drain_id: Res<LodPresentCullDrainId<T, S, Pr, M>>,
) where
	T: Send + Sync + 'static,
	S: Resource + SpatialIndex<T>,
	Pr: SystemParam + 'static,
	for<'w, 's> Pr::Item<'w, 's>: RegionPresenter<T, S>,
	M: Send + Sync + 'static,
{
	let Some(keep_region) = keep.live_region() else {
		budget.finish_drain(0);
		return;
	};
	let keep_ids: HashSet<Id> = index
		.tracked_ids_for(keep_region)
		.into_iter()
		.map(|tracked| tracked.0)
		.collect();
	let mut presenter = presenter.into_inner();
	let share = budget.quota(drain_id.id);
	let stale = presenter
		.presented_ids()
		.into_iter()
		.filter(|id| !keep_ids.contains(id))
		.count() as u32;
	presenter.cull(&*index, &keep_ids, share);
	budget.finish_drain(share.min(stale));
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
		let id = {
			let mut drains = app.world_mut().resource_mut::<LodPresentDrainCount>();
			let id = drains.0;
			drains.0 += 1;
			id
		};
		app.init_resource::<LodPresentBudget>()
			.init_resource::<LodPresentQueue<T>>()
			.init_resource::<LodPresentKeepRegion<M>>()
			.insert_resource(LodPresentDrainId::<T, S, Pr, M, F> { id, _marker: PhantomData })
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
		let id = {
			let mut drains = app.world_mut().resource_mut::<LodPresentCullDrainCount>();
			let id = drains.0;
			drains.0 += 1;
			id
		};
		app.init_resource::<LodPresentCullBudget>()
			.init_resource::<LodPresentKeepRegion<M>>()
			.insert_resource(LodPresentCullDrainId::<T, S, Pr, M> { id, _marker: PhantomData })
			.add_systems(
				Update,
				drain_lod_present_cull::<T, S, Pr, M>.in_set(LodPresentSystems::Cull),
			);
	}
}
