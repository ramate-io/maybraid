//! Lounge: always-fit fallback when no catalog interior accepts the bay.
//!
//! **Semantically:** soft open sitting / holding space — “we still fill the cell.”
//!
//! **Programmatically:** single bay-filling label; never soft-fails (outside the
//! weighted catalog; last resort in [`super::interior`]).

use lod::gen::LodSceneLevel;
use procedural_common::NoiseParams;
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};

use super::label_util::label_filling_aabb;

/// Soft open lounge that accepts any non-degenerate confines.
#[derive(Debug, Clone, PartialEq)]
pub struct Lounge {
	pub stall_type: LabelNode,
}

impl Fit for Lounge {
	fn fit_to_confines(
		confines: &Confines,
		_noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Ok((
			Self {
				stall_type: label_filling_aabb(
					LabelStyle::Gray,
					"Lounge",
					&confines.bounds,
					confines.roll,
				),
			},
			FillableRegions::empty(),
		))
	}
}

impl BuildingComponents for Lounge {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		Layers::from_free(vec![self.stall_type.clone()])
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;

	#[test]
	fn lounge_fits_tiny_bay() {
		let confines =
			Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(0.5, 1.0, 0.5)));
		let (lounge, _) = Lounge::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert_eq!(lounge.stall_type.text, "Lounge");
	}
}
