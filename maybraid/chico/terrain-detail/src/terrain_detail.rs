//! Select-only parent. Mirrors [`ChicoForest`]: hopscotch a 400 m cell to a
//! [`FormationKind`]. Does not grow rocks. Does not implement `LodScene`.

use procedural_common::NoiseParams;

use crate::{FormationExtent, FormationKind};

/// One selected 400 m formation tile.
#[derive(Clone, Debug, PartialEq)]
pub struct TerrainDetail {
	pub extent: FormationExtent,
	pub formation: FormationKind,
}

impl TerrainDetail {
	/// Four-way formation throw. Does not grow outcroppings.
	pub fn select_on(extent: FormationExtent, noise: NoiseParams) -> Self {
		Self { extent, formation: FormationKind::throw_on(extent, noise) }
	}

	/// Pin a well-known formation (review / `/show` cells).
	pub fn select_formation(extent: FormationExtent, formation: FormationKind) -> Self {
		Self { extent, formation }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn select_on_records_extent_and_formation() -> Result<()> {
		let extent = FormationExtent::from_cell_index(0, 0);
		let detail = TerrainDetail::select_on(extent, NoiseParams::default());
		assert_eq!(detail.extent, extent);
		assert!(FormationKind::ALL.contains(&detail.formation));
		Ok(())
	}

	#[test]
	fn select_formation_pins_empty() -> Result<()> {
		let extent = FormationExtent::from_cell_index(1, 1);
		let detail = TerrainDetail::select_formation(extent, FormationKind::Empty);
		assert_eq!(detail.formation, FormationKind::Empty);
		Ok(())
	}
}
