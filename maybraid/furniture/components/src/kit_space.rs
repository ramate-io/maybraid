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
//! After remap a box kit is \(X,Z \in [-1, 1]\), \(Y \in [0, 1]\) (origin on
//! the floor). [`BOX_KIT_TO_UNIT`] maps that into the Richmond unit slot cube
//! \([-0.5, 0.5]^3\). [`slab`] is floor-origin in that cube; compose
//! `kit_to_unit.compose_child(slab)` before applying the slot placement.

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

/// Box kit → unit slot: origin on the unit-cube floor, extents fill the cube.
pub const BOX_KIT_TO_UNIT: Placement = Placement {
	translation: Vec3::new(0.0, -0.5, 0.0),
	yaw: 0.0,
	pitch: 0.0,
	roll: 0.0,
	scale: Vec3::new(0.5, 1.0, 0.5),
};

/// Leg tube → unit slot (same floor origin).
pub const LEG_KIT_TO_UNIT: Placement = Placement {
	translation: Vec3::new(0.0, -0.5, 0.0),
	yaw: 0.0,
	pitch: 0.0,
	roll: 0.0,
	scale: Vec3::new(2.5, 1.0, 2.5),
};

/// Floor-origin slab in the unit slot cube: \(y\) from the floor (\(y=-0.5\))
/// as height fractions; \(x,z\) scale in \([0,1]\) of the slot plan.
///
/// Kit GLBs keep their authored floor origin; do not use a centered cuboid.
pub fn slab(xz_scale: f32, y0: f32, y1: f32) -> Placement {
	let y0 = y0.clamp(0.0, 1.0);
	let y1 = y1.clamp(y0 + 1e-4, 1.0);
	Placement {
		translation: Vec3::new(0.0, y0, 0.0),
		yaw: 0.0,
		pitch: 0.0,
		roll: 0.0,
		scale: Vec3::new(xz_scale.max(1e-4), (y1 - y0).max(1e-4), xz_scale.max(1e-4)),
	}
}

/// Slab with independent plan scales (counters).
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

/// Map a unit-slot slab onto an authored kit, then into `slot`.
///
/// Plan offsets on `unit` stay in unit-slot space. [`compose_child`] would
/// otherwise scale them by [`LEG_KIT_TO_UNIT`] (2.5) and throw chair legs
/// outside the seat.
pub fn place_kit(slot: Placement, kit_to_unit: Placement, unit: Placement) -> Placement {
	let plan_offset = Vec3::new(unit.translation.x, 0.0, unit.translation.z);
	let mut centered = unit;
	centered.translation.x = 0.0;
	centered.translation.z = 0.0;
	let mut local = kit_to_unit.compose_child(centered);
	local.translation += plan_offset;
	slot.compose_child(local)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn box_kit_to_unit_puts_the_floor_on_the_slot_floor() -> anyhow::Result<()> {
		let floor = BOX_KIT_TO_UNIT.compose_child(Placement::IDENTITY);
		if (floor.translation.y + 0.5).abs() > 1e-5 {
			return Err(anyhow::anyhow!("expected floor at y=-0.5, got {}", floor.translation.y));
		}
		if (floor.scale - Vec3::new(0.5, 1.0, 0.5)).length() > 1e-5 {
			return Err(anyhow::anyhow!("box kit scale should be (0.5, 1, 0.5)"));
		}
		Ok(())
	}

	#[test]
	fn plan_offsets_are_not_scaled_by_the_leg_kit() -> anyhow::Result<()> {
		let unit = shift(slab(0.12, 0.0, 0.42), Vec3::new(0.32, 0.0, 0.32));
		let placed = place_kit(Placement::IDENTITY, LEG_KIT_TO_UNIT, unit);
		if (placed.translation.x - 0.32).abs() > 1e-4 || (placed.translation.z - 0.32).abs() > 1e-4
		{
			return Err(anyhow::anyhow!(
				"leg plan offset should stay 0.32 in unit space, got ({}, {})",
				placed.translation.x,
				placed.translation.z
			));
		}
		Ok(())
	}

	#[test]
	fn floor_slab_keeps_the_kit_origin_on_the_band_floor() -> anyhow::Result<()> {
		let band = slab(0.92, 0.32, 0.92);
		if (band.translation.y - 0.32).abs() > 1e-5 {
			return Err(anyhow::anyhow!("slab origin should sit on y0"));
		}
		if (band.scale.y - 0.60).abs() > 1e-5 {
			return Err(anyhow::anyhow!("slab height should be y1-y0"));
		}
		Ok(())
	}
}
