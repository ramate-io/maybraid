//! `/show` — LodScene presentation (VegetationComponents).

use crate::forest_stream::{
	parse_layering_kind, DEFAULT_FOREST_NOISE, DEFAULT_FOREST_STREAM_RADIUS,
};
use crate::monster_grass_plain::spawn_monster_grass_plain;
use crate::vast::{parse_vast_grove_name, spawn_vast_grove};
use bevy::prelude::*;
use chico_forests::LayeringKind;
use chico_groves::{
	AlpineParams, AridConiferSaplingParams, BraidGrassParams, BushScrubParams,
	ChristmasTaigaParams, CommonTuftsParams, ConiferMassivesParams, ConiferSaplingParams,
	DateGroveParams, DrylandParams, ForlornSavannaParams, GoettingenFollowParams, GroveExtent,
	HighBushParams, JerrysChaparralParams, JungleLowerMassivesParams, JungleMassivesParams,
	LeewardParams, LevantineScrubParams, LowBushParams, MonsterGrassParams, OrchardParams,
	PalmShadeParams, RiparianGeneralParams, RiparianMixParams, RiverineGreenParams,
	RollingOaksParams, ShamanhomeParams, SpottyBushesParams, StorytellersParams,
	StrangeOasisParams, TallGrassParams, TemperateLowerMassivesParams, TemperateMassivesParams,
	TradeWindsParams, TropicalThicketParams, TropicalTuftsParams, TropicalUndergrowthParams,
	UnendingJungleParams, VineyardParams, WanderingAcaciaParams, WildGrassParams,
	DEFAULT_GROVE_EXTENT_XZ,
};
use chico_sbs_trees::{
	BraidOakTreeParams, DatePalmParams, FriendsConiferParams, HighBushShootsParams,
	HonuBanyanParams, JungleGrowthParams, JungleStorybookTreeParams, KamakuraTorchParams,
	LiamsConiferParams, NorthernConiferParams, PalmBushParams, PalmCrownParams,
	PenmarchTorchParams, RorysHeadTrainedParams, SimplemansHedgeParams, SopesBanyanParams,
	StorybookTreeParams, TemperateConiferParams, TuftPatchParams, VaseTreeParams,
	WaialeaPalmParams,
};
use chico_vegetation_components::{
	spawn_flattened_placed_vegetation, spawn_lod_scene_host, vegetation_bounds,
	VegetationComponents,
};
use clap::{Args, Subcommand};
use lod::gen::LodScene;
use procedural_common::{noise_params_from_scalar_str, NoiseParams};

use crate::render::SbsRenderItem;

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Show {
	/// Sope's Banyan via VegetationComponents / LodScene.
	SopesBanyan(ShowSopesBanyan),
	/// Penmarch Torch via VegetationComponents / LodScene.
	PenmarchTorch(ShowPenmarchTorch),
	/// Kamakura Torch via VegetationComponents / LodScene.
	KamakuraTorch(ShowKamakuraTorch),
	/// Rory's Head-trained via VegetationComponents / LodScene.
	RorysHeadTrained(ShowRorysHeadTrained),
	/// Storybook Tree via VegetationComponents / LodScene.
	StorybookTree(ShowStorybookTree),
	/// Vase Tree via VegetationComponents / LodScene.
	VaseTree(ShowVaseTree),
	/// Northern Conifer via VegetationComponents / LodScene.
	NorthernConifer(ShowNorthernConifer),
	/// Liam's Conifer via VegetationComponents / LodScene.
	LiamsConifer(ShowLiamsConifer),
	/// Temperate Conifer via VegetationComponents / LodScene.
	TemperateConifer(ShowTemperateConifer),
	/// Honu Banyan via VegetationComponents / LodScene.
	HonuBanyan(ShowHonuBanyan),
	/// Jungle Storybook Tree via VegetationComponents / LodScene.
	JungleStorybookTree(ShowJungleStorybookTree),
	/// Braid Oak Tree via VegetationComponents / LodScene.
	BraidOakTree(ShowBraidOakTree),
	/// Simpleman's Hedge via VegetationComponents / LodScene.
	SimplemansHedge(ShowSimplemansHedge),
	/// Tuft Patch via VegetationComponents / LodScene (straight frond segments).
	TuftPatch(ShowTuftPatch),
	/// Palm Crown via VegetationComponents / LodScene (fronds; five-chord star at Low).
	PalmCrown(ShowPalmCrown),
	/// Date Palm via VegetationComponents / LodScene.
	DatePalm(ShowDatePalm),
	/// Waialea Palm via VegetationComponents / LodScene.
	WaialeaPalm(ShowWaialeaPalm),
	/// Palm Bush via VegetationComponents / LodScene.
	PalmBush(ShowPalmBush),
	/// Monster Grass grove via VegetationComponents / LodScene.
	MonsterGrass(ShowMonsterGrass),
	/// Centered radius-10 tile of default Monster Grass groves (21×21).
	MonsterGrassPlains,
	/// Braid Grass grove via VegetationComponents / LodScene.
	BraidGrass(ShowBraidGrass),
	/// Tropical Tufts grove via VegetationComponents / LodScene.
	TropicalTufts(ShowTropicalTufts),
	/// Common Tufts grove via VegetationComponents / LodScene.
	CommonTufts(ShowCommonTufts),
	/// Tall Grass grove via VegetationComponents / LodScene.
	TallGrass(ShowTallGrass),
	/// Wild Grass grove via VegetationComponents / LodScene.
	WildGrass(ShowWildGrass),
	/// Bush Scrub grove via VegetationComponents / LodScene.
	BushScrub(ShowBushScrub),
	/// Tropical Undergrowth grove via VegetationComponents / LodScene.
	TropicalUndergrowth(ShowTropicalUndergrowth),
	/// Levantine Scrub grove via VegetationComponents / LodScene.
	LevantineScrub(ShowLevantineScrub),
	/// Strange Oasis grove via VegetationComponents / LodScene.
	StrangeOasis(ShowStrangeOasis),
	/// Tropical Thicket grove via VegetationComponents / LodScene.
	TropicalThicket(ShowTropicalThicket),
	/// Rolling Oaks grove via VegetationComponents / LodScene.
	RollingOaks(ShowRollingOaks),
	/// Orchard grove via VegetationComponents / LodScene.
	Orchard(ShowOrchard),
	/// Centered radius-10 tile of a named grove (21×21) for scale testing.
	Vast(ShowVast),
	/// Centered radius-10 tile of default Orchard groves (21×21) for scale testing.
	VastOrchards,
	/// Unified Chico forest: Hopscotch 1600 m cells and spawn grove LodScene hosts.
	Forest(ShowForest),
	/// Riparian General grove via VegetationComponents / LodScene.
	RiparianGeneral(ShowRiparianGeneral),
	/// Forlorn Savanna grove via VegetationComponents / LodScene.
	ForlornSavanna(ShowForlornSavanna),
	/// Göttingen Follow grove via VegetationComponents / LodScene.
	GoettingenFollow(ShowGoettingenFollow),
	/// Vineyard grove via VegetationComponents / LodScene.
	Vineyard(ShowVineyard),
	/// Dryland grove via VegetationComponents / LodScene.
	Dryland(ShowDryland),
	/// Leeward grove via VegetationComponents / LodScene.
	Leeward(ShowLeeward),
	/// Temperate Lower Massives grove via VegetationComponents / LodScene.
	TemperateLowerMassives(ShowTemperateLowerMassives),
	/// Temperate Massives grove via VegetationComponents / LodScene.
	TemperateMassives(ShowTemperateMassives),
	/// Storytellers grove via VegetationComponents / LodScene.
	Storytellers(ShowStorytellers),
	/// Wandering Acacia grove via VegetationComponents / LodScene.
	WanderingAcacia(ShowWanderingAcacia),
	/// Trade Winds grove via VegetationComponents / LodScene.
	TradeWinds(ShowTradeWinds),
	/// High Bush grove via VegetationComponents / LodScene.
	HighBush(ShowHighBush),
	/// Spotty Bushes grove via VegetationComponents / LodScene.
	SpottyBushes(ShowSpottyBushes),
	/// Riverine Green grove via VegetationComponents / LodScene.
	RiverineGreen(ShowRiverineGreen),
	/// Low Bush grove via VegetationComponents / LodScene.
	LowBush(ShowLowBush),
	/// Jungle Massives grove via VegetationComponents / LodScene.
	JungleMassives(ShowJungleMassives),
	/// Jungle Lower Massives grove via VegetationComponents / LodScene.
	JungleLowerMassives(ShowJungleLowerMassives),
	/// Unending Jungle grove via VegetationComponents / LodScene.
	UnendingJungle(ShowUnendingJungle),
	/// Jerry's Chaparral grove via VegetationComponents / LodScene.
	JerrysChaparral(ShowJerrysChaparral),
	/// Riparian Mix grove via VegetationComponents / LodScene.
	RiparianMix(ShowRiparianMix),
	/// Alpine grove via VegetationComponents / LodScene.
	Alpine(ShowAlpine),
	/// Christmas Taiga grove via VegetationComponents / LodScene.
	ChristmasTaiga(ShowChristmasTaiga),
	/// Conifer Sapling grove via VegetationComponents / LodScene.
	ConiferSapling(ShowConiferSapling),
	/// Arid Conifer Sapling grove via VegetationComponents / LodScene.
	AridConiferSapling(ShowAridConiferSapling),
	/// Conifer Massives grove via VegetationComponents / LodScene.
	ConiferMassives(ShowConiferMassives),
	/// Palm Shade grove via VegetationComponents / LodScene.
	PalmShade(ShowPalmShade),
	/// Shamanhome grove via VegetationComponents / LodScene.
	Shamanhome(ShowShamanhome),
	/// Date Grove via VegetationComponents / LodScene.
	DateGrove(ShowDateGrove),
	/// Friend's Conifer via VegetationComponents / LodScene.
	FriendsConifer(ShowFriendsConifer),
	/// High Bush Shoots via VegetationComponents / LodScene.
	HighBushShoots(ShowHighBushShoots),
	/// Jungle Growth via VegetationComponents / LodScene.
	JungleGrowth(ShowJungleGrowth),
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowSopesBanyan {
	#[command(flatten)]
	pub tree: SopesBanyanParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowPenmarchTorch {
	#[command(flatten)]
	pub tree: PenmarchTorchParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowKamakuraTorch {
	#[command(flatten)]
	pub tree: KamakuraTorchParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowRorysHeadTrained {
	#[command(flatten)]
	pub tree: RorysHeadTrainedParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowStorybookTree {
	#[command(flatten)]
	pub tree: StorybookTreeParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowVaseTree {
	#[command(flatten)]
	pub tree: VaseTreeParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowNorthernConifer {
	#[command(flatten)]
	pub tree: NorthernConiferParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowLiamsConifer {
	#[command(flatten)]
	pub tree: LiamsConiferParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowTemperateConifer {
	#[command(flatten)]
	pub tree: TemperateConiferParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowHonuBanyan {
	#[command(flatten)]
	pub tree: HonuBanyanParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowJungleStorybookTree {
	#[command(flatten)]
	pub tree: JungleStorybookTreeParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowBraidOakTree {
	#[command(flatten)]
	pub tree: BraidOakTreeParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowSimplemansHedge {
	#[command(flatten)]
	pub hedge: SimplemansHedgeParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowTuftPatch {
	#[command(flatten)]
	pub patch: TuftPatchParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowPalmCrown {
	#[command(flatten)]
	pub crown: PalmCrownParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowDatePalm {
	#[command(flatten)]
	pub tree: DatePalmParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowWaialeaPalm {
	#[command(flatten)]
	pub tree: WaialeaPalmParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowPalmBush {
	#[command(flatten)]
	pub bush: PalmBushParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowMonsterGrass {
	#[command(flatten)]
	pub grass: MonsterGrassParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowMonsterGrass {
	fn configured(self) -> MonsterGrassParams {
		let mut grass = self.grass;
		let cell = grass.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grass.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grass
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowBraidGrass {
	#[command(flatten)]
	pub grass: BraidGrassParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowBraidGrass {
	fn configured(self) -> BraidGrassParams {
		let mut grass = self.grass;
		let cell = grass.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grass.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grass
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowTropicalTufts {
	#[command(flatten)]
	pub grass: TropicalTuftsParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowTropicalTufts {
	fn configured(self) -> TropicalTuftsParams {
		let mut grass = self.grass;
		let cell = grass.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grass.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grass
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowCommonTufts {
	#[command(flatten)]
	pub grass: CommonTuftsParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowCommonTufts {
	fn configured(self) -> CommonTuftsParams {
		let mut grass = self.grass;
		let cell = grass.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grass.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grass
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowTallGrass {
	#[command(flatten)]
	pub grass: TallGrassParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowTallGrass {
	fn configured(self) -> TallGrassParams {
		let mut grass = self.grass;
		let cell = grass.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grass.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grass
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowWildGrass {
	#[command(flatten)]
	pub grass: WildGrassParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowWildGrass {
	fn configured(self) -> WildGrassParams {
		let mut grass = self.grass;
		let cell = grass.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grass.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grass
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowBushScrub {
	#[command(flatten)]
	pub grove: BushScrubParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowBushScrub {
	fn configured(self) -> BushScrubParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowTropicalUndergrowth {
	#[command(flatten)]
	pub grove: TropicalUndergrowthParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowTropicalUndergrowth {
	fn configured(self) -> TropicalUndergrowthParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowLevantineScrub {
	#[command(flatten)]
	pub grove: LevantineScrubParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowLevantineScrub {
	fn configured(self) -> LevantineScrubParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowStrangeOasis {
	#[command(flatten)]
	pub grove: StrangeOasisParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowStrangeOasis {
	fn configured(self) -> StrangeOasisParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowTropicalThicket {
	#[command(flatten)]
	pub grove: TropicalThicketParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowTropicalThicket {
	fn configured(self) -> TropicalThicketParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowRollingOaks {
	#[command(flatten)]
	pub grove: RollingOaksParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowRollingOaks {
	fn configured(self) -> RollingOaksParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowOrchard {
	#[command(flatten)]
	pub grove: OrchardParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowOrchard {
	fn configured(self) -> OrchardParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowVast {
	/// Grove construction kebab-case name (`orchard`, `goettingen-follow`, `rolling-oaks`, …).
	#[arg(long, value_parser = parse_vast_grove_name)]
	pub grove_name: String,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowForest {
	/// Pin a well-known layering (`lush-jungle`, `ag-town`, …). Omit to Hopscotch.
	#[arg(value_parser = parse_layering_kind, value_name = "LAYERING")]
	pub layering: Option<LayeringKind>,

	/// Hopscotch / layer-throw noise (`seed,frequency,amplitude,octaves[,type]`).
	#[arg(
		long,
		default_value = DEFAULT_FOREST_NOISE,
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]"
	)]
	pub noise: NoiseParams,

	/// Present-ring multiplier (`1` = 1 km present / 3 km generate; `0` = one 100 m tile).
	#[arg(long, default_value_t = DEFAULT_FOREST_STREAM_RADIUS)]
	pub stream_radius: u32,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowRiparianGeneral {
	#[command(flatten)]
	pub grove: RiparianGeneralParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowRiparianGeneral {
	fn configured(self) -> RiparianGeneralParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowForlornSavanna {
	#[command(flatten)]
	pub grove: ForlornSavannaParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowForlornSavanna {
	fn configured(self) -> ForlornSavannaParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowGoettingenFollow {
	#[command(flatten)]
	pub grove: GoettingenFollowParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowGoettingenFollow {
	fn configured(self) -> GoettingenFollowParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowVineyard {
	#[command(flatten)]
	pub grove: VineyardParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowVineyard {
	fn configured(self) -> VineyardParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowDryland {
	#[command(flatten)]
	pub grove: DrylandParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowDryland {
	fn configured(self) -> DrylandParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowLeeward {
	#[command(flatten)]
	pub grove: LeewardParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowLeeward {
	fn configured(self) -> LeewardParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowTemperateLowerMassives {
	#[command(flatten)]
	pub grove: TemperateLowerMassivesParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowTemperateLowerMassives {
	fn configured(self) -> TemperateLowerMassivesParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowTemperateMassives {
	#[command(flatten)]
	pub grove: TemperateMassivesParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowTemperateMassives {
	fn configured(self) -> TemperateMassivesParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowStorytellers {
	#[command(flatten)]
	pub grove: StorytellersParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowStorytellers {
	fn configured(self) -> StorytellersParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowWanderingAcacia {
	#[command(flatten)]
	pub grove: WanderingAcaciaParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowWanderingAcacia {
	fn configured(self) -> WanderingAcaciaParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowTradeWinds {
	#[command(flatten)]
	pub grove: TradeWindsParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowTradeWinds {
	fn configured(self) -> TradeWindsParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowHighBush {
	#[command(flatten)]
	pub grove: HighBushParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowHighBush {
	fn configured(self) -> HighBushParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowSpottyBushes {
	#[command(flatten)]
	pub grove: SpottyBushesParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowSpottyBushes {
	fn configured(self) -> SpottyBushesParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowRiverineGreen {
	#[command(flatten)]
	pub grove: RiverineGreenParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowRiverineGreen {
	fn configured(self) -> RiverineGreenParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowLowBush {
	#[command(flatten)]
	pub grove: LowBushParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowLowBush {
	fn configured(self) -> LowBushParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowJungleMassives {
	#[command(flatten)]
	pub grove: JungleMassivesParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowJungleMassives {
	fn configured(self) -> JungleMassivesParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowJungleLowerMassives {
	#[command(flatten)]
	pub grove: JungleLowerMassivesParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowJungleLowerMassives {
	fn configured(self) -> JungleLowerMassivesParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowUnendingJungle {
	#[command(flatten)]
	pub grove: UnendingJungleParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowUnendingJungle {
	fn configured(self) -> UnendingJungleParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowJerrysChaparral {
	#[command(flatten)]
	pub grove: JerrysChaparralParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowJerrysChaparral {
	fn configured(self) -> JerrysChaparralParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowRiparianMix {
	#[command(flatten)]
	pub grove: RiparianMixParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowRiparianMix {
	fn configured(self) -> RiparianMixParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowAlpine {
	#[command(flatten)]
	pub grove: AlpineParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowAlpine {
	fn configured(self) -> AlpineParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowChristmasTaiga {
	#[command(flatten)]
	pub grove: ChristmasTaigaParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowChristmasTaiga {
	fn configured(self) -> ChristmasTaigaParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowConiferSapling {
	#[command(flatten)]
	pub grove: ConiferSaplingParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowConiferSapling {
	fn configured(self) -> ConiferSaplingParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowAridConiferSapling {
	#[command(flatten)]
	pub grove: AridConiferSaplingParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowAridConiferSapling {
	fn configured(self) -> AridConiferSaplingParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowConiferMassives {
	#[command(flatten)]
	pub grove: ConiferMassivesParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowConiferMassives {
	fn configured(self) -> ConiferMassivesParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowPalmShade {
	#[command(flatten)]
	pub grove: PalmShadeParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowPalmShade {
	fn configured(self) -> PalmShadeParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowShamanhome {
	#[command(flatten)]
	pub grove: ShamanhomeParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowShamanhome {
	fn configured(self) -> ShamanhomeParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowDateGrove {
	#[command(flatten)]
	pub grove: DateGroveParams,

	/// Square preview extent (m) on XZ; at least one authored cell.
	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl ShowDateGrove {
	fn configured(self) -> DateGroveParams {
		let mut grove = self.grove;
		let cell = grove.cell_extent_xz();
		let span = self.grove_extent_xz.max(cell.x).max(cell.y);
		grove.extent = GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span));
		grove
	}
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowFriendsConifer {
	#[command(flatten)]
	pub tree: FriendsConiferParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowHighBushShoots {
	#[command(flatten)]
	pub bush: HighBushShootsParams,
}

#[derive(Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowJungleGrowth {
	#[command(flatten)]
	pub growth: JungleGrowthParams,
}

impl Show {
	pub fn react(self, commands: &mut Commands) {
		let subject = match self {
			Self::SopesBanyan(args) => ShowSubject::SopesBanyan(args.tree),
			Self::PenmarchTorch(args) => ShowSubject::PenmarchTorch(args.tree),
			Self::KamakuraTorch(args) => ShowSubject::KamakuraTorch(args.tree),
			Self::RorysHeadTrained(args) => ShowSubject::RorysHeadTrained(args.tree),
			Self::StorybookTree(args) => ShowSubject::StorybookTree(args.tree),
			Self::VaseTree(args) => ShowSubject::VaseTree(args.tree),
			Self::NorthernConifer(args) => ShowSubject::NorthernConifer(args.tree),
			Self::LiamsConifer(args) => ShowSubject::LiamsConifer(args.tree),
			Self::TemperateConifer(args) => ShowSubject::TemperateConifer(args.tree),
			Self::HonuBanyan(args) => ShowSubject::HonuBanyan(args.tree),
			Self::JungleStorybookTree(args) => ShowSubject::JungleStorybookTree(args.tree),
			Self::BraidOakTree(args) => ShowSubject::BraidOakTree(args.tree),
			Self::SimplemansHedge(args) => ShowSubject::SimplemansHedge(args.hedge),
			Self::TuftPatch(args) => ShowSubject::TuftPatch(args.patch),
			Self::PalmCrown(args) => ShowSubject::PalmCrown(args.crown),
			Self::DatePalm(args) => ShowSubject::DatePalm(args.tree),
			Self::WaialeaPalm(args) => ShowSubject::WaialeaPalm(args.tree),
			Self::PalmBush(args) => ShowSubject::PalmBush(args.bush),
			Self::MonsterGrass(args) => ShowSubject::MonsterGrass(args.configured()),
			Self::MonsterGrassPlains => ShowSubject::MonsterGrassPlains,
			Self::BraidGrass(args) => ShowSubject::BraidGrass(args.configured()),
			Self::TropicalTufts(args) => ShowSubject::TropicalTufts(args.configured()),
			Self::CommonTufts(args) => ShowSubject::CommonTufts(args.configured()),
			Self::TallGrass(args) => ShowSubject::TallGrass(args.configured()),
			Self::WildGrass(args) => ShowSubject::WildGrass(args.configured()),
			Self::BushScrub(args) => ShowSubject::BushScrub(args.configured()),
			Self::TropicalUndergrowth(args) => ShowSubject::TropicalUndergrowth(args.configured()),
			Self::LevantineScrub(args) => ShowSubject::LevantineScrub(args.configured()),
			Self::StrangeOasis(args) => ShowSubject::StrangeOasis(args.configured()),
			Self::TropicalThicket(args) => ShowSubject::TropicalThicket(args.configured()),
			Self::RollingOaks(args) => ShowSubject::RollingOaks(args.configured()),
			Self::Orchard(args) => ShowSubject::Orchard(args.configured()),
			Self::Vast(args) => ShowSubject::Vast { grove_name: args.grove_name },
			Self::VastOrchards => ShowSubject::Vast { grove_name: "orchard".into() },
			Self::Forest(args) => ShowSubject::Forest {
				noise: args.noise,
				stream_radius: args.stream_radius,
				layering: args.layering,
			},
			Self::RiparianGeneral(args) => ShowSubject::RiparianGeneral(args.configured()),
			Self::ForlornSavanna(args) => ShowSubject::ForlornSavanna(args.configured()),
			Self::GoettingenFollow(args) => ShowSubject::GoettingenFollow(args.configured()),
			Self::Vineyard(args) => ShowSubject::Vineyard(args.configured()),
			Self::Dryland(args) => ShowSubject::Dryland(args.configured()),
			Self::Leeward(args) => ShowSubject::Leeward(args.configured()),
			Self::TemperateLowerMassives(args) => {
				ShowSubject::TemperateLowerMassives(args.configured())
			}
			Self::TemperateMassives(args) => ShowSubject::TemperateMassives(args.configured()),
			Self::Storytellers(args) => ShowSubject::Storytellers(args.configured()),
			Self::WanderingAcacia(args) => ShowSubject::WanderingAcacia(args.configured()),
			Self::TradeWinds(args) => ShowSubject::TradeWinds(args.configured()),
			Self::HighBush(args) => ShowSubject::HighBush(args.configured()),
			Self::SpottyBushes(args) => ShowSubject::SpottyBushes(args.configured()),
			Self::RiverineGreen(args) => ShowSubject::RiverineGreen(args.configured()),
			Self::LowBush(args) => ShowSubject::LowBush(args.configured()),
			Self::JungleMassives(args) => ShowSubject::JungleMassives(args.configured()),
			Self::JungleLowerMassives(args) => ShowSubject::JungleLowerMassives(args.configured()),
			Self::UnendingJungle(args) => ShowSubject::UnendingJungle(args.configured()),
			Self::JerrysChaparral(args) => ShowSubject::JerrysChaparral(args.configured()),
			Self::RiparianMix(args) => ShowSubject::RiparianMix(args.configured()),
			Self::Alpine(args) => ShowSubject::Alpine(args.configured()),
			Self::ChristmasTaiga(args) => ShowSubject::ChristmasTaiga(args.configured()),
			Self::ConiferSapling(args) => ShowSubject::ConiferSapling(args.configured()),
			Self::AridConiferSapling(args) => ShowSubject::AridConiferSapling(args.configured()),
			Self::ConiferMassives(args) => ShowSubject::ConiferMassives(args.configured()),
			Self::PalmShade(args) => ShowSubject::PalmShade(args.configured()),
			Self::Shamanhome(args) => ShowSubject::Shamanhome(args.configured()),
			Self::DateGrove(args) => ShowSubject::DateGrove(args.configured()),
			Self::FriendsConifer(args) => ShowSubject::FriendsConifer(args.tree),
			Self::HighBushShoots(args) => ShowSubject::HighBushShoots(args.bush),
			Self::JungleGrowth(args) => ShowSubject::JungleGrowth(args.growth),
		};
		commands.insert_resource(ShowConfig { subject: Some(subject) });
	}
}

#[derive(Resource, Default)]
pub struct ShowConfig {
	pub subject: Option<ShowSubject>,
}

#[derive(Clone, Debug)]
pub enum ShowSubject {
	SopesBanyan(SopesBanyanParams),
	PenmarchTorch(PenmarchTorchParams),
	KamakuraTorch(KamakuraTorchParams),
	RorysHeadTrained(RorysHeadTrainedParams),
	StorybookTree(StorybookTreeParams),
	VaseTree(VaseTreeParams),
	NorthernConifer(NorthernConiferParams),
	LiamsConifer(LiamsConiferParams),
	TemperateConifer(TemperateConiferParams),
	HonuBanyan(HonuBanyanParams),
	JungleStorybookTree(JungleStorybookTreeParams),
	BraidOakTree(BraidOakTreeParams),
	SimplemansHedge(SimplemansHedgeParams),
	TuftPatch(TuftPatchParams),
	PalmCrown(PalmCrownParams),
	DatePalm(DatePalmParams),
	WaialeaPalm(WaialeaPalmParams),
	PalmBush(PalmBushParams),
	MonsterGrass(MonsterGrassParams),
	MonsterGrassPlains,
	BraidGrass(BraidGrassParams),
	TropicalTufts(TropicalTuftsParams),
	CommonTufts(CommonTuftsParams),
	TallGrass(TallGrassParams),
	WildGrass(WildGrassParams),
	BushScrub(BushScrubParams),
	TropicalUndergrowth(TropicalUndergrowthParams),
	LevantineScrub(LevantineScrubParams),
	StrangeOasis(StrangeOasisParams),
	TropicalThicket(TropicalThicketParams),
	RollingOaks(RollingOaksParams),
	Orchard(OrchardParams),
	Vast { grove_name: String },
	Forest { noise: NoiseParams, stream_radius: u32, layering: Option<LayeringKind> },
	RiparianGeneral(RiparianGeneralParams),
	ForlornSavanna(ForlornSavannaParams),
	GoettingenFollow(GoettingenFollowParams),
	Vineyard(VineyardParams),
	Dryland(DrylandParams),
	Leeward(LeewardParams),
	TemperateLowerMassives(TemperateLowerMassivesParams),
	TemperateMassives(TemperateMassivesParams),
	Storytellers(StorytellersParams),
	WanderingAcacia(WanderingAcaciaParams),
	TradeWinds(TradeWindsParams),
	HighBush(HighBushParams),
	SpottyBushes(SpottyBushesParams),
	RiverineGreen(RiverineGreenParams),
	LowBush(LowBushParams),
	JungleMassives(JungleMassivesParams),
	JungleLowerMassives(JungleLowerMassivesParams),
	UnendingJungle(UnendingJungleParams),
	JerrysChaparral(JerrysChaparralParams),
	RiparianMix(RiparianMixParams),
	Alpine(AlpineParams),
	ChristmasTaiga(ChristmasTaigaParams),
	ConiferSapling(ConiferSaplingParams),
	AridConiferSapling(AridConiferSaplingParams),
	ConiferMassives(ConiferMassivesParams),
	PalmShade(PalmShadeParams),
	Shamanhome(ShamanhomeParams),
	DateGrove(DateGroveParams),
	FriendsConifer(FriendsConiferParams),
	HighBushShoots(HighBushShootsParams),
	JungleGrowth(JungleGrowthParams),
}

#[derive(Component)]
pub struct ShowRoot;

fn spawn_show_tree<T>(commands: &mut Commands, tree: &T)
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
{
	let bounds = vegetation_bounds(tree);
	let entities = spawn_flattened_placed_vegetation(commands, tree, Transform::IDENTITY, bounds);
	for entity in entities {
		commands.entity(entity).insert(ShowRoot);
	}
}

fn spawn_show_grove<T>(commands: &mut Commands, grove: &T)
where
	T: LodScene + VegetationComponents + Component + Clone + Send + Sync + 'static,
{
	let bounds = grove
		.structural_lod()
		.map(|p| p.footprint_aabb())
		.unwrap_or_else(|| vegetation_bounds(grove));
	let entities = spawn_lod_scene_host(commands, grove, Transform::IDENTITY, bounds);
	for entity in entities {
		commands.entity(entity).insert(ShowRoot);
	}
}

/// Present `/show` subjects when `ShowConfig` changes. Clears legacy `/render` roots.
pub fn sync_show(
	mut commands: Commands,
	config: Res<ShowConfig>,
	show_roots: Query<Entity, With<ShowRoot>>,
	render_roots: Query<Entity, (With<SbsRenderItem>, Without<ChildOf>)>,
	mut last: Local<Option<String>>,
) {
	let key = match &config.subject {
		None => None,
		Some(ShowSubject::SopesBanyan(t)) => Some(format!("sopes-banyan:{:?}", t.geometry)),
		Some(ShowSubject::PenmarchTorch(t)) => Some(format!("penmarch-torch:{:?}", t.geometry)),
		Some(ShowSubject::KamakuraTorch(t)) => Some(format!("kamakura-torch:{:?}", t.geometry)),
		Some(ShowSubject::RorysHeadTrained(t)) => {
			Some(format!("rorys-head-trained:{:?}", t.geometry))
		}
		Some(ShowSubject::StorybookTree(t)) => Some(format!("storybook-tree:{:?}", t.geometry)),
		Some(ShowSubject::VaseTree(t)) => Some(format!("vase-tree:{:?}", t.geometry)),
		Some(ShowSubject::NorthernConifer(t)) => Some(format!(
			"northern-conifer:{:?}|splay={}|spawn={}|apex={}",
			t.geometry,
			t.splay_radius_fraction_of_height,
			t.splay_spawn_fraction,
			t.apex_canopy_spawn_fraction
		)),
		Some(ShowSubject::LiamsConifer(t)) => Some(format!("liams-conifer:{:?}", t.geometry)),
		Some(ShowSubject::TemperateConifer(t)) => Some(format!(
			"temperate-conifer:{:?}|fronds={:?}|len={:?}|spawn={}",
			t.geometry.inner, t.fronds_per_joint, t.frond_length_fraction, t.frond_spawn_fraction
		)),
		Some(ShowSubject::HonuBanyan(t)) => {
			Some(format!("honu-banyan:{:?}|growth={}", t.geometry, t.growth_spawn_fraction))
		}
		Some(ShowSubject::JungleStorybookTree(t)) => Some(format!(
			"jungle-storybook-tree:{:?}|growth={}",
			t.geometry, t.growth_spawn_fraction
		)),
		Some(ShowSubject::BraidOakTree(t)) => {
			Some(format!("braid-oak-tree:{:?}|stick={:?}", t.geometry, t.stick_surface_noise))
		}
		Some(ShowSubject::SimplemansHedge(t)) => Some(format!(
			"simplemans-hedge:h={}|xz={}|d={}|seed={}|clumps={}",
			t.height, t.footprint_xz, t.density, t.seed, t.clump_count
		)),
		Some(ShowSubject::TuftPatch(t)) => Some(format!(
			"tuft-patch:{:?}|clumps={}|patch_extent_xz={}",
			t.shape, t.clump_count, t.patch_extent_xz
		)),
		Some(ShowSubject::PalmCrown(t)) => Some(format!(
			"palm-crown:{:?}|rings={}|spacing={}",
			t.shape, t.ring_count, t.ring_spacing
		)),
		Some(ShowSubject::DatePalm(t)) => Some(format!("date-palm:{:?}", t.geometry)),
		Some(ShowSubject::WaialeaPalm(t)) => Some(format!("waialea-palm:{:?}", t.geometry)),
		Some(ShowSubject::PalmBush(t)) => Some(format!("palm-bush:{:?}", t.geometry)),
		Some(ShowSubject::MonsterGrass(g)) => Some(format!(
			"monster-grass:extent={:?}|cell={:?}|terrain={:?}|foliage={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain,
			g.foliage_noise
		)),
		Some(ShowSubject::MonsterGrassPlains) => Some("monster-grass-plains".into()),
		Some(ShowSubject::BraidGrass(g)) => Some(format!(
			"braid-grass:extent={:?}|cell={:?}|terrain={:?}|merge={}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain,
			g.merge_collections
		)),
		Some(ShowSubject::TropicalTufts(g)) => Some(format!(
			"tropical-tufts:extent={:?}|cell={:?}|terrain={:?}|merge={}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain,
			g.merge_collections
		)),
		Some(ShowSubject::CommonTufts(g)) => Some(format!(
			"common-tufts:extent={:?}|cell={:?}|terrain={:?}|merge={}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain,
			g.merge_collections
		)),
		Some(ShowSubject::TallGrass(g)) => Some(format!(
			"tall-grass:extent={:?}|cell={:?}|terrain={:?}|merge={}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain,
			g.merge_collections
		)),
		Some(ShowSubject::WildGrass(g)) => Some(format!(
			"wild-grass:extent={:?}|cell={:?}|terrain={:?}|merge={}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain,
			g.merge_collections
		)),
		Some(ShowSubject::BushScrub(g)) => Some(format!(
			"bush-scrub:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::TropicalUndergrowth(g)) => Some(format!(
			"tropical-undergrowth:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::LevantineScrub(g)) => Some(format!(
			"levantine-scrub:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::StrangeOasis(g)) => Some(format!(
			"strange-oasis:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::TropicalThicket(g)) => Some(format!(
			"tropical-thicket:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::RollingOaks(g)) => Some(format!(
			"rolling-oaks:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::Orchard(g)) => Some(format!(
			"orchard:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::Vast { grove_name }) => Some(format!("vast:{grove_name}")),
		Some(ShowSubject::Forest { noise, stream_radius, layering }) => {
			let layering = layering.map(LayeringKind::as_kebab).unwrap_or("hopscotch");
			Some(format!("forest:{layering}|{noise:?}|r={stream_radius}"))
		}
		Some(ShowSubject::RiparianGeneral(g)) => Some(format!(
			"riparian-general:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::ForlornSavanna(g)) => Some(format!(
			"forlorn-savanna:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::GoettingenFollow(g)) => Some(format!(
			"goettingen-follow:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::Vineyard(g)) => Some(format!(
			"vineyard:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::Dryland(g)) => Some(format!(
			"dryland:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::Leeward(g)) => Some(format!(
			"leeward:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::TemperateLowerMassives(g)) => Some(format!(
			"temperate-lower-massives:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::TemperateMassives(g)) => Some(format!(
			"temperate-massives:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::Storytellers(g)) => Some(format!(
			"storytellers:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::WanderingAcacia(g)) => Some(format!(
			"wandering-acacia:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::TradeWinds(g)) => Some(format!(
			"trade-winds:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::HighBush(g)) => Some(format!(
			"high-bush:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::SpottyBushes(g)) => Some(format!(
			"spotty-bushes:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::RiverineGreen(g)) => Some(format!(
			"riverine-green:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::LowBush(g)) => Some(format!(
			"low-bush:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::JungleMassives(g)) => Some(format!(
			"jungle-massives:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::JungleLowerMassives(g)) => Some(format!(
			"jungle-lower-massives:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::UnendingJungle(g)) => Some(format!(
			"unending-jungle:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::JerrysChaparral(g)) => Some(format!(
			"jerrys-chaparral:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::RiparianMix(g)) => Some(format!(
			"riparian-mix:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::Alpine(g)) => Some(format!(
			"alpine:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::ChristmasTaiga(g)) => Some(format!(
			"christmas-taiga:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::ConiferSapling(g)) => Some(format!(
			"conifer-sapling:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::AridConiferSapling(g)) => Some(format!(
			"arid-conifer-sapling:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::ConiferMassives(g)) => Some(format!(
			"conifer-massives:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::PalmShade(g)) => Some(format!(
			"palm-shade:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::Shamanhome(g)) => Some(format!(
			"shamanhome:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::DateGrove(g)) => Some(format!(
			"date-grove:extent={:?}|cell={:?}|terrain={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain
		)),
		Some(ShowSubject::FriendsConifer(t)) => Some(format!(
			"friends-conifer:{:?}|splay={}",
			t.geometry, t.splay_radius_fraction_of_height
		)),
		Some(ShowSubject::HighBushShoots(b)) => Some(format!("high-bush-shoots:{:?}", b.shape)),
		Some(ShowSubject::JungleGrowth(g)) => Some(format!("jungle-growth:{:?}", g.shape)),
	};
	if key == *last && show_roots.iter().next().is_some() {
		return;
	}
	for entity in &show_roots {
		commands.entity(entity).despawn();
	}
	*last = key.clone();
	let Some(subject) = &config.subject else {
		return;
	};

	for entity in &render_roots {
		commands.entity(entity).despawn();
	}

	match subject {
		ShowSubject::SopesBanyan(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::PenmarchTorch(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::KamakuraTorch(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::RorysHeadTrained(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::StorybookTree(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::VaseTree(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::NorthernConifer(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::LiamsConifer(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::TemperateConifer(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::HonuBanyan(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::JungleStorybookTree(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::BraidOakTree(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::SimplemansHedge(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::TuftPatch(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::PalmCrown(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::DatePalm(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::WaialeaPalm(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::PalmBush(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::MonsterGrass(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::MonsterGrassPlains => {
			for entity in spawn_monster_grass_plain(&mut commands, Transform::IDENTITY) {
				commands.entity(entity).insert(ShowRoot);
			}
		}
		ShowSubject::BraidGrass(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::TropicalTufts(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::CommonTufts(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::TallGrass(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::WildGrass(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::BushScrub(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::TropicalUndergrowth(params) => {
			spawn_show_grove(&mut commands, &params.build())
		}
		ShowSubject::LevantineScrub(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::StrangeOasis(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::TropicalThicket(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::RollingOaks(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::Orchard(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::Vast { grove_name } => {
			match spawn_vast_grove(&mut commands, Transform::IDENTITY, grove_name) {
				Ok(entities) => {
					for entity in entities {
						commands.entity(entity).insert(ShowRoot);
					}
				}
				Err(msg) => bevy::log::error!("show vast: {msg}"),
			}
		}
		ShowSubject::Forest { .. } => {}
		ShowSubject::RiparianGeneral(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::ForlornSavanna(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::GoettingenFollow(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::Vineyard(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::Dryland(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::Leeward(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::TemperateLowerMassives(params) => {
			spawn_show_grove(&mut commands, &params.build())
		}
		ShowSubject::TemperateMassives(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::Storytellers(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::WanderingAcacia(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::TradeWinds(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::HighBush(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::SpottyBushes(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::RiverineGreen(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::LowBush(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::JungleMassives(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::JungleLowerMassives(params) => {
			spawn_show_grove(&mut commands, &params.build())
		}
		ShowSubject::UnendingJungle(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::JerrysChaparral(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::RiparianMix(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::Alpine(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::ChristmasTaiga(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::ConiferSapling(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::AridConiferSapling(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::ConiferMassives(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::PalmShade(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::Shamanhome(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::DateGrove(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::FriendsConifer(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::HighBushShoots(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::JungleGrowth(params) => spawn_show_tree(&mut commands, &params.build()),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn show_monster_grass_configures_extent_and_builds() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"show monster-grass --elevation 0.35 --grove-extent-xz 25 --merge-collections 100",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Show(Show::MonsterGrass(args)) = cmd else {
			anyhow::bail!("expected show monster-grass command");
		};
		assert!((args.grove_extent_xz - 25.0).abs() < 1e-5);
		let grass = args.configured();
		assert!((grass.terrain.elevation - 0.35).abs() < 1e-5);
		assert!(!grass.placements().is_empty());
		assert!(!grass.build().plants.is_empty());
		Ok(())
	}

	#[test]
	fn show_monster_grass_plains_parses() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("show monster-grass-plains")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(matches!(cmd, crate::commands::PlaygroundCommand::Show(Show::MonsterGrassPlains)));
		Ok(())
	}

	#[test]
	fn show_vast_orchards_parses() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("show vast-orchards")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert!(matches!(cmd, crate::commands::PlaygroundCommand::Show(Show::VastOrchards)));
		Ok(())
	}

	#[test]
	fn show_vast_parses_grove_name() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("show vast --grove-name orchard")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Show(Show::Vast(args)) = cmd else {
			anyhow::bail!("expected show vast command");
		};
		assert_eq!(args.grove_name, "orchard");
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"show vast --grove-name goettingen-follow",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Show(Show::Vast(args)) = cmd else {
			anyhow::bail!("expected show vast command");
		};
		assert_eq!(args.grove_name, "goettingen-follow");
		Ok(())
	}

	#[test]
	fn show_forest_parses_defaults_and_overrides() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("show forest")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Show(Show::Forest(args)) = cmd else {
			anyhow::bail!("expected show forest command");
		};
		assert_eq!(args.stream_radius, DEFAULT_FOREST_STREAM_RADIUS);
		assert_eq!(args.noise.seed, 1337);
		assert!((args.noise.frequency - 0.0005).abs() < 1e-8);
		assert!(args.layering.is_none());
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"show forest --stream-radius 0 --noise 3,0.005,1,1",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Show(Show::Forest(args)) = cmd else {
			anyhow::bail!("expected show forest command");
		};
		assert_eq!(args.stream_radius, 0);
		assert_eq!(args.noise.seed, 3);
		assert!((args.noise.frequency - 0.005).abs() < 1e-8);
		assert!(args.layering.is_none());
		let cmd = crate::commands::PlaygroundCommand::parse_line("show forest lush-jungle")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Show(Show::Forest(args)) = cmd else {
			anyhow::bail!("expected show forest lush-jungle command");
		};
		assert_eq!(args.layering, Some(chico_forests::LayeringKind::LushJungle));
		Ok(())
	}

	#[test]
	fn show_vast_rejects_unknown_grove() -> Result<()> {
		match crate::commands::PlaygroundCommand::parse_line("show vast --grove-name not-a-grove") {
			Ok(_) => anyhow::bail!("unknown grove should fail parse"),
			Err(err) => {
				assert!(err.contains("unknown grove") || err.contains("not-a-grove"));
				Ok(())
			}
		}
	}

	#[test]
	fn show_refactored_groves_parse_and_build() -> Result<()> {
		for line in [
			"show braid-grass --grove-extent-xz 12.75 --elevation 0.4",
			"show tropical-tufts --grove-extent-xz 26 --elevation 0.4",
			"show common-tufts --grove-extent-xz 8 --elevation 0.4",
			"show tall-grass --grove-extent-xz 14 --elevation 0.40",
			"show wild-grass --grove-extent-xz 14 --elevation 0.35",
			"show bush-scrub --grove-extent-xz 35 --elevation 0.40",
			"show tropical-undergrowth --grove-extent-xz 35 --elevation 0.35",
			"show levantine-scrub --grove-extent-xz 20",
			"show strange-oasis --grove-extent-xz 39",
			"show tropical-thicket --grove-extent-xz 20",
			"show rolling-oaks --grove-extent-xz 260 --elevation 0.40",
			"show orchard --grove-extent-xz 160",
			"show riparian-general --grove-extent-xz 200",
			"show forlorn-savanna --grove-extent-xz 300",
			"show goettingen-follow --grove-extent-xz 39 --elevation 0.25",
			"show vineyard --grove-extent-xz 90 --elevation 0.35",
			"show dryland --grove-extent-xz 280",
			"show leeward --grove-extent-xz 220",
			"show temperate-lower-massives --grove-extent-xz 92 --elevation 0.35",
			"show temperate-massives --grove-extent-xz 400",
			"show storytellers --grove-extent-xz 220",
			"show wandering-acacia --grove-extent-xz 300",
			"show trade-winds --grove-extent-xz 260",
			"show high-bush --grove-extent-xz 46 --elevation 0.35",
			"show spotty-bushes --grove-extent-xz 39 --elevation 0.35",
			"show riverine-green --grove-extent-xz 28 --elevation 0.25",
			"show low-bush --grove-extent-xz 34 --elevation 0.30",
			"show jungle-massives --grove-extent-xz 220",
			"show jungle-lower-massives --grove-extent-xz 92",
			"show unending-jungle --grove-extent-xz 39 --elevation 0.35",
			"show jerrys-chaparral --grove-extent-xz 39 --elevation 0.35",
			"show riparian-mix --grove-extent-xz 180",
			"show alpine --grove-extent-xz 220",
			"show christmas-taiga --grove-extent-xz 200",
			"show conifer-sapling --grove-extent-xz 39 --elevation 0.55",
			"show arid-conifer-sapling --grove-extent-xz 39",
			"show conifer-massives --grove-extent-xz 400",
			"show palm-shade --grove-extent-xz 220",
			"show shamanhome --grove-extent-xz 39 --elevation 0.25",
			"show date-grove --grove-extent-xz 160",
			"show friends-conifer",
			"show high-bush-shoots",
			"show jungle-growth",
		] {
			let cmd = crate::commands::PlaygroundCommand::parse_line(line)
				.map_err(|e| anyhow::anyhow!("{line}: {e}"))?;
			match cmd {
				crate::commands::PlaygroundCommand::Show(Show::BraidGrass(args)) => {
					assert!(!args.configured().build().plants().is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::TropicalTufts(args)) => {
					assert!(!args.configured().placements().is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::CommonTufts(args)) => {
					assert!(!args.configured().build().plants().is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::TallGrass(args)) => {
					assert!(!args.configured().build().plants().is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::WildGrass(args)) => {
					assert!(!args.configured().build().plants().is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::BushScrub(args)) => {
					assert!(!args.configured().build().is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::TropicalUndergrowth(args)) => {
					assert!(!args.configured().build().is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::LevantineScrub(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::StrangeOasis(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::TropicalThicket(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::RollingOaks(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::Orchard(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::RiparianGeneral(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::ForlornSavanna(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::GoettingenFollow(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::Vineyard(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::Dryland(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::Leeward(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::TemperateLowerMassives(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::TemperateMassives(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::Storytellers(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::WanderingAcacia(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::TradeWinds(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::HighBush(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::SpottyBushes(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::RiverineGreen(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::LowBush(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::JungleMassives(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::JungleLowerMassives(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::UnendingJungle(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::JerrysChaparral(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::RiparianMix(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::Alpine(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::ChristmasTaiga(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::ConiferSapling(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::AridConiferSapling(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::ConiferMassives(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::PalmShade(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::Shamanhome(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::DateGrove(args)) => {
					assert!(!args.configured().build().plants.is_empty());
				}
				crate::commands::PlaygroundCommand::Show(Show::FriendsConifer(args)) => {
					let _ = args.tree.build();
				}
				crate::commands::PlaygroundCommand::Show(Show::HighBushShoots(args)) => {
					let _ = args.bush.build();
				}
				crate::commands::PlaygroundCommand::Show(Show::JungleGrowth(args)) => {
					let _ = args.growth.build();
				}
				_ => anyhow::bail!("unexpected command for {line}"),
			}
		}
		Ok(())
	}
}
