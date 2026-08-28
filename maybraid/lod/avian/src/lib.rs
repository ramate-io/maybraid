//! Avian-backed region indexes for LOD generate / present / scene refresh.
//!
//! Query volumes are stamped by layer-specific marshallers
//! ([`AvianLodGenerateBoundsMarshaller`], [`AvianLodPresentBoundsMarshaller`],
//! [`AvianLodSceneBoundsMarshaller`]) so each spatial query can mask to one
//! [`PhysicsInteractionLayer`]. Hosts must still carry an Avian [`Collider`]
//! on the **host** entity (no `RigidBody` required — query-only).
//!
//! Scene hosts use [`PhysicsInteractionLayer::Host`]. Generated and presented
//! volumes use [`PhysicsInteractionLayer::Generate`] and
//! [`PhysicsInteractionLayer::Present`]. All three are query-only and do not
//! enter narrowphase against terrain / buildings ([`layers`]).

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
	LodSceneHost, LodSceneHostIndex, LodSceneRefreshPlugin, LodSceneRegionIndex, LodViewer,
	PatchSceneBounds,
};

/// [`LodSceneBoundsMarshaller`] for generated-id volumes ([`PhysicsInteractionLayer::Generate`]).
#[derive(Debug, Clone, Copy, Default)]
pub struct AvianLodGenerateBoundsMarshaller;

/// [`LodSceneBoundsMarshaller`] for presented-id volumes ([`PhysicsInteractionLayer::Present`]).
#[derive(Debug, Clone, Copy, Default)]
pub struct AvianLodPresentBoundsMarshaller;

/// [`LodSceneBoundsMarshaller`] for scene-host volumes ([`PhysicsInteractionLayer::Host`]).
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

/// Untyped Avian lookup of generated-id volumes.
#[derive(SystemParam)]
pub struct AvianLodGenerateIndex<'w, 's> {
	spatial: SpatialQuery<'w, 's>,
}

impl AvianLodGenerateIndex<'_, '_> {
	pub fn entities_in_region(&self, region: Aabb3d) -> Vec<Entity> {
		region_hits(&self.spatial, region, PhysicsInteractionLayer::Generate)
	}
}

/// Untyped Avian lookup of presented-id volumes.
#[derive(SystemParam)]
pub struct AvianLodPresentIndex<'w, 's> {
	spatial: SpatialQuery<'w, 's>,
}

impl AvianLodPresentIndex<'_, '_> {
	pub fn entities_in_region(&self, region: Aabb3d) -> Vec<Entity> {
		region_hits(&self.spatial, region, PhysicsInteractionLayer::Present)
	}
}

/// Untyped Avian host lookup for the shared produce cache.
///
/// Hits are already restricted to [`PhysicsInteractionLayer::Host`], so this
/// does not scan generate / present / terrain / mover colliders.
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

fn ensure_avian_host_bounds<T: Component + SemanticLodScene + 'static>(app: &mut App) {
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
		ensure_avian_host_bounds::<T>(app);
		app.add_plugins(
			lod::LodSceneRegionCullPlugin::<AvianLodSceneHostIndex<'_, '_>, M, T, F>::default(),
		);
	}
}
