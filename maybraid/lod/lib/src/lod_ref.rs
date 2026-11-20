//! Driver entities ([`LodNode`]) and ephemeral [`LodRef`] views.
//!
//! Not scene-specific: generation, presentation, and refresh all consume these.

mod node;

use std::cmp::Ordering;

use bevy::prelude::*;
use lod_cascade::Aabb3d;

pub use node::{
	collect_node_snapshots, collect_node_snapshots_into, lod_refs_from_snapshots, point_bounds,
	track_lod_nodes, LodNode, LodNodeBounds, LodNodePlugin, LodNodePose, LodNodeSnapshot,
	LodNodeSystems,
};

/// A component type to mark fine LOD.
/// This enables archetype filtering to ignore a lot of entities.
#[derive(Debug, Component)]
pub struct FineLod;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodRequest {
	/// Details with cells of the immediate neighborhood.
	Fine,
	/// Details outside the immediate neighborhood but that are still required for gameplay.
	Coarse,
	/// Details that are meant to be prepared for future use.
	Warm,
}

/// Borrowed view of a driver ([`LodNode`]) pose + that driver's extents.
#[derive(Debug, Clone, Copy)]
pub struct LodRef<'a> {
	/// The entity that triggered the LOD change.
	///
	/// For the most part, the developer will not use this.
	/// However, for more complex use cases without well-defined systems, this can be used to lookup the original entity
	/// and perform bespoke logic.
	pub entity: Entity,
	/// The previous transform of the entity that triggered the LOD change.
	pub previous_transform: &'a Transform,
	/// The transform of the entity that triggered the LOD change.
	pub current_transform: &'a Transform,
	/// Extents of the driver ([`LodNodeBounds`], or a point if the node is pointlike).
	///
	/// Not the host / scene AABB — host geometry is separate ([`crate::LodHostBounds`]).
	pub bounds: &'a Aabb3d,
}

impl LodRef<'_> {
	/// XZ distance² from this driver's current translation to `to`.
	pub fn xz_distance_squared_to(&self, to: Vec3) -> f32 {
		let delta = self.current_transform.translation - to;
		delta.x * delta.x + delta.z * delta.z
	}

	/// Nearest driver by the caller's `distance_squared` reduction.
	///
	/// Empty input is `None` — callers must not invent a viewer at the origin.
	pub fn nearest_by<'a>(
		refs: &'a [LodRef<'a>],
		distance_squared: impl Fn(&LodRef<'_>) -> f32,
	) -> Option<&'a LodRef<'a>> {
		refs.iter().min_by(|a, b| distance_squared(a).total_cmp(&distance_squared(b)))
	}

	/// Nearest driver to `to` in 3D.
	pub fn nearest_3d<'a>(refs: &'a [LodRef<'a>], to: Vec3) -> Option<&'a LodRef<'a>> {
		Self::nearest_by(refs, |lod_ref| lod_ref.current_transform.translation.distance_squared(to))
	}

	/// Nearest driver to `to`: min XZ, then min 3D for ties.
	///
	/// Use this where XZ already defines the consumer's rule (chunk admission,
	/// terrain XZ). 3D probes should call [`Self::nearest_3d`] or [`Self::nearest_by`].
	/// Empty input is `None` — callers must not invent a viewer at the origin.
	pub fn nearest<'a>(refs: &'a [LodRef<'a>], to: Vec3) -> Option<&'a LodRef<'a>> {
		refs.iter().min_by(|a, b| {
			compare_driver_distance(
				a.current_transform.translation,
				b.current_transform.translation,
				to,
			)
		})
	}
}

/// Min XZ, then min 3D. Shared by [`LodRef::nearest`] and [`LodNodePose::nearest`].
pub(crate) fn compare_driver_distance(a: Vec3, b: Vec3, to: Vec3) -> Ordering {
	let axz = (a.x - to.x).powi(2) + (a.z - to.z).powi(2);
	let bxz = (b.x - to.x).powi(2) + (b.z - to.z).powi(2);
	axz.total_cmp(&bxz)
		.then_with(|| a.distance_squared(to).total_cmp(&b.distance_squared(to)))
}

#[cfg(test)]
mod nearest_tests {
	use super::*;

	fn lod_ref_at<'a>(at: &'a Transform, bounds: &'a Aabb3d) -> LodRef<'a> {
		LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: at,
			current_transform: at,
			bounds,
		}
	}

	#[test]
	fn nearest_picks_the_closer_xz_driver() {
		let ground = Transform::from_translation(Vec3::new(0.0, 2.0, 0.0));
		let map = Transform::from_translation(Vec3::new(80.0, 420.0, 0.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let refs = [lod_ref_at(&ground, &bounds), lod_ref_at(&map, &bounds)];
		assert_eq!(
			LodRef::nearest(&refs, Vec3::new(1.0, 0.0, 0.0))
				.map(|r| r.current_transform.translation),
			Some(ground.translation)
		);
		assert_eq!(
			LodRef::nearest(&refs, Vec3::new(80.0, 0.0, 0.0))
				.map(|r| r.current_transform.translation),
			Some(map.translation)
		);
	}

	#[test]
	fn nearest_breaks_xz_ties_with_3d() {
		let ground = Transform::from_translation(Vec3::new(4.0, 2.0, 0.0));
		let map = Transform::from_translation(Vec3::new(4.0, 420.0, 0.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let refs = [lod_ref_at(&map, &bounds), lod_ref_at(&ground, &bounds)];
		assert_eq!(
			LodRef::nearest(&refs, Vec3::new(4.0, 0.0, 0.0))
				.map(|r| r.current_transform.translation),
			Some(ground.translation)
		);
	}

	#[test]
	fn nearest_is_none_without_refs() {
		assert!(LodRef::nearest(&[], Vec3::ONE).is_none());
		assert!(LodRef::nearest_by(&[], |_| 0.0).is_none());
	}

	#[test]
	fn nearest_by_3d_picks_the_closer_driver_when_xz_would_not() {
		let ground = Transform::from_translation(Vec3::new(5.0, 0.0, 0.0));
		let map = Transform::from_translation(Vec3::new(0.0, 420.0, 0.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let refs = [lod_ref_at(&ground, &bounds), lod_ref_at(&map, &bounds)];
		let host = Vec3::ZERO;
		assert_eq!(
			LodRef::nearest(&refs, host).map(|r| r.current_transform.translation),
			Some(map.translation),
			"XZ still prefers the overhead map ref"
		);
		assert_eq!(
			LodRef::nearest_3d(&refs, host).map(|r| r.current_transform.translation),
			Some(ground.translation),
			"3D prefers the 5 m ground ref over the 420 m map ref"
		);
	}
}
