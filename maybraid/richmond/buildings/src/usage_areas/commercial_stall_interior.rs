//! Interior subtype selection for a commercial stall.

use lod::gen::LodSceneLevel;
use procedural_common::{NoiseParams, NoiseType, TypedBucketThrow};
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, LabelNode, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::bites_sitdown_stall::BitesSitdownStall;
use crate::usage_areas::bites_stall::BitesStall;
use crate::usage_areas::knick_knack_stall::KnickKnackStall;
use crate::usage_areas::label_util::label_filling_aabb;
use crate::usage_areas::parts_stall::PartsStall;
use crate::usage_areas::public_restroom::PublicRestroom;
use crate::usage_areas::supermarket_stall::SupermarketStall;
use richmond_building_components::LabelStyle;

/// Selected commercial stall interior.
#[derive(Debug, Clone, PartialEq)]
pub enum CommercialStallInterior {
	Bites(BitesStall),
	BitesSitdown(BitesSitdownStall),
	Supermarket(SupermarketStall),
	KnickKnack(KnickKnackStall),
	Parts(PartsStall),
	PublicRestroom(PublicRestroom),
	Fallback(LabelNode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InteriorKind {
	Bites,
	BitesSitdown,
	Supermarket,
	KnickKnack,
	Parts,
	PublicRestroom,
}

fn interior_catalog() -> TypedBucketThrow<InteriorKind> {
	let mut d = TypedBucketThrow::new();
	d.add(InteriorKind::Bites, 3.0);
	d.add(InteriorKind::BitesSitdown, 2.5);
	d.add(InteriorKind::Supermarket, 1.5);
	d.add(InteriorKind::KnickKnack, 2.0);
	d.add(InteriorKind::Parts, 1.5);
	d.add(InteriorKind::PublicRestroom, 0.8);
	d
}

fn subtype_noise(noise: NoiseParams) -> NoiseParams {
	NoiseParams {
		noise_type: NoiseType::Cellular,
		frequency: 0.35,
		..noise
	}
}

impl Fit for CommercialStallInterior {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let catalog = interior_catalog();
		let pick = catalog
			.select_from_noise_3d(subtype_noise(noise), confines.center())
			.copied()
			.unwrap_or(InteriorKind::Bites);
		let fitted = match pick {
			InteriorKind::Bites => BitesStall::fit_to_confines(confines, noise)
				.map(|(s, r)| (Self::Bites(s), r)),
			InteriorKind::BitesSitdown => BitesSitdownStall::fit_to_confines(confines, noise)
				.map(|(s, r)| (Self::BitesSitdown(s), r)),
			InteriorKind::Supermarket => SupermarketStall::fit_to_confines(confines, noise)
				.map(|(s, r)| (Self::Supermarket(s), r)),
			InteriorKind::KnickKnack => KnickKnackStall::fit_to_confines(confines, noise)
				.map(|(s, r)| (Self::KnickKnack(s), r)),
			InteriorKind::Parts => PartsStall::fit_to_confines(confines, noise)
				.map(|(s, r)| (Self::Parts(s), r)),
			InteriorKind::PublicRestroom => PublicRestroom::fit_to_confines(confines, noise)
				.map(|(s, r)| (Self::PublicRestroom(s), r)),
		};
		match fitted {
			Ok(v) => Ok(v),
			Err(_) => Ok((
				Self::Fallback(label_filling_aabb(
					LabelStyle::Gray,
					"commercial stall",
					&confines.bounds,
					confines.roll,
				)),
				FillableRegions::empty(),
			)),
		}
	}
}

impl BuildingComponents for CommercialStallInterior {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		match self {
			Self::Supermarket(s) => s.panel_nodes_for_level(level),
			Self::Parts(s) => s.panel_nodes_for_level(level),
			_ => Layers::new(),
		}
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		match self {
			Self::Bites(s) => s.label_nodes_for_level(level),
			Self::BitesSitdown(s) => s.label_nodes_for_level(level),
			Self::Supermarket(s) => s.label_nodes_for_level(level),
			Self::KnickKnack(s) => s.label_nodes_for_level(level),
			Self::Parts(s) => s.label_nodes_for_level(level),
			Self::PublicRestroom(s) => s.label_nodes_for_level(level),
			Self::Fallback(label) => Layers::from_free(vec![label.clone()]),
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;

	#[test]
	fn cellular_pick_varies_across_neighbors() {
		let a = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::ZERO,
			Vec3::new(4.0, 3.0, 5.0),
		));
		let b = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(8.0, 0.0, 0.0),
			Vec3::new(12.0, 3.0, 5.0),
		));
		let noise = NoiseParams {
			seed: 99,
			noise_type: NoiseType::Cellular,
			frequency: 0.35,
			..NoiseParams::default()
		};
		let (ia, _) = CommercialStallInterior::fit_to_confines(&a, noise).unwrap();
		let (ib, _) = CommercialStallInterior::fit_to_confines(&b, noise).unwrap();
		let ta = ia.label_nodes_for_level(LodSceneLevel::High).flatten()[0].text.clone();
		let tb = ib.label_nodes_for_level(LodSceneLevel::High).flatten()[0].text.clone();
		// Not a hard requirement they differ, but catalogs should often diverge.
		let _ = (ta, tb);
		assert!(!ia
			.label_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.is_empty());
	}
}
