//! Blender \(Z\)-up furniture kits → engine \(Y\)-up slot space.
//!
//! Authored (Blender): most parts \(X,Y \in [-1, 1]\), \(Z \in [0, 1]\). Chair
//! legs are tubes \(X,Y \in [-0.2, 0.2]\), \(Z \in [0, 1]\). Chair back fills
//! authored \(+Y\).
//!
//! Remap \((X,Y,Z)_{\text{Blender}} \mapsto (X,Z,Y)_{\text{engine}}\):
//! - plan \(X\) stays \(X\)
//! - authored \(+Y\) (back / head) becomes engine \(+Z\)
//! - authored \(+Z\) (up) becomes engine \(+Y\)
//!
//! A unit slot is the centered cube \([-0.5, 0.5]^3\) used by
//! [`richmond_building_components::FurnitureNode`] placements. Part placements
//! below are relative to that cube; [`Placement::compose_child`] puts them in
//! the slot. Future GLBs at [`crate::assets`] should occupy the remapped AABB
//! so they drop in for the procedural cuboids.

use bevy::math::Vec3;
use richmond_building_components::Placement;

/// Authored box kit after remap: plan \(X,Z \in [-1, 1]\), \(Y \in [0, 1]\).
pub const BOX_KIT_MIN: Vec3 = Vec3::new(-1.0, 0.0, -1.0);
/// See [`BOX_KIT_MIN`].
pub const BOX_KIT_MAX: Vec3 = Vec3::new(1.0, 1.0, 1.0);

/// Authored chair-leg tube after remap: plan \(X,Z \in [-0.2, 0.2]\), \(Y \in [0, 1]\).
pub const LEG_KIT_MIN: Vec3 = Vec3::new(-0.2, 0.0, -0.2);
/// See [`LEG_KIT_MIN`].
pub const LEG_KIT_MAX: Vec3 = Vec3::new(0.2, 1.0, 0.2);

/// Slab inside the unit slot cube: \(x,z\) scale in \([0,1]\) of the slot plan,
/// \(y\) from the floor (\(y=-0.5\)) as a height fraction of the slot.
pub fn slab(xz_scale: f32, y0: f32, y1: f32) -> Placement {
	let y0 = y0.clamp(0.0, 1.0);
	let y1 = y1.clamp(y0 + 1e-4, 1.0);
	let h = y1 - y0;
	Placement {
		translation: Vec3::new(0.0, -0.5 + y0 + 0.5 * h, 0.0),
		yaw: 0.0,
		pitch: 0.0,
		roll: 0.0,
		scale: Vec3::new(xz_scale.max(1e-4), h.max(1e-4), xz_scale.max(1e-4)),
	}
}

/// Slab with independent plan scales (counters / backs).
pub fn slab_xz(scale_x: f32, scale_z: f32, y0: f32, y1: f32) -> Placement {
	let mut p = slab(1.0, y0, y1);
	p.scale.x = scale_x.max(1e-4);
	p.scale.z = scale_z.max(1e-4);
	p
}

/// Offset a placement in unit-slot coordinates (fractions of the parent cube).
pub fn shift(mut placement: Placement, delta: Vec3) -> Placement {
	placement.translation += delta;
	placement
}
