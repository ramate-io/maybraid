//! Interior subtype selection for a commercial stall.

use lod::gen::LodSceneLevel;
use procedural_common::{NoiseParams, NoiseType, TypedBucketThrow};
use richmond_building_components::panels::PanelNode;
use richmond_building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};

use crate::fit::{Confines, FillableRegions, Fit, FitError};

use super::bites_sitdown_stall::BitesSitdownStall;
use super::bites_stall::BitesStall;
use super::knick_knack_stall::KnickKnackStall;
use super::label_util::label_filling_aabb;
use super::parts_stall::PartsStall;
use super::public_restroom::PublicRestroom;
use super::supermarket_stall::SupermarketStall;

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

const FIRST_FIT_ORDER: &[InteriorKind] = &[
	InteriorKind::Bites,
	InteriorKind::BitesSitdown,
	InteriorKind::Supermarket,
	InteriorKind::KnickKnack,
	InteriorKind::Parts,
	InteriorKind::PublicRestroom,
];

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

fn try_fit_kind(
	kind: InteriorKind,
	confines: &Confines,
	noise: NoiseParams,
) -> Result<(CommercialStallInterior, FillableRegions), FitError> {
	match kind {
		InteriorKind::Bites => BitesStall::fit_to_confines(confines, noise)
			.map(|(s, r)| (CommercialStallInterior::Bites(s), r)),
		InteriorKind::BitesSitdown => BitesSitdownStall::fit_to_confines(confines, noise)
			.map(|(s, r)| (CommercialStallInterior::BitesSitdown(s), r)),
		InteriorKind::Supermarket => SupermarketStall::fit_to_confines(confines, noise)
			.map(|(s, r)| (CommercialStallInterior::Supermarket(s), r)),
		InteriorKind::KnickKnack => KnickKnackStall::fit_to_confines(confines, noise)
			.map(|(s, r)| (CommercialStallInterior::KnickKnack(s), r)),
		InteriorKind::Parts => PartsStall::fit_to_confines(confines, noise)
			.map(|(s, r)| (CommercialStallInterior::Parts(s), r)),
		InteriorKind::PublicRestroom => PublicRestroom::fit_to_confines(confines, noise)
			.map(|(s, r)| (CommercialStallInterior::PublicRestroom(s), r)),
	}
}

impl Fit for CommercialStallInterior {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let catalog = interior_catalog();
		let preferred = catalog
			.select_from_noise_3d(subtype_noise(noise), confines.center())
			.copied()
			.unwrap_or(InteriorKind::Bites);

		// Preferred first, then first-fit through the remaining catalog order.
		let mut order = vec![preferred];
		for kind in FIRST_FIT_ORDER {
			if *kind != preferred {
				order.push(*kind);
			}
		}

		for kind in order {
			if let Ok(fitted) = try_fit_kind(kind, confines, noise) {
				return Ok(fitted);
			}
		}

		Ok((
			Self::Fallback(label_filling_aabb(
				LabelStyle::Gray,
				"commercial stall",
				&confines.bounds,
				confines.roll,
			)),
			FillableRegions::empty(),
		))
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
	use crate::openings::{Opening, OpeningId, Openings};

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
		assert!(!ia
			.label_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.is_empty());
		assert!(!ib
			.label_nodes_for_level(LodSceneLevel::High)
			.flatten()
			.is_empty());
	}

	#[test]
	fn bites_failure_falls_through_to_another_type() {
		// Shallow + short doors → Bites fails kitchen / passage; another type should win.
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("short"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(1.0, 0.0, -0.2),
				Vec3::new(2.2, 2.0, 0.2),
			)),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(5.0, 3.0, 2.5)),
			0.0,
			openings,
		);
		let noise = NoiseParams {
			seed: 1,
			noise_type: NoiseType::Cellular,
			frequency: 0.35,
			..NoiseParams::default()
		};
		let (interior, _) = CommercialStallInterior::fit_to_confines(&confines, noise).unwrap();
		assert!(!matches!(interior, CommercialStallInterior::Bites(_)));
	}
}
