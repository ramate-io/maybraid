//! Avian-backed region index for LOD refresh ([`LodSceneRegionIndex`](lod::LodSceneRegionIndex)).
//!
//! Hosts must carry an Avian [`Collider`] on the **host** entity (no `RigidBody`
//! required — query-only) and a `T: Component + LodScene` on a
//! [`LodSceneHost`](lod::LodSceneHost). Prefer
//! [`PatchSceneBounds`](lod::PatchSceneBounds) with
//! [`AvianLodSceneBoundsMarshaller`] to stamp volumes from
//! [`LodScene::scene_bounds`](lod::LodScene::scene_bounds).
//!
//! Host volumes use [`PhysicsInteractionLayer::Host`] with empty filters so they
//! do not enter narrowphase against terrain / buildings ([`layers`]).

mod layers;

pub use layers::{AvianLodHostVolume, PhysicsInteractionLayer};

use std::marker::PhantomData;

use avian3d::prelude::{ColliderAabb, SpatialQuery};
use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::LodScene;
use lod::{
	LodSceneHost, LodSceneHostIndex, LodSceneRefreshPlugin, LodSceneRegionIndex, LodViewer,
	PatchSceneBounds,
};

/// [`LodSceneBoundsMarshaller`](lod::LodSceneBoundsMarshaller) that inserts a
/// query-only Avian [`avian3d::prelude::Collider`] + Host [`CollisionLayers`](avian3d::prelude::CollisionLayers).
#[derive(Debug, Clone, Copy, Default)]
pub struct AvianLodSceneBoundsMarshaller;

/// Untyped Avian host lookup for the shared produce cache.
///
/// Only colliders on the host entity itself count (physics children do not enroll
/// the host in LOD refresh).
#[derive(SystemParam)]
pub struct AvianLodSceneHostIndex<'w, 's> {
	spatial: SpatialQuery<'w, 's>,
	hosts: Query<'w, 's, Entity, With<LodSceneHost>>,
}

impl LodSceneHostIndex for AvianLodSceneHostIndex<'_, '_> {
	fn hosts_in_region<'a>(&'a mut self, region: Aabb3d) -> impl Iterator<Item = Entity> + 'a {
		let collider = ColliderAabb::from_min_max(Vec3::from(region.min), Vec3::from(region.max));
		let hits = self.spatial.aabb_intersections_with_aabb(collider);
		hits.into_iter().filter(|entity| self.hosts.contains(*entity))
	}
}

/// [`SystemParam`] Avian implementation of [`LodSceneRegionIndex`] for host type `T`.
///
/// Only colliders on the host entity itself count (physics children do not enroll
/// the host in LOD refresh). Used by region-scoped cull.
#[derive(SystemParam)]
pub struct AvianLodSceneRegionIndex<'w, 's, T: Component + LodScene + 'static> {
	spatial: SpatialQuery<'w, 's>,
	hosts: Query<'w, 's, &'static T, With<LodSceneHost>>,
}

impl<T: Component + LodScene + 'static> LodSceneRegionIndex<T>
	for AvianLodSceneRegionIndex<'_, '_, T>
{
	fn hosts_in_region<'a>(
		&'a mut self,
		region: Aabb3d,
	) -> impl Iterator<Item = (Entity, &'a T)> + 'a {
		let collider = ColliderAabb::from_min_max(Vec3::from(region.min), Vec3::from(region.max));
		let hits = self.spatial.aabb_intersections_with_aabb(collider);
		hits.into_iter()
			.filter_map(|entity| self.hosts.get(entity).ok().map(|scene| (entity, scene)))
	}
}

fn ensure_avian_host_bounds<T: Component + LodScene + 'static>(app: &mut App) {
	if !app.is_plugin_added::<PatchSceneBounds<T, AvianLodSceneBoundsMarshaller>>() {
		app.add_plugins(PatchSceneBounds::<T, AvianLodSceneBoundsMarshaller>::default());
	}
}

/// [`LodSceneRefreshPlugin`] with [`AvianLodSceneHostIndex`] + host volume patch.
///
/// Fill is once per (`I`, `F`); emit is once per `T`. Channel `M` is accepted so
/// existing dual bullseye/spotlight plugin adds stay valid.
///
/// Use [`Self::without_full_scan_cull`] with [`AvianLodSceneCullPlugin`] for
/// OpenLattice (or other) region-scoped cull enqueue.
pub struct AvianLodSceneRefreshPlugin<T, M, F = With<LodViewer>>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	full_scan_cull: bool,
	_marker: PhantomData<fn() -> (T, M, F)>,
}

impl<T, M, F> Default for AvianLodSceneRefreshPlugin<T, M, F>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self { full_scan_cull: true, _marker: PhantomData }
	}
}

impl<T, M, F> AvianLodSceneRefreshPlugin<T, M, F>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	pub fn without_full_scan_cull() -> Self {
		Self { full_scan_cull: false, _marker: PhantomData }
	}
}

impl<T, M, F> Plugin for AvianLodSceneRefreshPlugin<T, M, F>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_avian_host_bounds::<T>(app);
		if self.full_scan_cull {
			app.add_plugins(
				LodSceneRefreshPlugin::<T, M, AvianLodSceneHostIndex<'_, '_>, F>::default(),
			);
		} else {
			app.add_plugins(LodSceneRefreshPlugin::<
				T,
				M,
				AvianLodSceneHostIndex<'_, '_>,
				F,
			>::without_full_scan_cull());
		}
	}
}

/// Region-scoped cull enqueue for host `T` on cull channel `M` (Avian index).
pub struct AvianLodSceneCullPlugin<T, M, F = With<LodViewer>>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, M, F)>,
}

impl<T, M, F> Default for AvianLodSceneCullPlugin<T, M, F>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<T, M, F> Plugin for AvianLodSceneCullPlugin<T, M, F>
where
	T: Component + LodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_avian_host_bounds::<T>(app);
		app.add_plugins(lod::LodSceneRegionCullPlugin::<
			AvianLodSceneRegionIndex<'_, '_, T>,
			M,
			T,
			F,
		>::default());
	}
}
