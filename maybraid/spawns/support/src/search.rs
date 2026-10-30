//! Bounded vertical search columns. Generated heights define the interval;
//! collider geometry determines the accepted standing pose.

use crate::terrain::surface_or_hold_at;
use bevy::prelude::*;
use characters::LocomotionCapsule;
use durham::DurhamSurface;

use crate::{PlacementPolicy, SupportPolicy};

/// Extra outdoor search above the published / holding height so a collider
/// that sits higher than the generated sample can still be accepted.
pub const OUTDOOR_UPPER_MARGIN: f32 = 16.0;
/// Outdoor search below the generated sample (jersey plateaus, unpublished
/// cells, void). The probe stays bounded; it does not walk the whole map.
pub const OUTDOOR_LOWER_MARGIN: f32 = 32.0;
/// Interior interval above the intended floor, kept below the next storey.
pub const INTERIOR_UPPER_MARGIN: f32 = 1.25;
/// Interior interval below the intended floor.
pub const INTERIOR_LOWER_MARGIN: f32 = 0.75;

/// One destination's vertical search. Bounds are acceptable *support*
/// elevations, not capsule-center positions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacementSearch {
	pub xz: Vec2,
	pub min_surface_y: f32,
	pub max_surface_y: f32,
	pub hull: LocomotionCapsule,
	pub policy: PlacementPolicy,
	pub support: SupportPolicy,
}

impl PlacementSearch {
	pub fn new(
		xz: Vec2,
		min_surface_y: f32,
		max_surface_y: f32,
		hull: LocomotionCapsule,
		policy: PlacementPolicy,
	) -> Self {
		Self { xz, min_surface_y, max_surface_y, hull, policy, support: policy.support_policy() }
	}

	pub fn outdoor_at(surface: &DurhamSurface, xz: Vec2, hull: LocomotionCapsule) -> Self {
		let hint = surface_or_hold_at(surface, xz);
		Self::outdoor_hint(xz, hint, hull)
	}

	pub fn outdoor_hint(xz: Vec2, hint_y: f32, hull: LocomotionCapsule) -> Self {
		Self::new(
			xz,
			hint_y - OUTDOOR_LOWER_MARGIN,
			hint_y + OUTDOOR_UPPER_MARGIN,
			hull,
			PlacementPolicy::OutdoorGround,
		)
	}

	pub fn interior_at(floor: Vec3, hull: LocomotionCapsule) -> Self {
		Self::new(
			floor.xz(),
			floor.y - INTERIOR_LOWER_MARGIN,
			floor.y + INTERIOR_UPPER_MARGIN,
			hull,
			PlacementPolicy::InteriorFloor,
		)
	}

	pub fn airborne_at(position: Vec3, hull: LocomotionCapsule) -> Self {
		Self::new(position.xz(), position.y, position.y, hull, PlacementPolicy::Airborne)
	}

	pub const fn interior(self) -> bool {
		self.policy.interior()
	}

	pub fn bounds_valid(self) -> bool {
		self.xz.is_finite()
			&& self.min_surface_y.is_finite()
			&& self.max_surface_y.is_finite()
			&& self.hull.radius.is_finite()
			&& self.hull.length.is_finite()
			&& self.min_surface_y <= self.max_surface_y
			&& self.hull.radius > 0.0
			&& self.hull.length >= 0.0
	}

	/// Capsule center used to start the downward cast: above the upper
	/// support bound with room for the full hull.
	pub fn cast_origin(self) -> Vec3 {
		Vec3::new(
			self.xz.x,
			self.max_surface_y + self.hull.spawn_height() + START_CLEARANCE,
			self.xz.y,
		)
	}

	/// Distance that reaches a support at [`Self::min_surface_y`].
	pub fn cast_distance(self) -> f32 {
		let start = self.cast_origin().y;
		let min_center = self.min_surface_y + self.hull.spawn_height();
		(start - min_center + self.hull.half_height()).max(self.hull.half_height())
	}

	pub fn standing_at(self, surface_y: f32) -> Vec3 {
		Vec3::new(self.xz.x, surface_y + self.hull.spawn_height(), self.xz.y)
	}

	pub fn contains_surface(self, y: f32) -> bool {
		(self.min_surface_y..=self.max_surface_y).contains(&y)
	}

	/// Authored / generated support elevation that defined the column.
	pub fn hint_surface_y(self) -> f32 {
		match self.policy {
			PlacementPolicy::InteriorFloor => self.max_surface_y - INTERIOR_UPPER_MARGIN,
			PlacementPolicy::OutdoorGround => self.max_surface_y - OUTDOOR_UPPER_MARGIN,
			PlacementPolicy::Airborne => self.max_surface_y,
		}
	}
}

const START_CLEARANCE: f32 = 0.05;

/// Accepted standing pose after the probe. X/Z stay the search column.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlacementPose {
	pub position: Vec3,
	pub policy: PlacementPolicy,
	pub hull: LocomotionCapsule,
}

impl PlacementPose {
	pub const fn new(position: Vec3, policy: PlacementPolicy, hull: LocomotionCapsule) -> Self {
		Self { position, policy, hull }
	}

	pub const fn interior(self) -> bool {
		self.policy.interior()
	}
}
