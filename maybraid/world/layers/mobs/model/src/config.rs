//! Layer knobs the world assembler used to hard-code for mobs.

use bevy::prelude::*;

/// Generate budget for [`MobLodChan`](crate::MobLodChan).
///
/// Every budget is 16 in the world. Radii, seed, occupancy, and cell size stay
/// constants on the index and stream.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct MobLayerConfig {
	pub generate_budget: u32,
}

impl Default for MobLayerConfig {
	fn default() -> Self {
		Self { generate_budget: 16 }
	}
}

impl MobLayerConfig {
	/// Generate budget 16, the value the world assembler used to insert.
	pub fn world_defaults() -> Self {
		Self { generate_budget: 16 }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn world_defaults_keep_the_sixteen_id_budget() {
		assert_eq!(MobLayerConfig::world_defaults().generate_budget, 16);
		assert_eq!(MobLayerConfig::default().generate_budget, 16);
	}
}
