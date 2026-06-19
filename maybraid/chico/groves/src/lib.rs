//! Cellular grove selection for Chico vegetation ([RFC-183 §4.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation#47-grove-core-selection)).

pub mod grove;

pub mod braid_grass;
pub mod bush_scrub;
pub mod common_tufts;
pub mod arid_conifer_sapling;
pub mod conifer_sapling;
pub mod goettingen_follow;
pub mod high_bush;
pub mod jerrys_chaparral;
pub mod jungle_lower_massives;
pub mod jungle_massives;
pub mod levantine_scrub;
pub mod low_bush;
pub mod monster_grass;
pub mod palm_shade;
pub mod riparian_mix;
pub mod riverine_green;
pub mod shamanhome;
pub mod spotty_bushes;
pub mod strange_oasis;
pub mod tall_grass;
pub mod temperate_lower_massives;
pub mod tropical_thicket;
pub mod tropical_tufts;
pub mod tropical_undergrowth;
pub mod unending_jungle;
pub mod wild_grass;

#[cfg(feature = "render")]
pub mod skipped_mesh_material;

pub use grove::{
	cell_center, parse_variant_weights, parse_vec2_csv, parse_vec3_csv, placement_noise,
	FlatTerrainSample, ForestGroveBiases, Grove, GroveBucket, GroveCellOutcome, GroveDefinition,
	GroveDistribution, GroveExtent, GroveFrontend, GrovePlacedCell, GrovePlacementRanges,
	PaletteColor, PaletteMix, PaletteSlot, PlacementConstraints, PlacementSample,
	PreparedGroveDistribution, TerrainSample, VariantWeightOverrides, DEFAULT_GROVE_EXTENT_XZ,
};

#[cfg(feature = "render")]
pub use grove::{patch_spawned_leaf_material, resolve_palette_color, WithPalette};

pub use braid_grass::{BraidGrassCell, BraidGrassClump, BraidGrassItem, BraidSpearClump};
pub use bush_scrub::{BushScrubBush, BushScrubCell, BushScrubItem, BushScrubTuft};
pub use common_tufts::{CommonTuftClump, CommonTuftsCell, CommonTuftsItem};
pub use arid_conifer_sapling::{
	AridConiferSaplingCell, AridConiferSaplingFriendsConifer, AridConiferSaplingItem,
	AridConiferSaplingLiamsConifer, AridConiferSaplingNorthernConifer,
};
pub use conifer_sapling::{
	ConiferSaplingCell, ConiferSaplingFriendsConifer, ConiferSaplingItem,
	ConiferSaplingNorthernConifer,
};
pub use goettingen_follow::{
	GoettingenFollowBraidOak, GoettingenFollowCell, GoettingenFollowItem,
	GoettingenFollowStorybook,
};
pub use high_bush::{HighBushBush, HighBushCell, HighBushItem};
pub use jerrys_chaparral::{
	JerrysChaparralBush, JerrysChaparralCell, JerrysChaparralFriendsConifer, JerrysChaparralItem,
	JerrysChaparralRoryHead,
};
pub use jungle_lower_massives::{
	JungleLowerMassivesBanyan, JungleLowerMassivesBraidOak, JungleLowerMassivesCell,
	JungleLowerMassivesItem, JungleLowerMassivesJungleStorybook, JungleLowerMassivesWaialeaPalm,
};
pub use jungle_massives::{
	JungleMassivesBanyan, JungleMassivesCell, JungleMassivesItem, JungleMassivesJungleStorybook,
};
pub use levantine_scrub::{
	LevantineScrubBraidOak, LevantineScrubBush, LevantineScrubCell, LevantineScrubHedge,
	LevantineScrubItem, LevantineScrubRoryHead, LevantineScrubTorch, LevantineScrubVaseTree,
};
pub use low_bush::{LowBushBush, LowBushCell, LowBushItem};
pub use monster_grass::{MonsterGrassCell, MonsterGrassClump, MonsterGrassItem};
pub use palm_shade::{
	PalmShadeCell, PalmShadeDatePalm, PalmShadeItem, PalmShadeWaialeaPalm,
};
pub use riparian_mix::{
	RiparianMixBraidOak, RiparianMixCell, RiparianMixFriendsConifer, RiparianMixItem,
	RiparianMixStorybook, RiparianMixTemperateConifer,
};
pub use riverine_green::{RiverineGreenBush, RiverineGreenCell, RiverineGreenItem};
pub use shamanhome::{
	ShamanhomeBanyan, ShamanhomeBraidOak, ShamanhomeCell, ShamanhomeDatePalm, ShamanhomeItem,
};
pub use spotty_bushes::{SpottyBushesBush, SpottyBushesCell, SpottyBushesItem};
pub use temperate_lower_massives::{
	TemperateLowerMassivesBraidOak, TemperateLowerMassivesCell, TemperateLowerMassivesItem,
	TemperateLowerMassivesRory, TemperateLowerMassivesStorybook,
};
pub use strange_oasis::{
	StrangeOasisDatePalm, StrangeOasisCell, StrangeOasisItem, StrangeOasisStorybook,
	StrangeOasisTorch,
};
pub use tall_grass::{TallGrassCell, TallGrassClump, TallGrassItem};
pub use tropical_thicket::{
	TropicalThicketBanyan, TropicalThicketBush, TropicalThicketCell, TropicalThicketItem,
	TropicalThicketPalm,
};
pub use tropical_tufts::{
	TropicalPalmBush, TropicalTuftClump, TropicalTuftsCell, TropicalTuftsItem,
};
pub use tropical_undergrowth::{
	TropicalUndergrowthCell, TropicalUndergrowthItem, TropicalUndergrowthPalm,
	TropicalUndergrowthRoryHead, TropicalUndergrowthStorybook, TropicalUndergrowthTorch,
	TropicalUndergrowthTuft, TropicalUndergrowthVaseTree,
};
pub use unending_jungle::{
	UnendingJungleBanyan, UnendingJungleCell, UnendingJungleItem, UnendingJungleJungleStorybook,
	UnendingJungleRoryHead, UnendingJungleStorybook, UnendingJungleTorch, UnendingJungleWaialeaPalm,
};
pub use wild_grass::{WildGrassCell, WildGrassClump, WildGrassItem};

#[cfg(feature = "render")]
pub use braid_grass::{BraidGrass, BraidGrassStd};
#[cfg(feature = "render")]
pub use bush_scrub::{BushScrub, BushScrubStd};
#[cfg(feature = "render")]
pub use common_tufts::{CommonTufts, CommonTuftsStd};
#[cfg(feature = "render")]
pub use goettingen_follow::{GoettingenFollow, GoettingenFollowStd};
#[cfg(feature = "render")]
pub use high_bush::{HighBush, HighBushStd};
#[cfg(feature = "render")]
pub use jerrys_chaparral::{JerrysChaparral, JerrysChaparralStd};
#[cfg(feature = "render")]
pub use jungle_lower_massives::{JungleLowerMassives, JungleLowerMassivesStd};
#[cfg(feature = "render")]
pub use jungle_massives::{JungleMassives, JungleMassivesStd};
#[cfg(feature = "render")]
pub use levantine_scrub::{LevantineScrub, LevantineScrubStd};
#[cfg(feature = "render")]
pub use low_bush::{LowBush, LowBushStd};
#[cfg(feature = "render")]
pub use monster_grass::{MonsterGrass, MonsterGrassStd};
#[cfg(feature = "render")]
pub use palm_shade::{PalmShade, PalmShadeStd};
#[cfg(feature = "render")]
pub use riparian_mix::{RiparianMix, RiparianMixStd};
#[cfg(feature = "render")]
pub use riverine_green::{RiverineGreen, RiverineGreenStd};
#[cfg(feature = "render")]
pub use shamanhome::{Shamanhome, ShamanhomeStd};
#[cfg(feature = "render")]
pub use spotty_bushes::{SpottyBushes, SpottyBushesStd};
#[cfg(feature = "render")]
pub use temperate_lower_massives::{TemperateLowerMassives, TemperateLowerMassivesStd};
#[cfg(feature = "render")]
pub use strange_oasis::{StrangeOasis, StrangeOasisStd};
#[cfg(feature = "render")]
pub use tall_grass::{TallGrass, TallGrassStd};
#[cfg(feature = "render")]
pub use tropical_thicket::{TropicalThicket, TropicalThicketStd};
#[cfg(feature = "render")]
pub use tropical_tufts::{TropicalTufts, TropicalTuftsStd};
#[cfg(feature = "render")]
pub use tropical_undergrowth::{TropicalUndergrowth, TropicalUndergrowthStd};
#[cfg(feature = "render")]
pub use unending_jungle::{UnendingJungle, UnendingJungleStd};
#[cfg(feature = "render")]
pub use wild_grass::{WildGrass, WildGrassStd};
