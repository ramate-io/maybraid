//! Gimme scene-host lookup plus Avian physics layers.
//!
//! Scene-host refresh and cull use [`lod::GimmeLodSceneHostIndex`] so Host
//! colliders are not part of the physics broadphase.
//! [`PatchSceneBounds`] stamps [`GimmeLodHostMarshaller`] volumes, not Avian
//! cuboids.
//!
//! Generate and present id lookup stays on typed [`lod::gen::SpatialIndex`]
//! resources. This crate no longer stamps query-only Generate / Present
//! colliders.

mod layers;

pub use layers::{AvianLodHostVolume, AvianLodQueryVolume, PhysicsInteractionLayer};

use std::marker::PhantomData;

use avian3d::prelude::{Collider, SpatialQuery, SpatialQueryFilter};
use bevy::ecs::query::QueryFilter;
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::SemanticLodScene;
use lod::{
	GimmeLodHostMarshaller, GimmeLodHostPlugin, GimmeLodSceneHostIndex, LodSceneHost,
	LodSceneHostIndex, LodSceneRefreshPlugin, LodSceneRegionIndex, LodViewer, PatchSceneBounds,
};

/// [`LodSceneBoundsMarshaller`] for scene-host volumes ([`PhysicsInteractionLayer::Host`]).
///
/// Refresh plugins do not install this marshaller; they use
/// [`GimmeLodHostMarshaller`]. Kept so an Avian host query remains possible
/// without reintroducing Generate / Present volumes.
#[derive(Debug, Clone, Copy, Default)]
pub struct AvianLodSceneBoundsMarshaller;

fn region_hits(
	spatial: &SpatialQuery,
	region: Aabb3d,
	layer: PhysicsInteractionLayer,
) -> Vec<Entity> {
	let min = Vec3::from(region.min);
	let max = Vec3::from(region.max);
	let center = (min + max) * 0.5;
	let size = (max - min).max(Vec3::splat(1e-3));
	spatial.shape_intersections(
		&Collider::cuboid(size.x, size.y, size.z),
		center,
		Quat::IDENTITY,
		&SpatialQueryFilter::from_mask(layer),
	)
}

/// Untyped Avian host lookup.
///
/// Hits are already restricted to [`PhysicsInteractionLayer::Host`]. Refresh
/// and cull plugins use [`GimmeLodSceneHostIndex`] instead.
#[derive(SystemParam)]
pub struct AvianLodSceneHostIndex<'w, 's> {
	spatial: SpatialQuery<'w, 's>,
}

impl LodSceneHostIndex for AvianLodSceneHostIndex<'_, '_> {
	fn hosts_in_region<'a>(&'a mut self, region: Aabb3d) -> impl Iterator<Item = Entity> + 'a {
		region_hits(&self.spatial, region, PhysicsInteractionLayer::Host).into_iter()
	}
}

/// [`SystemParam`] Avian implementation of [`LodSceneRegionIndex`] for host type `T`.
///
/// Layer-masked to [`PhysicsInteractionLayer::Host`], then resolved as `T`.
#[derive(SystemParam)]
pub struct AvianLodSceneRegionIndex<'w, 's, T: Component + SemanticLodScene + 'static> {
	spatial: SpatialQuery<'w, 's>,
	hosts: Query<'w, 's, &'static T, With<LodSceneHost>>,
}

impl<T: Component + SemanticLodScene + 'static> LodSceneRegionIndex<T>
	for AvianLodSceneRegionIndex<'_, '_, T>
{
	fn hosts_in_region<'a>(
		&'a mut self,
		region: Aabb3d,
	) -> impl Iterator<Item = (Entity, &'a T)> + 'a {
		region_hits(&self.spatial, region, PhysicsInteractionLayer::Host)
			.into_iter()
			.filter_map(|entity| self.hosts.get(entity).ok().map(|scene| (entity, scene)))
	}
}

fn ensure_gimme_host_index<T: Component + SemanticLodScene + 'static>(app: &mut App) {
	if !app.is_plugin_added::<GimmeLodHostPlugin>() {
		app.add_plugins(GimmeLodHostPlugin);
	}
	if !app.is_plugin_added::<PatchSceneBounds<T, GimmeLodHostMarshaller>>() {
		app.add_plugins(PatchSceneBounds::<T, GimmeLodHostMarshaller>::default());
	}
}

/// [`LodSceneRefreshPlugin`] with [`GimmeLodSceneHostIndex`].
///
/// Fill is once per (`I`, `F`); emit is once per `T`. Channel `M` is accepted so
/// existing dual bullseye/spotlight plugin adds stay valid.
///
/// Use [`Self::without_full_scan_cull`] with [`AvianLodSceneCullPlugin`] for
/// OpenLattice (or other) region-scoped cull enqueue.
pub struct AvianLodSceneRefreshPlugin<T, M, F = With<LodViewer>>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	full_scan_cull: bool,
	_marker: PhantomData<fn() -> (T, M, F)>,
}

impl<T, M, F> Default for AvianLodSceneRefreshPlugin<T, M, F>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self { full_scan_cull: true, _marker: PhantomData }
	}
}

impl<T, M, F> AvianLodSceneRefreshPlugin<T, M, F>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	pub fn without_full_scan_cull() -> Self {
		Self { full_scan_cull: false, _marker: PhantomData }
	}
}

impl<T, M, F> Plugin for AvianLodSceneRefreshPlugin<T, M, F>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_gimme_host_index::<T>(app);
		if self.full_scan_cull {
			app.add_plugins(
				LodSceneRefreshPlugin::<T, M, GimmeLodSceneHostIndex<'_>, F>::default(),
			);
		} else {
			app.add_plugins(LodSceneRefreshPlugin::<
				T,
				M,
				GimmeLodSceneHostIndex<'_>,
				F,
			>::without_full_scan_cull());
		}
	}
}

/// Region-scoped cull enqueue for host `T` on cull channel `M` (Gimme index).
pub struct AvianLodSceneCullPlugin<T, M, F = With<LodViewer>>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	_marker: PhantomData<fn() -> (T, M, F)>,
}

impl<T, M, F> Default for AvianLodSceneCullPlugin<T, M, F>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<T, M, F> Plugin for AvianLodSceneCullPlugin<T, M, F>
where
	T: Component + SemanticLodScene + 'static,
	M: Send + Sync + 'static,
	F: QueryFilter + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_gimme_host_index::<T>(app);
		app.add_plugins(
			lod::LodSceneRegionCullPlugin::<GimmeLodSceneHostIndex<'_>, M, T, F>::default(),
		);
	}
}
