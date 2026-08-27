//! Grove and layering identities that exist in `chico-groves`.
//!
//! Ground-cover groves and Conifer Lower Massives are not listed — they are not
//! implemented and must not be aliased onto another grove.

/// Existing well-known grove a forest layer may select.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ForestGroveKind {
	Alpine,
	AridConiferSapling,
	BraidGrass,
	BushScrub,
	ChristmasTaiga,
	CommonTufts,
	ConiferMassives,
	ConiferSapling,
	DateGrove,
	Dryland,
	ForlornSavanna,
	GoettingenFollow,
	HighBush,
	JerrysChaparral,
	JungleLowerMassives,
	JungleMassives,
	Leeward,
	LevantineScrub,
	LowBush,
	MonsterGrass,
	Orchard,
	PalmShade,
	RiparianGeneral,
	RiparianMix,
	RiverineGreen,
	RollingOaks,
	Shamanhome,
	SpottyBushes,
	Storytellers,
	StrangeOasis,
	TallGrass,
	TemperateLowerMassives,
	TemperateMassives,
	TradeWinds,
	TropicalThicket,
	TropicalTufts,
	TropicalUndergrowth,
	UnendingJungle,
	Vineyard,
	WanderingAcacia,
	WildGrass,
}

/// Well-known forest layering ([RFC-183 §3.5.4]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LayeringKind {
	LushJungle,
	Riparian,
	Taiga,
	LiamsSummer,
	OwlsDesert,
	MiRobles,
	Seceda,
	Kumulipo,
	Waiguo,
	AgTown,
	SunsBarren,
	TemperateHoly,
	OldSteppe,
	TrapThicket,
	Bush,
	OldNevada,
	Storybook,
	Meadowland,
	FruitPlains,
	DamasEdge,
	OpenTropics,
	WestMaui,
	UpperPark,
	SteppeDown,
}

/// One layer bucket: a grove or explicit `None`, plus throw weight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeightedGrove {
	pub kind: Option<ForestGroveKind>,
	pub weight: f32,
}

/// Four forest layers (no ground cover). Each is a Bucket Throw including `None`.
#[derive(Debug, Clone, PartialEq)]
pub struct ForestLayering {
	pub kind: LayeringKind,
	pub tufts: Vec<WeightedGrove>,
	pub understory: Vec<WeightedGrove>,
	pub lower_canopy: Vec<WeightedGrove>,
	pub upper_canopy: Vec<WeightedGrove>,
}

/// Groves selected for one forest cell (one per layer, or `None`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedLayers {
	pub layering: LayeringKind,
	pub tufts: Option<ForestGroveKind>,
	pub understory: Option<ForestGroveKind>,
	pub lower_canopy: Option<ForestGroveKind>,
	pub upper_canopy: Option<ForestGroveKind>,
}

impl LayeringKind {
	pub const ALL: &[Self] = &[
		Self::LushJungle,
		Self::Riparian,
		Self::Taiga,
		Self::LiamsSummer,
		Self::OwlsDesert,
		Self::MiRobles,
		Self::Seceda,
		Self::Kumulipo,
		Self::Waiguo,
		Self::AgTown,
		Self::SunsBarren,
		Self::TemperateHoly,
		Self::OldSteppe,
		Self::TrapThicket,
		Self::Bush,
		Self::OldNevada,
		Self::Storybook,
		Self::Meadowland,
		Self::FruitPlains,
		Self::DamasEdge,
		Self::OpenTropics,
		Self::WestMaui,
		Self::UpperPark,
		Self::SteppeDown,
	];

	pub fn as_kebab(self) -> &'static str {
		match self {
			Self::LushJungle => "lush-jungle",
			Self::Riparian => "riparian",
			Self::Taiga => "taiga",
			Self::LiamsSummer => "liams-summer",
			Self::OwlsDesert => "owls-desert",
			Self::MiRobles => "mi-robles",
			Self::Seceda => "seceda",
			Self::Kumulipo => "kumulipo",
			Self::Waiguo => "waiguo",
			Self::AgTown => "ag-town",
			Self::SunsBarren => "suns-barren",
			Self::TemperateHoly => "temperate-holy",
			Self::OldSteppe => "old-steppe",
			Self::TrapThicket => "trap-thicket",
			Self::Bush => "bush",
			Self::OldNevada => "old-nevada",
			Self::Storybook => "storybook",
			Self::Meadowland => "meadowland",
			Self::FruitPlains => "fruit-plains",
			Self::DamasEdge => "damas-edge",
			Self::OpenTropics => "open-tropics",
			Self::WestMaui => "west-maui",
			Self::UpperPark => "upper-park",
			Self::SteppeDown => "steppe-down",
		}
	}

	pub fn from_kebab(name: &str) -> Option<Self> {
		let key = name.trim().to_ascii_lowercase();
		Self::ALL.iter().copied().find(|kind| kind.as_kebab() == key)
	}
}

impl ForestLayering {
	/// Highest-weight non-`None` grove on each layer (review / pinned cells).
	pub fn typical_layers(&self) -> SelectedLayers {
		SelectedLayers {
			layering: self.kind,
			tufts: typical_grove(&self.tufts),
			understory: typical_grove(&self.understory),
			lower_canopy: typical_grove(&self.lower_canopy),
			upper_canopy: typical_grove(&self.upper_canopy),
		}
	}
}

fn typical_grove(buckets: &[WeightedGrove]) -> Option<ForestGroveKind> {
	buckets
		.iter()
		.filter_map(|bucket| Some((bucket.kind?, bucket.weight)))
		.max_by(|a, b| a.1.total_cmp(&b.1))
		.map(|(kind, _)| kind)
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn kebab_round_trips_every_layering() -> Result<()> {
		assert_eq!(LayeringKind::ALL.len(), 24);
		for kind in LayeringKind::ALL {
			let name = kind.as_kebab();
			assert_eq!(LayeringKind::from_kebab(name), Some(*kind));
		}
		assert!(LayeringKind::from_kebab("not-a-layering").is_none());
		Ok(())
	}

	#[test]
	fn typical_lush_jungle_keeps_canopy() -> Result<()> {
		let layers = LayeringKind::LushJungle.layering().typical_layers();
		assert_eq!(layers.upper_canopy, Some(ForestGroveKind::TradeWinds));
		assert!(layers.tufts.is_some());
		Ok(())
	}
}
