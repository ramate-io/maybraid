//! Generate-region production and budgeted index insert.

use std::collections::VecDeque;
use std::marker::PhantomData;

use bevy::ecs::component::Mutable;
use bevy::ecs::query::QueryFilter;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use crate::gen::{
	expand_keep_xz, expire_pending_outside_keep, id_xz_distance2, GeneratingSpatialIndex,
	GenerationScheme, Id, StorageStatus, QUEUE_KEEP_SLACK_XZ,
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

/// Pending origin ids for type `T`.
#[derive(Resource, Debug)]
pub struct LodGenerateQueue<T> {
	pub pending: VecDeque<Id>,
	_marker: PhantomData<T>,
}

impl<T> Default for LodGenerateQueue<T> {
	fn default() -> Self {
		Self { pending: VecDeque::new(), _marker: PhantomData }
	}
}

/// Last generate-ring AABB for this channel. Drain polls this every frame so a
/// one-shot region is not required after the first ids land in the index.
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
		app.configure_sets(
			Update,
			(LodGenerateSystems::Produce, LodGenerateSystems::Drain)
				.chain()
				.after(LodNodeSystems::Track),
		);
	}
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
	budget: Res<LodGenerateBudget>,
	mut regions: MessageReader<LodGenerateRegion<M>>,
	keep: Res<LodGenerateKeepRegion<M>>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), (With<LodNode>, F)>,
) where
	T: GenerationScheme<S> + Send + Sync + 'static,
	S: Resource<Mutability = Mutable> + GeneratingSpatialIndex<T>,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	let mut scan: Vec<Aabb3d> = regions.read().map(|message| message.region).collect();
	push_keep_region(&mut scan, keep.region);
	for region in scan {
		for original in T::original_ids_for(&mut *index, region) {
			if index.storage_status(original.0) != StorageStatus::NotTracked {
				continue;
			}
			if !queue.pending.contains(&original.0) {
				queue.pending.push_back(original.0);
			}
		}
	}

	expire_pending_outside_keep(&mut queue.pending, keep.region, keep.slack_xz);

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
		id_xz_distance2(*a, origin)
			.partial_cmp(&id_xz_distance2(*b, origin))
			.unwrap_or(std::cmp::Ordering::Equal)
	});

	let n = budget.ids_per_frame.max(1) as usize;
	for _ in 0..n {
		let Some(id) = queue.pending.pop_front() else {
			break;
		};
		index.get_or_generate(id, lod_ref);
	}
}

fn push_keep_region(scan: &mut Vec<Aabb3d>, keep: Option<Aabb3d>) {
	let Some(keep_region) = keep else {
		return;
	};
	if scan.iter().any(|region| regions_match(*region, keep_region)) {
		return;
	}
	scan.push(keep_region);
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
		app.init_resource::<LodGenerateBudget>()
			.init_resource::<LodGenerateQueue<T>>()
			.init_resource::<LodGenerateKeepRegion<M>>()
			.add_message::<LodGenerateRegion<M>>()
			.add_systems(
				Update,
				drain_lod_generate::<T, S, M, F>.in_set(LodGenerateSystems::Drain),
			);
	}
}
