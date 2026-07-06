//! Presentation. A pass over storage that runs after generation completes.
//!
//! The presenter owns runtime truth: which ids it has presented, at which
//! [`Version`]. Deciding what to (re)present is a per-id version comparison
//! against the index, so correctness does not depend on each asset's local
//! opinion about its scene, and no commit phase is needed between generation
//! and presentation.

use crate::gen::id::Id;
use crate::gen::spatial_index::{SpatialIndex, Version};
use crate::lod_ref::LodRef;
use bevy::{math::bounding::Aabb3d, scene::Scene};
use std::collections::HashSet;

pub trait LodScene {
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static;
}

/// Presents one layer (`T`) of a spatial index over a region.
///
/// A top-level controller overrides [`RegionPresenter::present_all`] to chain
/// the descendant layers it wants visible; presentation order and selection
/// are policy, unlike generation-tree recursion which is a data dependency and
/// stays automatic in [`crate::gen::GeneratingSpatialIndex`].
pub trait RegionPresenter<T, S>
where
	T: LodScene,
	S: SpatialIndex<T>,
{
	/// The version this presenter last presented for the id, if any.
	fn presented_version(&self, id: Id) -> Option<Version>;

	/// Spawns or patches the scene for the id. Implementations must record
	/// `version` so [`RegionPresenter::presented_version`] reflects it.
	fn handle(&mut self, id: Id, version: Version, scene: impl Scene, lod_ref: &LodRef);

	/// Removes presented ids within the region that are not in `wanted`.
	///
	/// Contract: strictly removal. This runs *after* the handle pass in
	/// [`RegionPresenter::present`], so every wanted id has already been
	/// presented; the only remaining job is dropping stale ones. Moved assets
	/// are covered by the pair: the old region's pass removes the id here,
	/// the new region's handle pass presents it.
	fn remove_stale(&mut self, region: Aabb3d, wanted: &HashSet<Id>);

	fn present(&mut self, spatial_index: &S, region: Aabb3d, lod_ref: &LodRef) {
		let wanted: HashSet<Id> = spatial_index
			.tracked_ids_for(region)
			.into_iter()
			.map(|tracked| tracked.0)
			.collect();

		for &id in &wanted {
			let Some(version) = spatial_index.version(id) else {
				continue;
			};
			if self.presented_version(id).is_none_or(|presented| presented < version) {
				if let Some(instance) = spatial_index.get(id) {
					self.handle(id, version, instance.scene_with_lod(lod_ref), lod_ref);
				}
			}
		}

		self.remove_stale(region, &wanted);
	}

	/// By default presents just this layer. Top-level controllers override
	/// this to also present descendant layers.
	fn present_all(&mut self, spatial_index: &S, region: Aabb3d, lod_ref: &LodRef) {
		self.present(spatial_index, region, lod_ref);
	}
}
