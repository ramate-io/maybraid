//! Unified generated-development artifact.
//!
//! Selection and terrain padding happen once per development cell. The fitted
//! result is then stored behind this enum so adding an archetype does not add a
//! parallel spatial store, generation pass, and playground scan.

use urbanization_developments::{
	MixedUseLesHallesDevelopment, OldCityMarket, PlacedBuilding, RingFort, ShepherdsCommune,
	ShepherdsVillage, SingleHighrise, SkybridgeBazaar, SolitaryWizardsTower, SuburbanHomes,
	TempleComplex,
};

/// One fitted development generated for an occupied cell.
#[derive(Debug, Clone)]
pub enum BuiltDevelopment {
	LesHalles(Box<PlacedBuilding<MixedUseLesHallesDevelopment>>),
	ShepherdsVillage(Box<ShepherdsVillage>),
	ShepherdsCommune(Box<ShepherdsCommune>),
	RingFort(Box<PlacedBuilding<RingFort>>),
	TempleComplex(Box<TempleComplex>),
	SingleHighrise(Box<PlacedBuilding<SingleHighrise>>),
	SuburbanHomes(Box<SuburbanHomes>),
	WizardsTower(Box<PlacedBuilding<SolitaryWizardsTower>>),
	SkybridgeBazaar(Box<SkybridgeBazaar>),
	OldCityMarket(Box<OldCityMarket>),
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn catalog_artifact_stays_pointer_sized_per_variant() {
		assert!(std::mem::size_of::<BuiltDevelopment>() <= 16);
	}
}
