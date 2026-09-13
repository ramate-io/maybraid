//! Chest: trunk plus a lid on top.

use crate::kit_space::slab;
use crate::palette::wood;
use crate::parts::{Assembly, PartKind, PlacedPart};
use richmond_building_components::FurnitureGeometry;

/// Finish-only knobs. Topology does not change with [`Self::finish_seed`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ChestParams {
	pub finish_seed: u64,
}

impl ChestParams {
	pub fn unit_from_num(num: u32) -> Self {
		Self { finish_seed: u64::from(num) }
	}

	pub fn build(&self) -> Chest {
		Chest::from_params(*self)
	}
}

/// Trunk fills most of the slot; lid occupies the top band.
#[derive(Clone, Debug, PartialEq)]
pub struct Chest {
	pub finish_seed: u64,
	pub parts: Vec<PlacedPart>,
}

impl Chest {
	pub fn from_params(params: ChestParams) -> Self {
		let seed = params.finish_seed;
		Self {
			finish_seed: seed,
			parts: vec![
				PlacedPart {
					kind: PartKind::ChestTrunk,
					placement: slab(1.0, 0.0, 0.78),
					material: wood(seed, 1),
				},
				PlacedPart {
					kind: PartKind::ChestLid,
					placement: slab(1.02, 0.78, 1.0),
					material: wood(seed, 2),
				},
			],
		}
	}

	pub fn unit_from_num(num: u32) -> Self {
		ChestParams::unit_from_num(num).build()
	}

	pub fn assembly(&self) -> Assembly {
		Assembly {
			geometry: FurnitureGeometry::Chest,
			finish_seed: self.finish_seed,
			parts: self.parts.clone(),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn y_range(part: &PlacedPart) -> (f32, f32) {
		let half = part.placement.scale.y * 0.5;
		(part.placement.translation.y - half, part.placement.translation.y + half)
	}

	#[test]
	fn lid_sits_on_the_trunk() -> anyhow::Result<()> {
		let chest = ChestParams::unit_from_num(3).build();
		let trunk = chest
			.parts
			.iter()
			.find(|p| p.kind == PartKind::ChestTrunk)
			.ok_or_else(|| anyhow::anyhow!("missing trunk"))?;
		let lid = chest
			.parts
			.iter()
			.find(|p| p.kind == PartKind::ChestLid)
			.ok_or_else(|| anyhow::anyhow!("missing lid"))?;
		let (_, trunk_top) = y_range(trunk);
		let (lid_bottom, _) = y_range(lid);
		if (lid_bottom - trunk_top).abs() > 1e-4 {
			return Err(anyhow::anyhow!(
				"lid bottom {lid_bottom} should meet trunk top {trunk_top}"
			));
		}
		Ok(())
	}
}
