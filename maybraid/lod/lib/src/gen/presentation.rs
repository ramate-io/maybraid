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
use crate::lod_cull::LodSceneCulls;
use crate::lod_level::LodSceneLevel;
use crate::lod_ref::LodRef;
use bevy::{math::bounding::Aabb3d, scene::Scene};
use std::collections::HashSet;

/// Whether the presented LOD selection should be updated for this [`LodRef`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodSceneStatus {
	/// Desired level changed; payload is the new level.
	Changed(LodSceneLevel),
	Unchanged,
}

pub trait LodScene {
	/// Desired presentation level for `lod_ref.current_*`. Must be cheap — no scene build.
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		let _ = lod_ref;
		LodSceneLevel::High
	}

	/// Whether the presented LOD selection should change for this [`LodRef`].
	///
	/// Must be cheap — no scene build. Implementors that care about camera motion
	/// should compare their own previous/current banding here. Default always
	/// reports a change to [`LodSceneLevel::High`].
	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let _ = lod_ref;
		LodSceneStatus::Changed(LodSceneLevel::High)
	}

	/// Inactive [`crate::LodLevelRoot`]s this scene is willing to despawn.
	///
	/// Must be cheap — no scene build. `current` is the host's desired
	/// [`LodSceneLevel`]. Default keeps every root warm ([`LodSceneCulls::None`]).
	/// Prefer [`crate::cull_non_adjacent_bands`] (or [`crate::cull_offset_bands`])
	/// when memory matters; “not current” alone is not a cull reason. Culling the
	/// immediately adjacent band is usually a bad idea — respawning that root on
	/// the way back is expensive; keep it warm unless you are well into the
	/// current band.
	///
	/// Host GC never despawns the host's current/desired level even if listed.
	/// After a despawn, Sync + Fulfill spawn the desired level again via
	/// [`Self::scene_with_level`] — there is no separate rebuild path.
	fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		let _ = (lod_ref, current);
		LodSceneCulls::None
	}

	/// Scene for one LOD level root (primary implementation target).
	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static;

	/// Scene for the **current** LOD selection only (first present / non-host path).
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		self.scene_with_level(lod_ref, self.scene_lod_level(lod_ref))
	}
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

	/// LOD-only update: set the presented host's [`LodSceneLevel`] without rebuilding.
	///
	/// Default falls back to [`RegionPresenter::handle`] with `scene_with_lod` for
	/// presenters that do not track host entities yet.
	fn set_lod_level(&mut self, id: Id, level: LodSceneLevel, spatial_index: &S, lod_ref: &LodRef) {
		let _ = level;
		if let Some(version) = spatial_index.version(id) {
			if let Some(instance) = spatial_index.get(id) {
				self.handle(id, version, instance.scene_with_lod(lod_ref), lod_ref);
			}
		}
	}

	/// Removes presented ids within the region that are not in `wanted`.
	///
	/// Contract: strictly removal. [`RegionPresenter::present`] invokes this
	/// after the handle pass, so every wanted id has already been presented.
	fn remove_stale(&mut self, region: Aabb3d, wanted: &HashSet<Id>);

	/// Presents and heals this layer.
	///
	/// Contract:
	///
	/// - wanted ids are spawned or patched first (version / repair)
	/// - LOD-only changes call [`RegionPresenter::set_lod_level`] (not a full rebuild)
	/// - stale ids are removed after
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
				self.handle(id, version, instance.scene_with_lod(lod_ref), lod_ref);
				continue;
			}

			if let LodSceneStatus::Changed(level) = instance.scene_lod_status(lod_ref) {
				self.set_lod_level(id, level, spatial_index, lod_ref);
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
