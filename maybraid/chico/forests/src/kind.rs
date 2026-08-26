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
