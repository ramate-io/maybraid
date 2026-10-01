//! Layer knobs the playground's `PlaygroundConfig` used to carry for this layer.

use bevy::prelude::*;
use richmond_development_models::DevelopmentConfig;
use richmond_urbanization::UrbanizationKind;

use crate::stream::UrbanizationStreamSpec;

/// Occupancy fill used by both the world and the developments playground.
pub const PLAYGROUND_LIKELIHOOD: f32 = 0.9;

/// Exclusive development-archetype focus (playground `/focus-development`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevelopmentFocus {
	All,
	LesHalles,
	ShepherdsVillage,
	ShepherdsCommune,
	RingFort,
	TempleComplex,
	SingleHighrise,
	SuburbanHomes,
	WizardsTower,
	SkybridgeBazaar,
	OldCityMarket,
}

impl DevelopmentFocus {
	pub fn apply(self, config: &mut DevelopmentConfig) {
		let selected = if self == Self::All { None } else { Some(self) };
		config.les_halles_weight = weight(selected, Self::LesHalles);
		config.shepherds_village_weight = weight(selected, Self::ShepherdsVillage);
		config.shepherds_commune_weight = weight(selected, Self::ShepherdsCommune);
		config.ring_fort_weight = weight(selected, Self::RingFort);
		config.temple_complex_weight = weight(selected, Self::TempleComplex);
		config.single_highrise_weight = weight(selected, Self::SingleHighrise);
		config.suburban_homes_weight = weight(selected, Self::SuburbanHomes);
		config.wizards_tower_weight = weight(selected, Self::WizardsTower);
		config.skybridge_bazaar_weight = weight(selected, Self::SkybridgeBazaar);
		config.old_city_market_weight = weight(selected, Self::OldCityMarket);
	}

	pub fn as_kebab(self) -> &'static str {
		match self {
			Self::All => "all",
			Self::LesHalles => "les-halles",
			Self::ShepherdsVillage => "shepherds-village",
			Self::ShepherdsCommune => "shepherds-commune",
			Self::RingFort => "ring-fort",
			Self::TempleComplex => "temple-complex",
			Self::SingleHighrise => "single-highrise",
			Self::SuburbanHomes => "suburban-homes",
			Self::WizardsTower => "wizards-tower",
			Self::SkybridgeBazaar => "skybridge-bazaar",
			Self::OldCityMarket => "old-city-market",
		}
	}

	pub fn from_kebab(name: &str) -> Option<Self> {
		Some(match name {
			"all" => Self::All,
			"les-halles" => Self::LesHalles,
			"shepherds-village" => Self::ShepherdsVillage,
			"shepherds-commune" => Self::ShepherdsCommune,
			"ring-fort" => Self::RingFort,
			"temple-complex" => Self::TempleComplex,
			"single-highrise" => Self::SingleHighrise,
			"suburban-homes" => Self::SuburbanHomes,
			"wizards-tower" => Self::WizardsTower,
			"skybridge-bazaar" => Self::SkybridgeBazaar,
			"old-city-market" => Self::OldCityMarket,
			_ => return None,
		})
	}
}

impl std::fmt::Display for DevelopmentFocus {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(self.as_kebab())
	}
}

fn weight(selected: Option<DevelopmentFocus>, kind: DevelopmentFocus) -> f32 {
	match selected {
		None => 1.0,
		Some(selected) if selected == kind => 1.0,
		Some(_) => 0.0,
	}
}

/// Stream spec, focus pins, and the urbanization generate budget.
///
/// `world_defaults()` is 16 (assembled world). [`Default`] is 8 (standalone
/// playground), matching the table in [#883](https://github.com/ramate-io/maybraid/issues/883).
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct UrbanizationLayerConfig {
	pub urbanization: Option<UrbanizationStreamSpec>,
	pub focus_urbanization: Option<UrbanizationKind>,
	pub focus_development: Option<DevelopmentFocus>,
	pub generate_budget: u32,
}

impl Default for UrbanizationLayerConfig {
	fn default() -> Self {
		Self {
			urbanization: None,
			focus_urbanization: None,
			focus_development: None,
			generate_budget: 8,
		}
	}
}

impl UrbanizationLayerConfig {
	/// Hopscotch at 1 km / 3 km rings, generate budget 16.
	pub fn world_defaults() -> Self {
		Self {
			urbanization: Some(UrbanizationStreamSpec::default()),
			focus_urbanization: None,
			focus_development: None,
			generate_budget: 16,
		}
	}

	/// Shared generate budget and development knobs, no hopscotch spec.
	pub fn shared_world() -> Self {
		Self {
			urbanization: None,
			focus_urbanization: None,
			focus_development: None,
			generate_budget: 16,
		}
	}

	pub fn shared_config(&self) -> UrbanizationSharedConfig {
		UrbanizationSharedConfig {
			generate_budget: self.generate_budget,
			development: self.development_config(),
		}
	}

	pub fn development_config(&self) -> DevelopmentConfig {
		let mut development = DevelopmentConfig {
			likelihood: PLAYGROUND_LIKELIHOOD,
			// Stream generate walks urbanization leaves. A development focus
			// then only changes kind weights. The 300 m lattice is the no-stream
			// catalog path.
			use_urbanization: self.urbanization.is_some() || self.focus_development.is_none(),
			..DevelopmentConfig::from_world_seed(42)
		};
		if let Some(focus) = self.focus_development {
			focus.apply(&mut development);
		}
		development
	}
}

/// Generate budget and [`DevelopmentConfig`] every mode must agree on.
#[derive(Clone, Debug, PartialEq)]
pub struct UrbanizationSharedConfig {
	pub generate_budget: u32,
	pub development: DevelopmentConfig,
}
