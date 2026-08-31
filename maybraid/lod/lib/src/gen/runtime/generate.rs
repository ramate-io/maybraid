//! Generate-region production and budgeted index insert.

use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;

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
	collect_node_snapshots, lod_refs_from_snapshots, LodNode, LodNodeBounds, LodNodePlugin,
	LodNodePose, LodNodeSystems,
};
use crate::scene::{LodRefreshRegions, LodRefreshRegionsStatus};

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

/// How many origin ids to materialize per frame. Independent of scene / present.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LodGenerateBudget {
	pub ids_per_frame: u32,
}

impl Default for LodGenerateBudget {
	fn default() -> Self {
		Self { ids_per_frame: 1 }
	}
}

/// Shared generate allowance split fairly across all registered drains.
///
/// Remainder slots rotate between drain identities each frame, so a budget
/// smaller than the number of types cannot permanently starve later systems.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LodGenerateBudgetClock {
	remaining: u32,
	total: u32,
	drain_count: u32,
	extra_start: u32,
	next_extra_start: u32,
}

impl LodGenerateBudgetClock {
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
struct LodGenerateDrainCount(u32);

#[derive(Resource)]
#[doc(hidden)]
pub struct LodGenerateDrainId<T, S, M, F> {
	id: u32,
	_marker: PhantomData<fn() -> (T, S, M, F)>,
}

/// Pending origin ids for type `T`.
#[derive(Resource, Debug)]
pub struct LodGenerateQueue<T> {
	pending: VecDeque<Id>,
	pending_ids: HashSet<Id>,
	scan_regions: VecDeque<Aabb3d>,
	_marker: PhantomData<T>,
}

impl<T> Default for LodGenerateQueue<T> {
	fn default() -> Self {
		Self {
			pending: VecDeque::new(),
			pending_ids: HashSet::new(),
			scan_regions: VecDeque::new(),
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

	fn expire_outside_keep(&mut self, keep: Option<Aabb3d>, slack: f32) {
		let Some(keep) = keep else {
			return;
		};
		self.pending.retain(|id| id_lives_in_keep(*id, keep, slack));
		self.pending_ids.clear();
		self.pending_ids.extend(self.pending.iter().copied());
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
			.init_resource::<LodGenerateBudgetClock>()
			.init_resource::<LodGenerateDrainCount>()
			.configure_sets(
				Update,
				(LodGenerateSystems::Produce, LodGenerateSystems::Drain)
					.chain()
					.after(LodNodeSystems::Track),
			)
			.add_systems(Update, reset_lod_generate_budget.in_set(LodGenerateSystems::Produce));
	}
}

fn reset_lod_generate_budget(
	budget: Res<LodGenerateBudget>,
	drains: Res<LodGenerateDrainCount>,
	mut clock: ResMut<LodGenerateBudgetClock>,
) {
	clock.reset(budget.ids_per_frame, drains.0);
}

/// Read `F`-filtered [`LodNode`]s whose pose changed, emit [`LodGenerateRegion<M>`].
pub fn produce_lod_generate_regions<P, F, M>(
	producer: Res<P>,
	nodes: Query<
		(Entity, &LodNodePose, Option<&LodNodeBounds>),
		(With<LodNode>, Changed<LodNodePose>, F),
	>,
	mut writer: MessageWriter<LodGenerateRegion<M>>,
	mut keep: ResMut<LodGenerateKeepRegion<M>>,
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
	writer.write(LodGenerateRegion::<M>::new(region));
}

/// Enqueue fresh origin ids from generate regions, then materialize a budgeted slice.
pub fn drain_lod_generate<T, S, M, F>(
	mut index: ResMut<S>,
	mut queue: ResMut<LodGenerateQueue<T>>,
	mut budget: ResMut<LodGenerateBudgetClock>,
	drain_id: Res<LodGenerateDrainId<T, S, M, F>>,
	mut regions: MessageReader<LodGenerateRegion<M>>,
	keep: Res<LodGenerateKeepRegion<M>>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, F)>,
	mut last_keep: Local<Option<Aabb3d>>,
	mut generated: MessageWriter<LodGenerated<T>>,
) where
	T: GenerationScheme<S> + Send + Sync + 'static,
	S: Resource<Mutability = Mutable> + GeneratingSpatialIndex<T>,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	let mut scan: Vec<Aabb3d> = regions.read().map(|message| message.region).collect();
	let keep_changed = keep_region_changed(*last_keep, keep.region);
	if keep_changed {
		let previous = *last_keep;
		*last_keep = keep.region;
		queue.expire_outside_keep(keep.region, keep.slack_xz);
		push_incremental_keep_regions(&mut scan, previous, keep.region);
	}
	for region in scan {
		queue.enqueue_scan(region);
	}

	let quota = budget.quota(drain_id.id);
	if quota == 0 {
		budget.finish_drain(0);
		return;
	}

	let scanned = queue.scan_regions.pop_front().is_some_and(|region| {
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
		true
	});

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
	if scanned {
		queue.pending.make_contiguous().sort_by(|a, b| {
			id_xz_distance2(*a, origin)
				.partial_cmp(&id_xz_distance2(*b, origin))
				.unwrap_or(std::cmp::Ordering::Equal)
		});
	}

	let n = quota as usize;
	let mut consumed = 0u32;
	for _ in 0..n {
		let Some(id) = queue.pop_front() else {
			break;
		};
		consumed = consumed.saturating_add(1);
		if index.get_or_generate(id, lod_ref) == Some(MaterializeStatus::Created) {
			generated.write(LodGenerated::new(id));
		}
	}
	budget.finish_drain(consumed);
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
	} else if previous.is_some() {
		// An explicit message not matching the keep region is additional work.
		// The keep delta below still covers camera movement.
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
		let id = {
			let mut drains = app.world_mut().resource_mut::<LodGenerateDrainCount>();
			let id = drains.0;
			drains.0 += 1;
			id
		};
		app.init_resource::<LodGenerateBudget>()
			.init_resource::<LodGenerateQueue<T>>()
			.init_resource::<LodGenerateKeepRegion<M>>()
			.insert_resource(LodGenerateDrainId::<T, S, M, F> { id, _marker: PhantomData })
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
	fn shared_clock_distributes_one_frame_cap() {
		let mut clock = LodGenerateBudgetClock::default();
		clock.reset(5, 2);
		assert_eq!(clock.quota(0), 3);
		assert_eq!(clock.quota(1), 2);
		clock.finish_drain(5);
		assert_eq!(clock.remaining, 0);
	}

	#[test]
	fn remainder_slot_rotates_across_types() {
		let mut clock = LodGenerateBudgetClock::default();
		for expected in 0..3 {
			clock.reset(1, 3);
			for id in 0..3 {
				assert_eq!(clock.quota(id), u32::from(id == expected));
			}
		}
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
