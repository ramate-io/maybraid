//! Shared floor-slab helpers for circular tower storeys.
//!
//! Storey floors use a paneling [`ApproximatedCircle`] annulus (outer radius +
//! concentric spire clip). Rectangular floor kits remain for AABB rooms
//! (bedroom / tower room).

use richmond_building_components::panels::PanelStyle;

use crate::paneling::{ApproximatedCircle, DEFAULT_SEGMENTS};

/// Centered spire-hole radius as a fraction of outer radius.
pub const SPIRE_HALF_FRAC: f32 = 0.28;

/// Floor slab Y scale — thin relative to the authored kit thickness (\(Y \in [-0.2, 0.2]\)).
pub const FLOOR_SLAB_Y_SCALE: f32 = 0.2;

/// Outer ring wall height in meters (kit \(Y \in [0, 1]\)).
pub const WALL_HEIGHT_METERS: f32 = 3.0;

/// Rectangular floor kit half-extent along \(X\) and \(Z\).
pub const RECT_HALF_EXTENT: f32 = 1.0;

/// Segment count for the n-gon floor disk / annulus.
pub const FLOOR_SEGMENTS: u32 = DEFAULT_SEGMENTS;

/// Horizontal rough-stone annulus: outer `radius`, hole at `spire_radius`.
pub fn circular_floor_with_spire_hole(
	center_xz: bevy_math::Vec3,
	radius: f32,
	spire_radius: f32,
) -> ApproximatedCircle {
	let radius = radius.max(1e-4);
	let spire_radius = spire_radius.clamp(1e-4, radius * 0.95);
	ApproximatedCircle::horizontal(
		PanelStyle::RoughStonework,
		center_xz,
		radius,
		FLOOR_SEGMENTS,
		Some(spire_radius),
	)
}
