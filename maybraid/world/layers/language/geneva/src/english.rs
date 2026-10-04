//! Explicit English naming vocabulary. Variant Debug text is not a source.

use chico::{ForestGroveKind, LayeringKind};
use durham::GeographicFeatureKind;
use maybraid_language_core::lexicalizer::mix;
use richmond::DiscoverablePlaceLabel;
use urbanization_cells::{UrbanDevelopmentKind, UrbanizationKind};

/// Terms Geneva may pick from a Durham geographic kind.
pub fn geographic_terms(kind: GeographicFeatureKind) -> Vec<String> {
	owned(match kind {
		GeographicFeatureKind::Massif => &["massif", "mountain"],
		GeographicFeatureKind::Plateau => &["plateau"],
		GeographicFeatureKind::Canyon => &["canyon"],
		GeographicFeatureKind::Rolling => &["rolling"],
		GeographicFeatureKind::Valley => &["valley"],
		GeographicFeatureKind::PocketWater => &["water"],
		GeographicFeatureKind::Lake => &["lake"],
		GeographicFeatureKind::Bog => &["bog"],
		GeographicFeatureKind::Stream | GeographicFeatureKind::StreamsGraph => &["stream"],
	})
}

/// Terms implied by a forest layering. Internal recipe names stay out.
pub fn layering_terms(kind: LayeringKind) -> Vec<String> {
	owned(match kind {
		LayeringKind::LushJungle | LayeringKind::Kumulipo => &["jungle"],
		LayeringKind::Riparian => &["riparian"],
		LayeringKind::Taiga => &["taiga"],
		LayeringKind::LiamsSummer => &["summer"],
		LayeringKind::OwlsDesert | LayeringKind::OldNevada | LayeringKind::DamasEdge => &["desert"],
		LayeringKind::MiRobles => &["oak"],
		LayeringKind::Seceda => &["alpine"],
		LayeringKind::Waiguo | LayeringKind::OldSteppe | LayeringKind::SteppeDown => &["steppe"],
		LayeringKind::AgTown | LayeringKind::Storybook | LayeringKind::FruitPlains => &["orchard"],
		LayeringKind::SunsBarren => &["barren"],
		LayeringKind::TemperateHoly => &["temperate"],
		LayeringKind::TrapThicket => &["thicket"],
		LayeringKind::Bush => &["bush"],
		LayeringKind::Meadowland => &["meadow"],
		LayeringKind::OpenTropics | LayeringKind::WestMaui => &["tropics"],
		LayeringKind::UpperPark => &["park"],
	})
}

/// Terms implied by a selected grove. Implementation variants collapse.
pub fn grove_kind_terms(kind: ForestGroveKind) -> Vec<String> {
	owned(match kind {
		ForestGroveKind::Alpine => &["alpine"],
		ForestGroveKind::AridConiferSapling
		| ForestGroveKind::ConiferSapling
		| ForestGroveKind::ConiferMassives => &["conifer"],
		ForestGroveKind::BraidGrass
		| ForestGroveKind::MonsterGrass
		| ForestGroveKind::TallGrass
		| ForestGroveKind::WildGrass => &["grass"],
		ForestGroveKind::BushScrub
		| ForestGroveKind::HighBush
		| ForestGroveKind::LowBush
		| ForestGroveKind::SpottyBushes => &["bush"],
		ForestGroveKind::ChristmasTaiga => &["taiga"],
		ForestGroveKind::CommonTufts | ForestGroveKind::TropicalTufts => &["tufts"],
		ForestGroveKind::DateGrove => &["date"],
		ForestGroveKind::Dryland => &["dryland"],
		ForestGroveKind::ForlornSavanna => &["savanna"],
		ForestGroveKind::GoettingenFollow
		| ForestGroveKind::Shamanhome
		| ForestGroveKind::Storytellers => &["grove"],
		ForestGroveKind::JerrysChaparral => &["chaparral"],
		ForestGroveKind::JungleLowerMassives
		| ForestGroveKind::JungleMassives
		| ForestGroveKind::UnendingJungle
		| ForestGroveKind::TropicalUndergrowth => &["jungle"],
		ForestGroveKind::Leeward => &["leeward"],
		ForestGroveKind::LevantineScrub => &["scrub"],
		ForestGroveKind::Orchard => &["orchard"],
		ForestGroveKind::PalmShade => &["palm"],
		ForestGroveKind::RiparianGeneral | ForestGroveKind::RiparianMix => &["riparian"],
		ForestGroveKind::RiverineGreen => &["river"],
		ForestGroveKind::RollingOaks => &["oak"],
		ForestGroveKind::StrangeOasis => &["oasis"],
		ForestGroveKind::TemperateLowerMassives | ForestGroveKind::TemperateMassives => {
			&["temperate"]
		}
		ForestGroveKind::TradeWinds => &["trade"],
		ForestGroveKind::TropicalThicket => &["thicket"],
		ForestGroveKind::Vineyard => &["vineyard"],
		ForestGroveKind::WanderingAcacia => &["acacia"],
	})
}

/// Terms implied by an urbanization cell kind.
pub fn urbanization_terms(kind: UrbanizationKind) -> Vec<String> {
	owned(match kind {
		UrbanizationKind::None => &[],
		UrbanizationKind::MixedAgeCity | UrbanizationKind::ModernCity => &["city"],
		UrbanizationKind::RuralLife => &["rural"],
		UrbanizationKind::Townships => &["town"],
		UrbanizationKind::Frontier => &["frontier"],
		UrbanizationKind::Colony => &["colony"],
	})
}

/// Terms implied by a development leaf.
pub fn development_terms(kind: UrbanDevelopmentKind) -> Vec<String> {
	owned(match kind {
		UrbanDevelopmentKind::Empty => &[],
		UrbanDevelopmentKind::LesHalles | UrbanDevelopmentKind::OldCityMarket => &["market"],
		UrbanDevelopmentKind::ShepherdsVillage => &["village"],
		UrbanDevelopmentKind::ShepherdsCommune => &["commune"],
		UrbanDevelopmentKind::RingFort => &["fort"],
		UrbanDevelopmentKind::TempleComplex => &["temple"],
		UrbanDevelopmentKind::SingleHighrise => &["highrise"],
		UrbanDevelopmentKind::SuburbanHomes => &["suburb"],
		UrbanDevelopmentKind::WizardsTower => &["tower"],
		UrbanDevelopmentKind::SkybridgeBazaar => &["bazaar"],
	})
}

/// Terms implied by a discoverable place label.
pub fn place_label_terms(label: DiscoverablePlaceLabel) -> Vec<String> {
	owned(match label {
		DiscoverablePlaceLabel::Stall => &["stall"],
		DiscoverablePlaceLabel::Lounge => &["lounge"],
		DiscoverablePlaceLabel::Bedroom => &["bedroom"],
		DiscoverablePlaceLabel::Sanctum => &["sanctum"],
		DiscoverablePlaceLabel::Storey => &["storey"],
		DiscoverablePlaceLabel::House => &["house"],
		DiscoverablePlaceLabel::Hut => &["hut"],
		DiscoverablePlaceLabel::Market => &["market"],
		DiscoverablePlaceLabel::Highrise => &["highrise"],
		DiscoverablePlaceLabel::Skybridge => &["skybridge"],
		DiscoverablePlaceLabel::Tower => &["tower"],
		DiscoverablePlaceLabel::Room => &["room"],
	})
}

/// Seeded color adjectives so nearby groves and POIs of the same kind differ.
pub const PLACE_COLORS: &[&str] = &[
	"red", "ochre", "amber", "gold", "olive", "green", "teal", "blue", "violet", "rose", "ivory",
	"umber", "slate", "copper", "silver", "rust",
];

/// One color from [`PLACE_COLORS`], stable for `seed`.
pub fn color_term(seed: u64) -> String {
	PLACE_COLORS[(mix(seed ^ 0xC010_A11A) as usize) % PLACE_COLORS.len()].to_owned()
}

/// Prepend a seeded color, keeping the kind terms after it.
pub fn with_color_name(mut english: Vec<String>, seed: u64) -> Vec<String> {
	let color = color_term(seed);
	english.retain(|word| word != &color);
	english.insert(0, color);
	english
}

fn owned(words: &[&str]) -> Vec<String> {
	words.iter().map(|word| (*word).to_owned()).collect()
}
