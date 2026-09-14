//! Chair: four legs, seat band, back on remapped \(+Z\).

use bevy::math::Vec3;

use crate::palette::{cloth, wood};
use crate::Assembly;
use furniture_components::{shift, slab, PartKind, PlacedPart};
use richmond_building_components::FurnitureGeometry;

/// Seat band height in the unit slot (fractions of slot height).
pub const SEAT_Y0: f32 = 0.42;
/// See [`SEAT_Y0`].
pub const SEAT_Y1: f32 = 0.54;
/// Authored leg width over authored seat width (`0.4 / 2.0`).
pub const LEG_XZ: f32 = 0.20;
/// Inset from the slot rim so a 0.20-wide post flushes the seat corner.
pub const LEG_INSET: f32 = 0.40;

/// Finish-only knobs. Topology does not change with [`Self::finish_seed`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChairParams {
	pub finish_seed: u64,
}

impl ChairParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num) }
	}

	pub fn build(&self) -> Chair {
		Chair::from_params(*self)
	}
}

/// Seat is the slot plan band; legs drop to the floor; back is the box kit in
/// the upper band (authored \(+Y\) is engine \(+Z\)).
#[derive(Clone, Debug, PartialEq)]
pub struct Chair {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Chair {
	pub fn from_params(params: ChairParams) -> Self {
		let seed = params.finish_seed;
		let mut parts = Vec::with_capacity(6);
		for (x, z) in [
			(-LEG_INSET, -LEG_INSET),
			(LEG_INSET, -LEG_INSET),
			(-LEG_INSET, LEG_INSET),
			(LEG_INSET, LEG_INSET),
		] {
			parts.push(PlacedPart {
				kind: PartKind::ChairLeg,
				placement: shift(slab(LEG_XZ, 0.0, SEAT_Y0), Vec3::new(x, 0.0, z)),
				material: wood(seed, 1),
			});
		}
		parts.push(PlacedPart {
			kind: PartKind::ChairSeat,
			placement: slab(1.0, SEAT_Y0, SEAT_Y1),
			material: cloth(seed, 2),
		});
		parts.push(PlacedPart {
			kind: PartKind::ChairBack,
			placement: slab(1.0, SEAT_Y1, 1.0),
			material: wood(seed, 3),
		});
		Self { finish_seed: seed, parts }
	}

	pub fn unit_from_num(num: u32) -> Self {
		ChairParams::unit_from_num(num).build()
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Chair,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use furniture_components::pose_parts;
	use richmond_building_components::{FurnitureAbutment, FurnitureNode, Placement};

	#[test]
	fn back_uses_the_upper_band() -> anyhow::Result<()> {
		let chair = ChairParams::unit_from_num(4).build();
		let back = chair
			.parts
			.iter()
			.find(|p| p.kind == PartKind::ChairBack)
			.ok_or_else(|| anyhow::anyhow!("missing back"))?;
		if (back.placement.translation.y - SEAT_Y1).abs() > 1e-5 {
			return Err(anyhow::anyhow!(
				"chair back should sit on the seat top, got y={}",
				back.placement.translation.y
			));
		}
		Ok(())
	}

	#[test]
	fn abutment_yaw_aims_the_back_at_the_wall() -> anyhow::Result<()> {
		let chair = ChairParams::unit_from_num(4).build();
		let slot = FurnitureNode::chair(Placement::IDENTITY).with_abutment(FurnitureAbutment::PosX);
		let posed = pose_parts(slot.placement, &chair.parts);
		let back = posed
			.iter()
			.find(|p| p.kind == PartKind::ChairBack)
			.ok_or_else(|| anyhow::anyhow!("missing posed back"))?;
		let expected = FurnitureAbutment::PosX.facing_yaw();
		if (back.placement.yaw - expected).abs() > 1e-5 {
			return Err(anyhow::anyhow!(
				"PosX abutment should yaw the back to {expected}, got {}",
				back.placement.yaw
			));
		}
		Ok(())
	}
}
