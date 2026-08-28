//! Presentation. A pass over storage that runs after generation completes.
//!
//! The presenter owns runtime truth: which ids it has presented, at which
//! [`Version`]. Deciding what to (re)present is a per-id version comparison
//! against the index, so correctness does not depend on each asset's local
//! opinion about its scene, and no commit phase is needed between generation
//! and presentation.
//!
//! `T` is not required to be a [`crate::scene::LodScene`]. A presenter may
//! spawn scenes, grove hosts, or anything else. LOD level selection lives on
//! the scene stack after hosts exist.
//!
//! [`RegionPresenter::present`] only upserts. Hide / despawn is
//! [`RegionPresenter::cull`], driven by the present keep set. Bevy produce /
//! drain plugins live in [`runtime`].

mod runtime;

#[cfg(test)]
pub mod tests;

use crate::gen::{Id, SpatialIndex, Version};
use crate::lod_ref::LodRef;
use bevy::math::bounding::Aabb3d;
use std::collections::HashSet;

pub use crate::scene::{LodScene, LodSceneStatus};
pub use runtime::{
	drain_lod_present, drain_lod_present_cull, produce_lod_present_cull_regions,
	produce_lod_present_regions, LodPresentBudget, LodPresentCullBudget, LodPresentCullCursor,
	LodPresentCullPlugin, LodPresentCullRegion, LodPresentCullRegionPlugin, LodPresentKeepRegion,
	LodPresentPlugin, LodPresentQueue, LodPresentRegion, LodPresentRegionPlugin, LodPresentSystems,
};

/// Presents one layer (`T`) of a spatial index over a region.
///
/// [`RegionPresenter::present`] is the per-layer refresh entry point: wanted
/// ids are spawned or patched. Healing is part of presentation, not a
/// separate caller concern. Removal is [`RegionPresenter::cull`].
///
/// Two extension points sit above refresh:
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
	S: SpatialIndex<T>,
{
	/// The version this presenter last presented for the id, if any.
	fn presented_version(&self, id: Id) -> Option<Version>;

	/// Spawns or patches the id from the stored value. Implementations must
	/// record `version` so [`RegionPresenter::presented_version`] reflects it.
	fn handle(&mut self, id: Id, version: Version, value: &T, lod_ref: &LodRef);

	/// Hide a presented id that has left the present ring. Default is a no-op.
	/// [`RegionPresenter::cull`] despawns when the per-frame budget allows
	/// (same tick if there is a slot; otherwise hide now, despawn later).
	fn hide(&mut self, _id: Id) {}

	/// Whether [`RegionPresenter::hide`] has been applied and not yet removed.
	fn is_hidden(&self, _id: Id) -> bool {
		false
	}

	/// Presented ids this layer currently tracks (for cull).
	fn presented_ids(&self) -> Vec<Id>;

	/// Despawn presented ids that are not in `wanted`.
	///
	/// Contract: strictly removal. Refresh does not call this.
	fn remove_stale(&mut self, wanted: &HashSet<Id>);

	/// Presents and heals this layer (no removal).
	///
	/// Contract:
	///
	/// - wanted ids are spawned or patched (version / repair)
	/// - ids already presented at the current version are left alone
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

			let needs_present =
				self.presented_version(id).is_none_or(|presented| presented < version);

			let Some(instance) = spatial_index.get(id) else {
				continue;
			};

			if needs_present || self.needs_repair(region, id, version) {
				self.handle(id, version, instance, lod_ref);
			}
		}
	}

	/// Hide, then budget-despawn, presented ids that are not in `keep`
	/// (typically the last present-ring set).
	///
	/// Stale is keep-set membership. Each stale id with stored bounds is
	/// hidden immediately. Up to `despawn_budget` ids are removed this call
	/// (including first visit). Returns remaining budget.
	fn cull(&mut self, spatial_index: &S, keep: &HashSet<Id>, mut despawn_budget: u32) -> u32 {
		let stale: Vec<Id> = self
			.presented_ids()
			.into_iter()
			.filter(|id| !keep.contains(id))
			.filter(|id| spatial_index.get_bounds(*id).is_some())
			.collect();
		let mut to_remove = HashSet::new();
		for id in stale {
			if !self.is_hidden(id) {
				self.hide(id);
			}
			if despawn_budget > 0 {
				to_remove.insert(id);
				despawn_budget -= 1;
			}
		}
		if !to_remove.is_empty() {
			let wanted: HashSet<Id> =
				self.presented_ids().into_iter().filter(|id| !to_remove.contains(&id)).collect();
			self.remove_stale(&wanted);
		}
		despawn_budget
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
