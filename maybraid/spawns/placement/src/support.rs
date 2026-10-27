//! Relevant collision-presentation readiness for one search column.
//!
//! This crate observes existing collider presentation. It does not introduce
//! a spatial index or a generic generation framework. A collider below the
//! column is not enough while the expected floor inside the interval is
//! still loading.

use avian3d::prelude::{Collider, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;

use crate::request::PlacementPolicy;
use crate::terrain::PlacementSearch;

/// Whether the destination column has usable collision presentation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SupportReadiness {
	pub height_ready: bool,
	pub collider_ready: bool,
}

impl SupportReadiness {
	pub const fn new(height_ready: bool, collider_ready: bool) -> Self {
		Self { height_ready, collider_ready }
	}

	pub const fn ready(self) -> bool {
		self.height_ready && self.collider_ready
	}

	pub const fn pending(self) -> bool {
		!self.ready()
	}

	pub const fn presented() -> Self {
		Self::new(true, true)
	}

	pub const fn waiting() -> Self {
		Self::new(false, false)
	}

	pub fn for_search(search: PlacementSearch, presented: bool) -> Self {
		if search.policy.waits_for_support() {
			Self::new(presented, presented)
		} else {
			Self::presented()
		}
	}

	pub const fn for_policy(policy: PlacementPolicy, presented: bool) -> Self {
		if policy.waits_for_support() {
			Self::new(presented, presented)
		} else {
			Self::presented()
		}
	}
}

/// Snapshot written by tests. Production polling uses per-request
/// readiness on [`crate::PendingPlacement`].
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlacementSupport {
	pub readiness: SupportReadiness,
}

impl PlacementSupport {
	pub const fn ready() -> Self {
		Self { readiness: SupportReadiness::presented() }
	}

	pub const fn waiting() -> Self {
		Self { readiness: SupportReadiness::waiting() }
	}

	pub fn observe(&mut self, readiness: SupportReadiness) {
		self.readiness = readiness;
	}

	pub const fn is_ready(self) -> bool {
		self.readiness.ready()
	}
}

/// Cuboid covering acceptable support elevations at the destination XZ.
pub fn column_volume(search: PlacementSearch) -> (Collider, Vec3) {
	let radius = search.hull.radius.max(0.05);
	let half_height = ((search.max_surface_y - search.min_surface_y) * 0.5).max(0.05);
	let center =
		Vec3::new(search.xz.x, (search.min_surface_y + search.max_surface_y) * 0.5, search.xz.y);
	(Collider::cuboid(radius, half_height, radius), center)
}

/// True when a Fixed collider intersects the destination column. Terrain
/// under an interior storey does not satisfy this: it sits outside the
/// interval.
pub fn column_presentation_ready(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	search: PlacementSearch,
) -> bool {
	if !search.policy.waits_for_support() {
		return true;
	}
	let (volume, center) = column_volume(search);
	!spatial.shape_intersections(&volume, center, Quat::IDENTITY, filter).is_empty()
}
