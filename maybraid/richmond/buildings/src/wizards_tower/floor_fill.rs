//! Shared floor-slab layout for circular tower storeys.
//!
//! Squares off a circular footprint with four circle−inscribed-square caps, then
//! fills the inscribed square with rectangular slabs around a **centered** spire hole.

use bevy_math::Vec3;
use richmond_building_components::floors::Floor;
use richmond_building_components::Placed;

/// Inscribed-square half-extent as a fraction of outer radius (kit README).
pub const INSCRIBED_HALF_FRAC: f32 = 0.7;

/// Centered spire-hole half-extent as a fraction of outer radius.
pub const SPIRE_HALF_FRAC: f32 = 0.28;

/// Floor slab Y scale — thin relative to the authored kit thickness (\(Y \in [-0.2, 0.2]\)).
pub const FLOOR_SLAB_Y_SCALE: f32 = 0.2;

/// Outer ring wall height in meters (kit \(Y \in [0, 1]\)).
pub const WALL_HEIGHT_METERS: f32 = 3.0;

/// Four inscribed-square caps + four rectangular frame slabs around a centered spire hole.
pub fn squared_floor_with_spire_hole(
	center_xz: Vec3,
	radius: f32,
	spire_half: f32,
) -> ([Placed<Floor>; 4], [Placed<Floor>; 4]) {
	let radius = radius.max(1e-4);
	let inscribed_half = INSCRIBED_HALF_FRAC * radius;
	let spire_half = spire_half.clamp(1e-4, inscribed_half * 0.95);
	let ring_scale = Vec3::new(radius, FLOOR_SLAB_Y_SCALE, radius);

	let caps = [
		Placed::new(Floor::circle_inscribed_square(), center_xz, 0.0).with_scale(ring_scale),
		Placed::new(
			Floor::circle_inscribed_square(),
			center_xz,
			std::f32::consts::FRAC_PI_2,
		)
		.with_scale(ring_scale),
		Placed::new(Floor::circle_inscribed_square(), center_xz, std::f32::consts::PI)
			.with_scale(ring_scale),
		Placed::new(
			Floor::circle_inscribed_square(),
			center_xz,
			std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
		)
		.with_scale(ring_scale),
	];

	let cx = center_xz.x;
	let cz = center_xz.z;
	let y = center_xz.y;
	let inscribed_min_z = cz - inscribed_half;
	let inscribed_max_z = cz + inscribed_half;
	let inscribed_min_x = cx - inscribed_half;
	let inscribed_max_x = cx + inscribed_half;
	let spire_min_z = cz - spire_half;
	let spire_max_z = cz + spire_half;
	let spire_min_x = cx - spire_half;
	let spire_max_x = cx + spire_half;

	// Picture-frame rects around a hole centered on `center_xz`.
	let south = rect_slab(
		Vec3::new(cx, y, 0.5 * (inscribed_min_z + spire_min_z)),
		2.0 * inscribed_half,
		(spire_min_z - inscribed_min_z).max(0.0),
	);
	let north = rect_slab(
		Vec3::new(cx, y, 0.5 * (spire_max_z + inscribed_max_z)),
		2.0 * inscribed_half,
		(inscribed_max_z - spire_max_z).max(0.0),
	);
	let west = rect_slab(
		Vec3::new(0.5 * (inscribed_min_x + spire_min_x), y, cz),
		(spire_min_x - inscribed_min_x).max(0.0),
		2.0 * spire_half,
	);
	let east = rect_slab(
		Vec3::new(0.5 * (spire_max_x + inscribed_max_x), y, cz),
		(inscribed_max_x - spire_max_x).max(0.0),
		2.0 * spire_half,
	);

	(caps, [south, north, west, east])
}

fn rect_slab(center: Vec3, width_x: f32, depth_z: f32) -> Placed<Floor> {
	let width_x = width_x.max(1e-4);
	let depth_z = depth_z.max(1e-4);
	Placed::new(Floor::rectangle(), center, 0.0)
		.with_scale(Vec3::new(width_x, FLOOR_SLAB_Y_SCALE, depth_z))
}
