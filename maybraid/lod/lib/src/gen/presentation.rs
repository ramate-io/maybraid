//! Presentation. A pass over storage that runs after generation completes.
//!
//! The presenter owns runtime truth: which ids it has presented, at which
//! [`Version`]. Deciding what to (re)present is a per-id version comparison
//! against the index, so correctness does not depend on each asset's local
//! opinion about its scene, and no commit phase is needed between generation
//! and presentation.

#[cfg(test)]
pub mod tests;

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
/// [`RegionPresenter::present`] is the per-layer entry point: wanted ids are
/// spawned or patched first, then stale ids are removed. Healing is part of
/// presentation, not a separate caller concern.
///
/// Two extension points sit above it:
///
/// - [`RegionPresenter::present_with_descendants`] — compose the logical
///   descendant layers of a generation chain. Override this on types that
///   own a subtree and delegate to child-layer presenters.
/// - [`RegionPresenter::present_all`] — liberal entry point for "present
///   everything relevant in this region." Defaults to
///   [`RegionPresenter::present_with_descendants`]. A common pattern for types
///   at the **root of a generation hierarchy** is to override
///   `present_with_descendants` on that root; types used this way are **index
///   types**: they name a hierarchy and can be referred to when indexing that
///   tree from its root.
///
/// Generation-tree recursion stays automatic in
/// [`crate::gen::GeneratingSpatialIndex`]; presentation order and layer
/// selection are policy, expressed through these overrides.
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
	/// Contract: strictly removal. [`RegionPresenter::present`] invokes this
	/// after the handle pass, so every wanted id has already been presented.
	fn remove_stale(&mut self, region: Aabb3d, wanted: &HashSet<Id>);

	/// Presents and heals this layer.
	///
	/// Contract:
	///
	/// - wanted ids are spawned or patched first
	/// - stale ids are removed after
	/// - callers should use this method for all normal presentation
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

			let needs_present = self
				.presented_version(id)
				.is_none_or(|presented| presented < version);

			if needs_present || self.needs_repair(region, id, version) {
				if let Some(instance) = spatial_index.get(id) {
					self.handle(id, version, instance.scene_with_lod(lod_ref), lod_ref);
				}
			}
		}

		self.remove_stale(region, &wanted);
	}

	/// Optional runtime-world check.
	///
	/// Lets the presenter repair an id even when its version has not changed,
	/// e.g. the entity was healed away, parented under the wrong chunk, or
	/// otherwise missing from the expected region.
	fn needs_repair(&self, _region: Aabb3d, _id: Id, _version: Version) -> bool {
		false
	}

	/// Present this layer and its logical descendants in the generation chain.
	///
	/// **Contract:** overrides should delegate to descendant-layer presenters,
	/// composing the visible tree layer by layer. The default presents only
	/// this layer ([`RegionPresenter::present`]).
	fn present_with_descendants(&mut self, spatial_index: &S, region: Aabb3d, lod_ref: &LodRef) {
		self.present(spatial_index, region, lod_ref);
	}

	/// Present everything relevant for this region.
	///
	/// **Contract:** intentionally liberal — callers may use this for ad-hoc
	/// composition. Defaults to [`RegionPresenter::present_with_descendants`].
	/// Index types override `present_with_descendants` on the hierarchy root.
	fn present_all(&mut self, spatial_index: &S, region: Aabb3d, lod_ref: &LodRef) {
		self.present_with_descendants(spatial_index, region, lod_ref);
	}
}
