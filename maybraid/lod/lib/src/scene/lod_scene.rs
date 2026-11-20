//! [`SemanticLodScene`] (main-world spawn) and [`VisualLodScene`] (per-view draw).
//!
//! [`LodScene`] is a compatibility alias for [`SemanticLodScene`]. Grove and
//! building hosts keep `impl LodScene`. Visual work must not return a
//! [`SceneChunk`] of `Box<dyn Scene>` — that is still the exclusive spawn path.

use bevy::ecs::component::Component;
use bevy::math::Vec3;
use bevy::scene::prelude::{bsn, Scene};
use bevy::shape::Aabb3d;

use crate::lod_ref::LodRef;

use super::chunk::{SceneChunk, VisualLodPrimitive, VisualSceneChunk};
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

/// Semantic (main-world) presentation for one LOD host type.
///
/// Builds Bevy [`Scene`]s and [`SceneChunk`]s that
/// [`crate::drain_chunk_lod_fulfill`] spawns. Use this for physics, interaction,
/// High capsules, and anything that must exist as ECS. Graphical UltraLow ↔
/// Medium belongs on [`VisualLodScene`].
pub trait SemanticLodScene {
	/// Desired presentation level for `lod_ref.current_*`. Must be cheap — no scene build.
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		let _ = lod_ref;
		LodSceneLevel::High
	}

	/// Desired level from the driver nearest `to` (host world position).
	///
	/// Picks [`LodRef::nearest`], then [`Self::scene_lod_level`] of that one ref.
	/// Empty input is [`LodSceneLevel::UltraLow`] — no invented viewer at the origin.
	fn scene_lod_level_from_nearest(&self, lod_refs: &[LodRef], to: Vec3) -> LodSceneLevel {
		LodRef::nearest(lod_refs, to)
			.map(|lod_ref| self.scene_lod_level(lod_ref))
			.unwrap_or(LodSceneLevel::UltraLow)
	}

	/// Desired level from multiple driver [`LodRef`]s without a host point.
	///
	/// Prefer [`Self::scene_lod_level_from_nearest`] at call sites that have a
	/// host. Default nearest-to-origin is only for tests without a footprint.
	fn scene_lod_level_from_levels(&self, lod_refs: &[LodRef]) -> LodSceneLevel {
		self.scene_lod_level_from_nearest(lod_refs, Vec3::ZERO)
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
	/// Region cull produce may **lower** a stale desired when
	/// [`Self::scene_lod_level`] is farther than the stored value (camera left
	/// the refresh bullseye). After a despawn, Sync + Fulfill spawn the desired
	/// level again via [`Self::scene_with_level`] — there is no separate rebuild
	/// path. Begin will not start a root that this policy would cull.
	fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		let _ = (lod_ref, current);
		LodSceneCulls::None
	}

	/// Whether every driver would despawn `root` at `current`.
	///
	/// Union of interest: a root stays if any [`LodRef`] still wants it.
	fn scene_culls_root_from_levels(
		&self,
		lod_refs: &[LodRef],
		current: LodSceneLevel,
		root: LodSceneLevel,
	) -> bool {
		LodSceneCulls::all_cull(
			lod_refs.iter().map(|lod_ref| self.scene_lod_culls(lod_ref, current)),
			root,
		)
	}

	/// Scene for one LOD level root (primary implementation target).
	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static;

	/// Incremental composition for one **semantic** LOD level root.
	///
	/// Default wraps [`Self::scene_with_level`] as a single
	/// [`SceneChunk::primitive`]. Override to split expensive levels into
	/// weighted sub-chunks for [`super::chunk_fulfill`] drain.
	///
	/// Do not put packed / per-view forest here. That is [`VisualLodScene`].
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
			~{host}
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

/// Compatibility name for [`SemanticLodScene`].
pub use SemanticLodScene as LodScene;

impl<T: SemanticLodScene + Send + Sync + 'static> SemanticLodScene for std::sync::Arc<T> {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		(**self).scene_lod_level(lod_ref)
	}

	fn scene_lod_level_from_nearest(&self, lod_refs: &[LodRef], to: Vec3) -> LodSceneLevel {
		(**self).scene_lod_level_from_nearest(lod_refs, to)
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

	fn scene_culls_root_from_levels(
		&self,
		lod_refs: &[LodRef],
		current: LodSceneLevel,
		root: LodSceneLevel,
	) -> bool {
		(**self).scene_culls_root_from_levels(lod_refs, current, root)
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

/// Per-view render LOD. Does not build a Bevy [`Scene`].
///
/// Camera motion that only changes triangles / impostors / instance density
/// should implement this — not insert `LodLevelSpawnRequest` or rebuild ECS
/// roots. [#667](https://github.com/ramate-io/maybraid/issues/667) fills
/// [`VisualLodPrimitive`]. There is no consume plugin on this branch.
pub trait VisualLodScene {
	/// Packed / impostor / instance tree for `level`. Must be cheap to plan;
	/// do not `spawn_scene`.
	fn visual_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> VisualSceneChunk {
		let _ = lod_ref;
		VisualSceneChunk::primitive(VisualLodPrimitive { level })
	}
}

impl<T: VisualLodScene + Send + Sync + 'static> VisualLodScene for std::sync::Arc<T> {
	fn visual_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> VisualSceneChunk {
		(**self).visual_chunks_with_level(lod_ref, level)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::lod_ref::LodRef;
	use bevy::prelude::{Entity, Transform};

	struct StubVisual;

	impl VisualLodScene for StubVisual {}

	#[test]
	fn visual_default_is_a_level_stub() {
		let camera = Transform::default();
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &camera,
			current_transform: &camera,
			bounds: &bounds,
		};
		let chunk = StubVisual.visual_chunks_with_level(&lod_ref, LodSceneLevel::Medium);
		assert_eq!(chunk.total_primitives(), 1);
		let prims = chunk.into_primitives();
		assert_eq!(prims[0].1.level, LodSceneLevel::Medium);
	}

	struct HeightScene;

	impl SemanticLodScene for HeightScene {
		fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
			if lod_ref.current_transform.translation.y < 10.0 {
				LodSceneLevel::High
			} else {
				LodSceneLevel::UltraLow
			}
		}

		fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
			let _ = current;
			crate::cull_non_adjacent_bands(self.scene_lod_level(lod_ref))
		}

		fn scene_with_level(&self, _: &LodRef, _: LodSceneLevel) -> impl Scene + 'static {}
	}

	fn lod_ref_at<'a>(prev: &'a Transform, curr: &'a Transform, bounds: &'a Aabb3d) -> LodRef<'a> {
		LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: prev,
			current_transform: curr,
			bounds,
		}
	}

	#[test]
	fn scene_lod_level_from_nearest_picks_the_closer_ref_per_host() {
		let ground = Transform::from_translation(Vec3::new(0.0, 2.0, 0.0));
		let map = Transform::from_translation(Vec3::new(80.0, 420.0, 0.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let refs = [lod_ref_at(&ground, &ground, &bounds), lod_ref_at(&map, &map, &bounds)];
		assert_eq!(
			HeightScene.scene_lod_level_from_nearest(&refs, Vec3::new(1.0, 0.0, 0.0)),
			LodSceneLevel::High
		);
		assert_eq!(
			HeightScene.scene_lod_level_from_nearest(&refs, Vec3::new(80.0, 0.0, 0.0)),
			LodSceneLevel::UltraLow
		);
		assert!(!HeightScene.scene_culls_root_from_levels(
			&refs,
			LodSceneLevel::High,
			LodSceneLevel::High
		));
		assert_eq!(
			HeightScene.scene_lod_level_from_nearest(&[], Vec3::ONE),
			LodSceneLevel::UltraLow,
			"empty refs must not invent a viewer at the origin"
		);
	}

	struct DistanceScene {
		host: Vec3,
	}

	impl SemanticLodScene for DistanceScene {
		fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
			let distance = lod_ref.current_transform.translation.distance(self.host);
			if distance < 50.0 {
				LodSceneLevel::High
			} else if distance < 200.0 {
				LodSceneLevel::Medium
			} else {
				LodSceneLevel::Low
			}
		}

		fn scene_with_level(&self, _: &LodRef, _: LodSceneLevel) -> impl Scene + 'static {}
	}

	#[test]
	fn nearest_breaks_same_band_ties_with_the_closer_driver() {
		let closer = Transform::from_translation(Vec3::new(8.0, 2.0, 0.0));
		let farther = Transform::from_translation(Vec3::new(30.0, 2.0, 0.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let refs = [lod_ref_at(&farther, &farther, &bounds), lod_ref_at(&closer, &closer, &bounds)];
		let host = Vec3::ZERO;
		let scene = DistanceScene { host };
		assert_eq!(scene.scene_lod_level(&refs[0]), LodSceneLevel::High);
		assert_eq!(scene.scene_lod_level(&refs[1]), LodSceneLevel::High);
		assert_eq!(
			LodRef::nearest(&refs, host).map(|r| r.current_transform.translation),
			Some(closer.translation)
		);
		assert_eq!(scene.scene_lod_level_from_nearest(&refs, host), LodSceneLevel::High);
	}
}
