//! Avian-backed region index for LOD refresh ([`LodSceneRegionIndex`](lod::LodSceneRegionIndex)).
//!
//! Hosts must carry an Avian [`Collider`] on the **host** entity (no `RigidBody`
//! required — query-only) and a `T: Component + LodScene` on a
//! [`LodSceneHost`](lod::LodSceneHost). Prefer
//! [`PatchSceneBounds`](lod::PatchSceneBounds) with
//! [`AvianLodSceneBoundsMarshaller`] to stamp volumes from
//! [`LodScene::scene_bounds`](lod::LodScene::scene_bounds).

use std::any::type_name;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::time::Instant;

use avian3d::prelude::{Collider, ColliderAabb, SpatialQuery};
use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::LodScene;
use lod::{
	LodSceneBoundsMarshaller, LodSceneHost, LodSceneRegionIndex, LodSceneRefreshPlugin,
	LodViewer, PatchSceneBounds,
};

/// Frame-aggregated timing for Avian `aabb_intersections_with_aabb` in LOD refresh.
#[derive(Resource, Debug, Default)]
pub struct AvianLodSpatialQueryDiag {
	pub queries: u32,
	pub hits: u32,
	pub total_ms: f64,
	pub last_query_ms: f64,
	pub last_hits: u32,
}

impl AvianLodSpatialQueryDiag {
	fn record(&mut self, query_ms: f64, hits: usize) {
		let hits = hits as u32;
		self.queries = self.queries.saturating_add(1);
		self.hits = self.hits.saturating_add(hits);
		self.total_ms += query_ms;
		self.last_query_ms = query_ms;
		self.last_hits = hits;
	}

	fn take_frame(&mut self) -> (u32, u32, f64) {
		let out = (self.queries, self.hits, self.total_ms);
		self.queries = 0;
		self.hits = 0;
		self.total_ms = 0.0;
		out
	}
}

/// [`LodSceneBoundsMarshaller`] that inserts a query-only Avian [`Collider`] on the host.
#[derive(Debug, Clone, Copy, Default)]
pub struct AvianLodSceneBoundsMarshaller;

impl LodSceneBoundsMarshaller for AvianLodSceneBoundsMarshaller {
	type Volume = Collider;

	fn volume_from_bounds(bounds: Aabb3d) -> Self::Volume {
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let center = (min + max) * 0.5;
		let size = (max - min).max(Vec3::splat(1e-3));
		let cuboid = Collider::cuboid(size.x, size.y, size.z);
		if center.length_squared() <= 1e-8 {
			cuboid
		} else {
			Collider::compound(vec![(center, Quat::IDENTITY, cuboid)])
		}
	}
}

/// [`SystemParam`] Avian implementation of [`LodSceneRegionIndex`] for host type `T`.
///
/// Only colliders on the host entity itself count (physics children do not enroll
/// the host in LOD refresh).
#[derive(SystemParam)]
pub struct AvianLodSceneRegionIndex<'w, 's, T: Component + LodScene + 'static> {
	spatial: SpatialQuery<'w, 's>,
	hosts: Query<'w, 's, (Entity, &'static T), With<LodSceneHost>>,
	diag: ResMut<'w, AvianLodSpatialQueryDiag>,
}

impl<T: Component + LodScene + 'static> LodSceneRegionIndex<T>
	for AvianLodSceneRegionIndex<'_, '_, T>
{
	fn hosts_in_region<'a>(
		&'a mut self,
		region: Aabb3d,
	) -> impl Iterator<Item = (Entity, &'a T)> + 'a {
		let collider = ColliderAabb::from_min_max(Vec3::from(region.min), Vec3::from(region.max));
		let t0 = Instant::now();
		let hit: HashSet<Entity> =
			self.spatial.aabb_intersections_with_aabb(collider).into_iter().collect();
		let query_ms = t0.elapsed().as_secs_f64() * 1000.0;
		self.diag.record(query_ms, hit.len());
		if query_ms >= 0.25 {
			info!(
				"[lod.refresh] spatial.query: host={} hits={} aabb_ms={query_ms:.2}",
				type_name::<T>(),
				hit.len()
			);
		}
		self.hosts
			.iter()
			.filter(move |(entity, _)| hit.contains(entity))
			.map(|(entity, scene)| (entity, scene))
	}
}

fn log_avian_spatial_query_diag(mut diag: ResMut<AvianLodSpatialQueryDiag>) {
	let (queries, hits, total_ms) = diag.take_frame();
	if queries == 0 {
		return;
	}
	if total_ms >= 0.25 || queries > 1 {
		let avg = total_ms / f64::from(queries.max(1));
		info!(
			"[lod.refresh] spatial.frame: queries={queries} hits={hits} \
			 total_ms={total_ms:.2} avg_ms={avg:.2} last_ms={:.2}",
			diag.last_query_ms
		);
	}
}

/// Ensures [`AvianLodSpatialQueryDiag`] + once-per-frame summary logging.
struct AvianLodSpatialDiagPlugin;

impl Plugin for AvianLodSpatialDiagPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<AvianLodSpatialQueryDiag>().add_systems(
			Update,
			log_avian_spatial_query_diag.after(lod::LodRefreshSystems::ProduceLevels),
		);
	}
}

/// [`LodSceneRefreshPlugin`] with [`AvianLodSceneRegionIndex`] + host volume patch.
///
/// `T` listens for [`lod::LodSceneRefreshRegion`] on channel `M`; levels use
/// [`LodNode`]s filtered by `F` (default: [`LodViewer`]).
pub struct AvianLodSceneRefreshPlugin<T, M, F = With<LodViewer>>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, M, F)>,
}

impl<T, M, F> Default for AvianLodSceneRefreshPlugin<T, M, F>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self {
			_marker: PhantomData,
		}
	}
}

impl<T, M, F> Plugin for AvianLodSceneRefreshPlugin<T, M, F>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<AvianLodSpatialDiagPlugin>() {
			app.add_plugins(AvianLodSpatialDiagPlugin);
		}
		if !app.is_plugin_added::<PatchSceneBounds<T, AvianLodSceneBoundsMarshaller>>() {
			app.add_plugins(PatchSceneBounds::<T, AvianLodSceneBoundsMarshaller>::default());
		}
		app.add_plugins(LodSceneRefreshPlugin::<
			T,
			M,
			AvianLodSceneRegionIndex<'_, '_, T>,
			F,
		>::default());
	}
}
