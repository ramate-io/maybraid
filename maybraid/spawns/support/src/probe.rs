//! Bounded downward shape cast + standing-capsule clearance.
//!
//! Interior and outdoor share this probe. Generated heights define the
//! search column; collider geometry determines the accepted pose. The
//! first ineligible hit fails — the cast does not punch through a wall
//! or roof just because it is unsuitable for standing.

use avian3d::prelude::{Collider, ShapeCastConfig, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use characters::{CharacterLocomotion, LocomotionCapsule};

use crate::column_presentation_ready;
use crate::{PlacementPolicy, PlacementSupportHit, SupportProbeFailure};
use crate::{PlacementPose, PlacementSearch};

const MAX_HITS: u32 = 8;
const PROBE_HALF_HEIGHT: f32 = 0.02;
const START_LIFT: f32 = 0.05;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SupportProbeOutcome {
	Waiting,
	Ready { pose: PlacementPose, support: Option<PlacementSupportHit> },
	Failed(SupportProbeFailure),
}

pub fn probe_search(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	search: PlacementSearch,
) -> SupportProbeOutcome {
	probe_search_with_locomotion(spatial, filter, search, &CharacterLocomotion::default())
}

/// Apply the same configured uphill limit used by character movement.
pub fn probe_search_with_locomotion(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	search: PlacementSearch,
	locomotion: &CharacterLocomotion,
) -> SupportProbeOutcome {
	if !search.bounds_valid() {
		return SupportProbeOutcome::Failed(SupportProbeFailure::InvalidBounds);
	}
	if search.policy == PlacementPolicy::Airborne {
		let pose = PlacementPose::new(
			Vec3::new(search.xz.x, search.max_surface_y, search.xz.y),
			search.policy,
			search.hull,
		);
		return if clearance_blocked(spatial, filter, pose.position, search.hull, None) {
			SupportProbeOutcome::Failed(SupportProbeFailure::Clearance)
		} else {
			SupportProbeOutcome::Ready { pose, support: None }
		};
	}
	if !column_presentation_ready(spatial, filter, search) {
		return SupportProbeOutcome::Waiting;
	}
	cast_column(spatial, filter, search, locomotion)
}

fn cast_column(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	search: PlacementSearch,
	locomotion: &CharacterLocomotion,
) -> SupportProbeOutcome {
	let origin = support_origin(search);
	if start_overlaps(spatial, filter, origin, search.hull) {
		return SupportProbeOutcome::Failed(SupportProbeFailure::Clearance);
	}
	let probe = support_probe(search.hull);
	let config = ShapeCastConfig::from_max_distance(search.cast_distance().max(0.1));
	let mut hits =
		spatial.shape_hits(&probe, origin, Quat::IDENTITY, Dir3::NEG_Y, MAX_HITS, &config, filter);
	hits.sort_by(|left, right| left.distance.total_cmp(&right.distance));
	let Some(hit) = hits.into_iter().next() else {
		return SupportProbeOutcome::Failed(SupportProbeFailure::NoEligibleSupport);
	};
	if !search.support.accepts_normal_with_locomotion(hit.normal1, locomotion)
		|| !search.contains_surface(hit.point1.y)
	{
		return SupportProbeOutcome::Failed(
			if search.support.accepts_normal_with_locomotion(hit.normal1, locomotion) {
				SupportProbeFailure::NoEligibleSupport
			} else {
				SupportProbeFailure::Clearance
			},
		);
	}
	let standing = search.standing_at(hit.point1.y);
	if (standing.xz() - search.xz).length() > 1e-3 {
		return SupportProbeOutcome::Failed(SupportProbeFailure::Placement);
	}
	if clearance_blocked(spatial, filter, standing, search.hull, Some(hit.entity)) {
		return SupportProbeOutcome::Failed(SupportProbeFailure::Clearance);
	}
	SupportProbeOutcome::Ready {
		pose: PlacementPose::new(standing, search.policy, search.hull),
		support: Some(PlacementSupportHit::new(hit.entity, hit.point1, hit.normal1)),
	}
}

fn support_origin(search: PlacementSearch) -> Vec3 {
	Vec3::new(search.xz.x, search.max_surface_y + START_LIFT, search.xz.y)
}

fn start_overlaps(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	origin: Vec3,
	hull: LocomotionCapsule,
) -> bool {
	!spatial
		.shape_intersections(&support_probe(hull), origin, Quat::IDENTITY, filter)
		.is_empty()
		// Parry's convex intersection test can miss deep containment when
		// the support disk is much smaller than its enclosing solid.
		|| !spatial.point_intersections(origin, filter).is_empty()
}

fn support_probe(hull: LocomotionCapsule) -> Collider {
	Collider::cylinder((hull.radius * 0.5).max(0.08), PROBE_HALF_HEIGHT * 2.0)
}

fn hull_collider(hull: LocomotionCapsule) -> Collider {
	Collider::capsule(hull.radius.max(0.01), hull.length.max(0.01))
}

pub fn clearance_blocked(
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
	standing: Vec3,
	hull: LocomotionCapsule,
	support: Option<Entity>,
) -> bool {
	spatial
		.shape_intersections(&hull_collider(hull), standing, Quat::IDENTITY, filter)
		.into_iter()
		.any(|entity| support != Some(entity))
		|| spatial
			.point_intersections(standing, filter)
			.into_iter()
			.any(|entity| support != Some(entity))
}

/// Re-run the exact probe geometry only for a changed, debug-enabled failure.
/// The collision algorithm and its acceptance decision remain in `probe_search`.
#[derive(Debug)]
pub(crate) struct ProbeDiagnostics {
	pub(crate) origin: Vec3,
	pub(crate) start_overlaps: Vec<Entity>,
	pub(crate) start_containing: Vec<Entity>,
	pub(crate) first_hit: Option<PlacementSupportHit>,
	pub(crate) final_position: Option<Vec3>,
	pub(crate) final_clearance: Vec<Entity>,
	pub(crate) final_containing: Vec<Entity>,
}

impl ProbeDiagnostics {
	pub(crate) fn collect(
		spatial: &SpatialQuery,
		filter: &SpatialQueryFilter,
		search: PlacementSearch,
		minimum_center_y: Option<f32>,
	) -> Self {
		let origin = support_origin(search);
		let start_overlaps = spatial.shape_intersections(
			&support_probe(search.hull),
			origin,
			Quat::IDENTITY,
			filter,
		);
		let start_containing = spatial.point_intersections(origin, filter);
		let config = ShapeCastConfig::from_max_distance(search.cast_distance().max(0.1));
		let mut hits = spatial.shape_hits(
			&support_probe(search.hull),
			origin,
			Quat::IDENTITY,
			Dir3::NEG_Y,
			MAX_HITS,
			&config,
			filter,
		);
		hits.sort_by(|left, right| left.distance.total_cmp(&right.distance));
		let first_hit = if search.policy == PlacementPolicy::Airborne {
			None
		} else {
			hits.first()
				.map(|hit| PlacementSupportHit::new(hit.entity, hit.point1, hit.normal1))
		};
		let final_position = if search.policy == PlacementPolicy::Airborne {
			Some(Vec3::new(search.xz.x, search.max_surface_y, search.xz.y))
		} else {
			first_hit.map(|hit| search.standing_at(hit.point.y))
		}
		.map(|mut position| {
			if let Some(minimum) = minimum_center_y {
				position.y = position.y.max(minimum);
			}
			position
		});
		let final_clearance = final_position
			.map(|standing| {
				spatial
					.shape_intersections(
						&hull_collider(search.hull),
						standing,
						Quat::IDENTITY,
						filter,
					)
					.into_iter()
					.filter(|entity| first_hit.map(|hit| hit.entity) != Some(*entity))
					.collect()
			})
			.unwrap_or_default();
		let final_containing = final_position
			.map(|standing| {
				spatial
					.point_intersections(standing, filter)
					.into_iter()
					.filter(|entity| first_hit.map(|hit| hit.entity) != Some(*entity))
					.collect()
			})
			.unwrap_or_default();
		Self {
			origin,
			start_overlaps,
			start_containing,
			first_hit,
			final_position,
			final_clearance,
			final_containing,
		}
	}

	pub(crate) fn entities(&self) -> Vec<Entity> {
		let mut entities = self.start_overlaps.clone();
		entities.extend(self.start_containing.iter().copied());
		entities.extend(self.final_containing.iter().copied());
		entities.extend(self.final_clearance.iter().copied());
		entities.extend(self.first_hit.map(|hit| hit.entity));
		entities.sort_by_key(|entity| entity.to_bits());
		entities.dedup();
		entities
	}
}
