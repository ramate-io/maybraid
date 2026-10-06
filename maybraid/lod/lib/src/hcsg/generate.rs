//! Producer channels and generate subscribers over [`HcsgStorage`].
//!
//! A producer `P` is a type that publishes [`GenerationBounds`] on its own
//! channel ([`GenerationRequest<P>`]). [`GenerateOn<P, T>`] subscribes `T` to
//! that channel and materializes `T`'s origins inside the requested bounds.
//! Several types can subscribe to one producer, and one type can subscribe to
//! several producers; storage builds each id once.

use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;
use std::time::{Duration, Instant};

use bevy::ecs::query::QueryFilter;
use bevy::ecs::schedule::{InternedSystemSet, SystemSet};
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::gen::{
	ensure_generate_sets, entering_keep_regions, expand_keep_xz, id_lives_in_keep, id_xz_distance2,
	keep_region_changed, GenerationScheme, Id, LodGenerateBudget, LodGenerateSystems,
	LodGenerateTimeBudget, LodGenerated, MaterializeStatus, OriginalId, SpatialIndex,
	StorageStatus, QUEUE_KEEP_SLACK_XZ,
};
use crate::hcsg::storage::{HcsgNode, HcsgStorage};
use crate::jobs::LodJobCounter;
use crate::lod_ref::{
	collect_node_snapshots, lod_refs_from_snapshots, LodNode, LodNodeBounds, LodNodePose,
};
use crate::scene::{LodRefreshRegions, LodRefreshRegionsStatus};

impl HcsgStorage {
	/// Ids of `T` originating in `region`, per `T`'s scheme.
	pub fn original_ids_for<T>(&mut self, region: Aabb3d) -> Vec<OriginalId>
	where
		T: GenerationScheme<Self> + HcsgNode,
	{
		crate::gen::GeneratingSpatialIndex::<T>::original_ids_for(self, region)
	}

	pub fn get_or_generate<T>(&mut self, id: Id) -> Option<MaterializeStatus>
	where
		T: GenerationScheme<Self> + HcsgNode,
	{
		crate::gen::GeneratingSpatialIndex::<T>::get_or_generate(self, id)
	}

	pub fn get_one_or_generate<T>(&mut self, id: Id) -> Option<&T>
	where
		T: GenerationScheme<Self> + HcsgNode,
	{
		crate::gen::GeneratingSpatialIndex::<T>::get_one_or_generate(self, id)
	}
}

/// One set of requested bounds. The same shape on every channel.
#[derive(Debug, Clone, PartialEq)]
pub struct GenerationBounds {
	/// Region the channel wants materialized. Queued ids outside it (plus
	/// [`CurrentBounds::slack_xz`]) expire.
	pub keep: Aabb3d,
	/// Parts of `keep` the previous bounds did not cover. Only these are scanned.
	pub entering: Vec<Aabb3d>,
	/// Ordering hint: nearer origins generate first. Never changes what is built.
	pub priority_xz: Option<Vec2>,
	/// The origins inside `keep` changed wholesale (for example a layout that
	/// tiles differently around a new center). Subscribers drop pending ids
	/// and rescan all of `keep`.
	pub restart: bool,
}

impl GenerationBounds {
	/// Bounds for `keep`, scanning only what `previous` did not cover.
	pub fn after(previous: Option<Aabb3d>, keep: Aabb3d, priority_xz: Option<Vec2>) -> Self {
		Self { keep, entering: entering_keep_regions(previous, keep), priority_xz, restart: false }
	}

	/// Bounds that rescan all of `keep`, discarding pending work.
	pub fn restart(keep: Aabb3d, priority_xz: Option<Vec2>) -> Self {
		Self { keep, entering: vec![keep], priority_xz, restart: true }
	}
}

/// Impulse on producer `P`'s channel.
#[derive(Message, Debug, Clone)]
pub struct GenerationRequest<P: 'static> {
	pub bounds: GenerationBounds,
	_channel: PhantomData<fn() -> P>,
}

impl<P: 'static> GenerationRequest<P> {
	pub fn new(bounds: GenerationBounds) -> Self {
		Self { bounds, _channel: PhantomData }
	}
}

/// Latest bounds on `P`'s channel, for subscribers that start or reset late.
#[derive(Resource, Debug)]
pub struct CurrentBounds<P: 'static> {
	pub bounds: Option<GenerationBounds>,
	/// XZ margin around `keep` before queued ids expire. World-overridable.
	pub slack_xz: f32,
	_channel: PhantomData<fn() -> P>,
}

impl<P: 'static> Default for CurrentBounds<P> {
	fn default() -> Self {
		Self { bounds: None, slack_xz: QUEUE_KEEP_SLACK_XZ, _channel: PhantomData }
	}
}

impl<P: 'static> CurrentBounds<P> {
	pub fn keep(&self) -> Option<Aabb3d> {
		self.bounds.as_ref().map(|bounds| bounds.keep)
	}

	/// `keep` expanded by [`Self::slack_xz`]. `None` before the first publish.
	pub fn live_region(&self) -> Option<Aabb3d> {
		self.keep().map(|keep| expand_keep_xz(keep, self.slack_xz))
	}
}

/// Write side of producer `P`'s channel.
#[derive(SystemParam)]
pub struct GenerationProducer<'w, P: Send + Sync + 'static> {
	current: ResMut<'w, CurrentBounds<P>>,
	requests: MessageWriter<'w, GenerationRequest<P>>,
}

impl<P: Send + Sync + 'static> GenerationProducer<'_, P> {
	/// Publishes `keep` if it moved. Subscribers scan only the entering strips.
	pub fn publish(&mut self, keep: Aabb3d, priority_xz: Option<Vec2>) -> bool {
		let previous = self.current.keep();
		if !keep_region_changed(previous, Some(keep)) {
			if let Some(bounds) = self.current.bounds.as_mut() {
				bounds.priority_xz = priority_xz;
			}
			return false;
		}
		self.send(GenerationBounds::after(previous, keep, priority_xz));
		true
	}

	/// Publishes `keep` unconditionally and makes subscribers rescan all of it.
	pub fn restart(&mut self, keep: Aabb3d, priority_xz: Option<Vec2>) {
		self.send(GenerationBounds::restart(keep, priority_xz));
	}

	fn send(&mut self, bounds: GenerationBounds) {
		self.current.bounds = Some(bounds.clone());
		self.requests.write(GenerationRequest::new(bounds));
	}

	pub fn current(&self) -> Option<&GenerationBounds> {
		self.current.bounds.as_ref()
	}
}

/// Registers producer `P`'s channel. Producers write it through [`GenerationProducer`].
pub struct GenerationChannelPlugin<P>(PhantomData<fn() -> P>);

impl<P> Default for GenerationChannelPlugin<P> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<P: Send + Sync + 'static> Plugin for GenerationChannelPlugin<P> {
	fn build(&self, app: &mut App) {
		ensure_generate_sets(app);
		app.init_resource::<CurrentBounds<P>>().add_message::<GenerationRequest<P>>();
	}

	fn is_unique(&self) -> bool {
		false
	}
}

/// Producer `P` driven by `F`-filtered [`LodNode`] poses through its
/// [`LodRefreshRegions`] bullseye.
pub struct ProduceFromNodes<P, F>(PhantomData<fn() -> (P, F)>);

impl<P, F> Default for ProduceFromNodes<P, F> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<P, F> Plugin for ProduceFromNodes<P, F>
where
	P: Resource + LodRefreshRegions + Default,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		app.add_plugins(GenerationChannelPlugin::<P>::default())
			.init_resource::<P>()
			.add_systems(Update, produce_from_nodes::<P, F>.in_set(LodGenerateSystems::Produce));
	}
}

/// Publishes `P`'s bullseye region when a driver crosses its threshold.
pub fn produce_from_nodes<P, F>(
	producer: Res<P>,
	nodes: Query<
		(Entity, &LodNodePose, Option<&LodNodeBounds>),
		(With<LodNode>, Changed<LodNodePose>, F),
	>,
	mut channel: GenerationProducer<P>,
) where
	P: Resource + LodRefreshRegions,
	F: QueryFilter + 'static,
{
	if nodes.is_empty() {
		return;
	}
	let snapshots = collect_node_snapshots(&nodes);
	let refs = lod_refs_from_snapshots(&snapshots);
	let priority = refs.first().map(|lod_ref| lod_ref.current_transform.translation.xz());
	let Ok(LodRefreshRegionsStatus::Changed(region)) = producer.lod_refresh_regions_for(&refs)
	else {
		return;
	};
	channel.publish(region, priority);
}

/// Pending origin ids of `T` requested on `P`'s channel.
#[derive(Resource)]
pub struct GenerateQueue<P, T> {
	pending: VecDeque<Id>,
	pending_ids: HashSet<Id>,
	scan_regions: VecDeque<Aabb3d>,
	rescan: bool,
	last_keep: Option<Aabb3d>,
	_marker: PhantomData<fn() -> (P, T)>,
}

impl<P, T> Default for GenerateQueue<P, T> {
	fn default() -> Self {
		Self {
			pending: VecDeque::new(),
			pending_ids: HashSet::new(),
			scan_regions: VecDeque::new(),
			rescan: true,
			last_keep: None,
			_marker: PhantomData,
		}
	}
}

impl<P, T> GenerateQueue<P, T> {
	pub fn len(&self) -> usize {
		self.pending.len()
	}

	pub fn is_empty(&self) -> bool {
		self.pending.is_empty() && self.scan_regions.is_empty()
	}

	pub fn contains(&self, id: Id) -> bool {
		self.pending_ids.contains(&id)
	}

	/// Drops pending work and rescans the channel's current bounds next run.
	/// Returns how many job tickets to release.
	#[must_use]
	pub fn reset(&mut self) -> u64 {
		self.rescan = true;
		self.drop_pending()
	}

	fn drop_pending(&mut self) -> u64 {
		let cancelled = self.pending.len() as u64;
		self.pending.clear();
		self.pending_ids.clear();
		self.scan_regions.clear();
		cancelled
	}

	fn enqueue(&mut self, id: Id) -> bool {
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
		if !self
			.scan_regions
			.iter()
			.any(|queued| !keep_region_changed(Some(*queued), Some(region)))
		{
			self.scan_regions.push_back(region);
		}
	}

	fn retarget(&mut self, keep: Aabb3d, slack: f32) -> u64 {
		if !keep_region_changed(self.last_keep, Some(keep)) {
			return 0;
		}
		self.last_keep = Some(keep);
		let before = self.pending.len();
		self.pending.retain(|id| id_lives_in_keep(*id, keep, slack));
		self.pending_ids = self.pending.iter().copied().collect();
		let live = expand_keep_xz(keep, slack);
		self.scan_regions.retain(|region| overlaps_xz(*region, live));
		before.saturating_sub(self.pending.len()) as u64
	}
}

fn overlaps_xz(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.z <= b.max.z && a.max.z >= b.min.z
}

/// Replaces [`LodGenerateTimeBudget`] for subscribers on producer `P`'s channel.
#[derive(Resource, Debug, Clone, Copy)]
pub struct ChannelTimeBudget<P> {
	pub budget: LodGenerateTimeBudget,
	_channel: PhantomData<fn() -> P>,
}

impl<P> ChannelTimeBudget<P> {
	pub fn new(budget: LodGenerateTimeBudget) -> Self {
		Self { budget, _channel: PhantomData }
	}

	/// No wall-clock limit; [`LodGenerateBudget`] alone bounds each frame.
	pub fn unlimited() -> Self {
		Self::new(LodGenerateTimeBudget {
			time_per_frame: Duration::ZERO,
			max_atomic_cost: Duration::ZERO,
		})
	}
}

/// Subscribes `T` to producer `P`'s channel.
///
/// Runs in [`LodGenerateSystems::Drain`] unless [`Self::in_set`] names another set.
pub struct GenerateOn<P, T> {
	set: Option<InternedSystemSet>,
	_marker: PhantomData<fn() -> (P, T)>,
}

impl<P, T> Default for GenerateOn<P, T> {
	fn default() -> Self {
		Self { set: None, _marker: PhantomData }
	}
}

impl<P, T> GenerateOn<P, T> {
	/// Schedules the subscriber in `set` instead of [`LodGenerateSystems::Drain`].
	pub fn in_set(set: impl SystemSet) -> Self {
		Self { set: Some(set.intern()), _marker: PhantomData }
	}
}

impl<P, T> Plugin for GenerateOn<P, T>
where
	P: Send + Sync + 'static,
	T: GenerationScheme<HcsgStorage> + HcsgNode,
{
	fn build(&self, app: &mut App) {
		app.add_plugins(GenerationChannelPlugin::<P>::default())
			.init_resource::<HcsgStorage>()
			.init_resource::<LodGenerateBudget<P>>()
			.init_resource::<LodGenerateTimeBudget>()
			.init_resource::<GenerateQueue<P, T>>()
			.add_message::<LodGenerated<T>>();
		let system = generate::<P, T>;
		match self.set {
			Some(set) => app.add_systems(Update, system.in_set(set)),
			None => app.add_systems(Update, system.in_set(LodGenerateSystems::Drain)),
		};
	}
}

/// Scans newly requested bounds on `P` for `T`'s origins, then materializes a
/// budgeted, nearest-first slice of them.
///
/// Announces every origin of `T` in the requested bounds as [`LodGenerated`],
/// including ids another subscriber already built as a dependency.
#[allow(clippy::too_many_arguments)]
pub fn generate<P, T>(
	mut storage: ResMut<HcsgStorage>,
	mut queue: ResMut<GenerateQueue<P, T>>,
	current: Res<CurrentBounds<P>>,
	mut requests: MessageReader<GenerationRequest<P>>,
	budget: Res<LodGenerateBudget<P>>,
	global_time_budget: Res<LodGenerateTimeBudget>,
	channel_time_budget: Option<Res<ChannelTimeBudget<P>>>,
	jobs: Res<LodJobCounter>,
	mut generated: MessageWriter<LodGenerated<T>>,
) where
	P: Send + Sync + 'static,
	T: GenerationScheme<HcsgStorage> + HcsgNode,
{
	let started = Instant::now();
	let time_budget = channel_time_budget.map_or(*global_time_budget, |channel| channel.budget);
	let slack = current.slack_xz;

	if queue.rescan {
		requests.clear();
		if let Some(keep) = current.keep() {
			queue.rescan = false;
			queue.scan_regions.clear();
			jobs.end_n(queue.retarget(keep, slack));
			queue.enqueue_scan(keep);
		}
	} else {
		for request in requests.read() {
			jobs.end_n(queue.retarget(request.bounds.keep, slack));
			if request.bounds.restart {
				jobs.end_n(queue.drop_pending());
			}
			for region in &request.bounds.entering {
				queue.enqueue_scan(*region);
			}
		}
	}

	let keep = queue.last_keep;
	let mut scanned = false;
	while !time_up(started, time_budget.time_per_frame) {
		let Some(region) = queue.scan_regions.pop_front() else {
			break;
		};
		let quantum = Instant::now();
		for OriginalId(id) in storage.original_ids_for::<T>(region) {
			if keep.is_some_and(|keep| !id_lives_in_keep(id, keep, slack)) {
				continue;
			}
			// Another subscriber may have built `id` as a dependency. It is
			// still new to this channel's presenters.
			if SpatialIndex::<T>::storage_status(&*storage, id) != StorageStatus::NotTracked {
				generated.write(LodGenerated::new(id));
				continue;
			}
			if queue.enqueue(id) {
				jobs.begin();
			}
		}
		warn_atomic_overrun("generate region scan", quantum.elapsed(), time_budget.max_atomic_cost);
		scanned = true;
	}

	if queue.pending.is_empty() {
		return;
	}

	if scanned {
		if let Some(origin) = current.bounds.as_ref().and_then(|bounds| bounds.priority_xz) {
			let origin = Vec3::new(origin.x, 0.0, origin.y);
			queue.pending.make_contiguous().sort_by(|a, b| {
				id_xz_distance2(*a, origin).total_cmp(&id_xz_distance2(*b, origin))
			});
		}
	}

	for _ in 0..budget.ids_per_frame {
		if time_up(started, time_budget.time_per_frame) {
			break;
		}
		let Some(id) = queue.pop_front() else {
			break;
		};
		jobs.end();
		let quantum = Instant::now();
		if storage.get_or_generate::<T>(id).is_some() {
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
	debug!(
		stage,
		elapsed_us = elapsed.as_micros(),
		max_us = maximum.as_micros(),
		"HCSG generate quantum exceeded max_atomic_cost"
	);
}

/// Copies root input `R` into storage at `Id::Universal` whenever it changes.
///
/// Root inputs are values generation cannot derive (a seed, mode
/// configuration). Seed them here; derive everything else in a scheme.
pub struct Seed<R> {
	bounds: Aabb3d,
	_marker: PhantomData<fn() -> R>,
}

impl<R> Seed<R> {
	pub fn with_bounds(bounds: Aabb3d) -> Self {
		Self { bounds, _marker: PhantomData }
	}
}

impl<R> Default for Seed<R> {
	fn default() -> Self {
		Self::with_bounds(universal_bounds())
	}
}

/// Bounds for universal entries: everything a region query could ask about.
pub fn universal_bounds() -> Aabb3d {
	Aabb3d::from_min_max(Vec3::splat(-1.0e9), Vec3::splat(1.0e9))
}

/// Declares `T` a seeded root input: its only origin is `Id::Universal` and
/// it is never built, only [`Seed`]ed. Consumers still read it with
/// `get_one_or_generate(Id::Universal)`.
#[macro_export]
macro_rules! seeded_root {
	($T:ty) => {
		impl<S> $crate::gen::GenerationScheme<S> for $T {
			fn original_ids_for(
				_spatial_index: &mut S,
				_region: bevy::math::bounding::Aabb3d,
			) -> Vec<$crate::gen::OriginalId> {
				vec![$crate::gen::OriginalId::universal()]
			}

			fn build_with_id(
				_spatial_index: &mut S,
				_id: $crate::gen::Id,
			) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
				None
			}
		}
	};
}

/// Ordering for [`Seed`] systems: before every producer and subscriber.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct HcsgSeedSystems;

#[derive(Resource)]
struct SeedBounds<R> {
	bounds: Aabb3d,
	_marker: PhantomData<fn() -> R>,
}

impl<R> Plugin for Seed<R>
where
	R: Resource + Clone,
{
	fn build(&self, app: &mut App) {
		ensure_generate_sets(app);
		app.init_resource::<HcsgStorage>()
			.insert_resource(SeedBounds::<R> { bounds: self.bounds, _marker: PhantomData })
			.configure_sets(Update, HcsgSeedSystems.before(LodGenerateSystems::Produce))
			.add_systems(Update, seed_universal::<R>.in_set(HcsgSeedSystems));
	}
}

fn seed_universal<R: Resource + Clone>(
	source: Option<Res<R>>,
	bounds: Res<SeedBounds<R>>,
	mut storage: ResMut<HcsgStorage>,
) {
	let Some(source) = source else {
		return;
	};
	if source.is_changed() || !storage.contains::<R>(Id::Universal) {
		storage.seed(source.clone(), bounds.bounds);
	}
}
