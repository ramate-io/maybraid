//! Present-region production, budgeted handle, and hide-then-despawn cull.

use std::collections::{HashSet, VecDeque};
use std::marker::PhantomData;

use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use super::RegionPresenter;
use crate::gen::{expire_pending_outside_keep, Id, SpatialIndex};
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

/// Impulse: cull-evaluate presented ids overlapping `region`.
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

/// Pending present ids for type `T`.
#[derive(Resource, Debug)]
pub struct LodPresentQueue<T> {
	pub pending: VecDeque<Id>,
	_marker: PhantomData<T>,
}

impl<T> Default for LodPresentQueue<T> {
	fn default() -> Self {
		Self { pending: VecDeque::new(), _marker: PhantomData }
	}
}

/// Last present-ring AABB for this channel (cull `keep` set).
#[derive(Resource, Debug)]
pub struct LodPresentKeepRegion<M: Send + Sync + 'static> {
	pub region: Option<Aabb3d>,
	_marker: PhantomData<M>,
}

impl<M: Send + Sync + 'static> Default for LodPresentKeepRegion<M> {
	fn default() -> Self {
		Self { region: None, _marker: PhantomData }
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
		app.configure_sets(
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
	budget: Res<LodPresentBudget>,
	mut regions: MessageReader<LodPresentRegion<M>>,
	keep: Res<LodPresentKeepRegion<M>>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, F)>,
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
	if let Some(keep_region) = keep.region {
		if !scan.iter().any(|region| regions_match(*region, keep_region)) {
			scan.push(keep_region);
		}
	}
	for region in scan {
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
			if !queue.pending.contains(&tracked.0) {
				queue.pending.push_back(tracked.0);
			}
		}
	}

	expire_pending_outside_keep(&mut queue.pending, keep.region);

	if queue.pending.is_empty() {
		return;
	}

	let snapshots = collect_node_snapshots(&nodes);
	let refs = lod_refs_from_snapshots(&snapshots);
	let Some(lod_ref) = refs.first() else {
		return;
	};

	let origin = lod_ref.current_transform.translation;
	queue.pending.make_contiguous().sort_by(|a, b| {
		id_distance(*a, origin)
			.partial_cmp(&id_distance(*b, origin))
			.unwrap_or(std::cmp::Ordering::Equal)
	});

	let n = budget.ids_per_frame.max(1) as usize;
	let mut handled = 0;
	while handled < n {
		let Some(id) = queue.pending.pop_front() else {
			break;
		};
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
		handled += 1;
	}
}

fn id_distance(id: Id, origin: Vec3) -> f32 {
	let Some(bounds) = id.origin_cell_bounds() else {
		return f32::MAX;
	};
	let center = (bounds.min + bounds.max) * 0.5;
	let dx = center.x - origin.x;
	let dz = center.z - origin.z;
	dx * dx + dz * dz
}

fn regions_match(a: Aabb3d, b: Aabb3d) -> bool {
	(a.min.x - b.min.x).abs() < 1e-3
		&& (a.max.x - b.max.x).abs() < 1e-3
		&& (a.min.z - b.min.z).abs() < 1e-3
		&& (a.max.z - b.max.z).abs() < 1e-3
}

/// Emit [`LodPresentCullRegion<M>`] via strategy `P` and this layer's cursor.
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

/// Hide, then despawn, presented ids in cull tiles outside the keep ring.
pub fn drain_lod_present_cull<T, S, Pr, M>(
	presenter: StaticSystemParam<Pr>,
	index: Res<S>,
	keep: Res<LodPresentKeepRegion<M>>,
	mut regions: MessageReader<LodPresentCullRegion<M>>,
) where
	T: Send + Sync + 'static,
	S: Resource + SpatialIndex<T>,
	Pr: SystemParam + 'static,
	for<'w, 's> Pr::Item<'w, 's>: RegionPresenter<T, S>,
	M: Send + Sync + 'static,
{
	if regions.is_empty() {
		return;
	}
	let keep_ids: HashSet<Id> = keep
		.region
		.map(|region| index.tracked_ids_for(region).into_iter().map(|tracked| tracked.0).collect())
		.unwrap_or_default();
	let mut presenter = presenter.into_inner();
	for message in regions.read() {
		presenter.cull(&*index, message.region, &keep_ids);
	}
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
			.init_resource::<LodPresentQueue<T>>()
			.init_resource::<LodPresentKeepRegion<M>>()
			.add_message::<LodPresentRegion<M>>()
			.add_systems(
				Update,
				drain_lod_present::<T, S, Pr, M, F>.in_set(LodPresentSystems::Drain),
			);
	}
}

/// Produce [`LodPresentCullRegion<M>`] via strategy `P`.
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
		app.init_resource::<LodPresentKeepRegion<M>>().add_systems(
			Update,
			drain_lod_present_cull::<T, S, Pr, M>.in_set(LodPresentSystems::Cull),
		);
	}
}
