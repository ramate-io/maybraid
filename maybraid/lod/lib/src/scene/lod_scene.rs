//! [`LodScene`] — how a host builds and selects LOD presentation.

use bevy::ecs::component::Component;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::scene::prelude::{bsn, template_value, Scene};

use crate::lod_ref::LodRef;

use super::chunk::SceneChunk;
use super::cull::LodSceneCulls;
use super::host::lod_host_scene_pending;
use super::level::LodSceneLevel;

/// Whether the presented LOD selection should be updated for this [`LodRef`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodSceneStatus {
	/// Desired level changed; payload is the new level.
	Changed(LodSceneLevel),
	Unchanged,
}

/// Runtime presentation for one LOD host type.
pub trait LodScene {
	/// Desired presentation level for `lod_ref.current_*`. Must be cheap — no scene build.
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		let _ = lod_ref;
		LodSceneLevel::High
	}

	/// Desired level from multiple driver [`LodRef`]s (e.g. several [`crate::lod_ref::LodNode`]s).
	///
	/// Default: max of [`Self::scene_lod_level`] over `lod_refs`. Empty input means
	/// no driver this frame → [`LodSceneLevel::UltraLow`].
	fn scene_lod_level_from_levels(&self, lod_refs: &[LodRef]) -> LodSceneLevel {
		lod_refs
			.iter()
			.map(|lod_ref| self.scene_lod_level(lod_ref))
			.max()
			.unwrap_or(LodSceneLevel::UltraLow)
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

	/// Inactive [`super::host::LodLevelRoot`]s this scene is willing to despawn.
	///
	/// Must be cheap — no scene build. `current` is the host's desired
	/// [`LodSceneLevel`]. Default keeps every root warm ([`LodSceneCulls::None`]).
	/// Prefer [`super::cull_non_adjacent_bands`] (or [`super::cull_offset_bands`])
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

	/// Incremental composition for one LOD level root.
	///
	/// Default wraps [`Self::scene_with_level`] as a single
	/// [`SceneChunk::primitive`]. Override to split expensive levels into
	/// weighted sub-chunks for [`super::chunk_fulfill`] drain.
	///
	/// Note: the default still builds the full scene up front; hitch reduction
	/// requires an override (or future lazy chunk nodes).
	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	/// Scene for the **current** LOD selection only (first present / non-host path).
	///
	/// Default builds level content only. Typed ECS hosts that nest under a parent
	/// should prefer [`Self::host`] (pending shell + [`Self::host_contents`]).
	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		self.scene_with_level(lod_ref, self.scene_lod_level(lod_ref))
	}

	/// Domain identity on a new host entity (typed `Self`, markers, local transform, …).
	///
	/// **Do not** stamp LOD scaffolding here (`LodSceneHost`, roots bag, spawn request).
	/// [`Self::host`] wraps this in [`super::host::lod_host_scene_pending`].
	fn host_contents(&self, lod_ref: &LodRef) -> impl Scene + 'static
	where
		Self: Component + Clone + Default + Unpin + Sized,
	{
		let _ = lod_ref;
		let host = self.clone();
		bsn! {
			template_value(host)
		}
	}

	/// Spawn a pending [`super::host::LodSceneHost`] with [`Self::host_contents`].
	///
	/// Core owns the empty [`super::host::LodLevelRoots`] bag and
	/// [`super::host::LodLevelSpawnRequest`]. Domain types override
	/// [`Self::host_contents`] only.
	fn host(&self, lod_ref: &LodRef) -> impl Scene + 'static
	where
		Self: Component + Clone + Default + Unpin + Sized,
	{
		(
			lod_host_scene_pending(self.scene_lod_level(lod_ref), self.scene_bounds()),
			self.host_contents(lod_ref),
		)
	}

	/// Local-space structural AABB for this host (indexing / volume materialization).
	///
	/// Relative to the host [`bevy::prelude::Transform`]. Cached on the host as
	/// [`crate::LodHostBounds`] by [`crate::PatchSceneBounds`]. Default is a
	/// unit box at the local origin.
	fn scene_bounds(&self) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE)
	}
}

impl<T: LodScene + Send + Sync + 'static> LodScene for std::sync::Arc<T> {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		(**self).scene_lod_level(lod_ref)
	}

	fn scene_lod_level_from_levels(&self, lod_refs: &[LodRef]) -> LodSceneLevel {
		(**self).scene_lod_level_from_levels(lod_refs)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		(**self).scene_lod_status(lod_ref)
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		(**self).scene_lod_culls(lod_ref, current)
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		(**self).scene_with_level(lod_ref, level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		(**self).scene_chunks_with_level(lod_ref, level)
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		(**self).scene_with_lod(lod_ref)
	}

	fn scene_bounds(&self) -> Aabb3d {
		(**self).scene_bounds()
	}
}
