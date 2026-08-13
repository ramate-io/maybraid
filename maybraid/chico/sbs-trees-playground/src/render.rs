use bevy::prelude::*;
use chico_ball_components::tuft::{
	BladeTuft, BuddhaHandTuft, SpearTuft, SucculentTuft, WeepingTuft,
};
use chico_ball_components::{FrondCrown, ModerateLodFrondCrown};
use chico_grove_render_items::alpine::AlpineStd;
use chico_grove_render_items::arid_conifer_sapling::AridConiferSaplingStd;
use chico_grove_render_items::braid_grass::BraidGrassStd;
use chico_grove_render_items::bush_scrub::BushScrubStd;
use chico_grove_render_items::christmas_taiga::ChristmasTaigaStd;
use chico_grove_render_items::common_tufts::CommonTuftsStd;
use chico_grove_render_items::conifer_massives::ConiferMassivesStd;
use chico_grove_render_items::conifer_sapling::ConiferSaplingStd;
use chico_grove_render_items::date_grove::DateGroveStd;
use chico_grove_render_items::jerrys_chaparral::JerrysChaparralStd;
use chico_groves::{
	DrylandParams, ForlornSavannaParams, GoettingenFollowParams, HighBushParams,
	JungleLowerMassivesParams, JungleMassivesParams, LeewardParams, LevantineScrubParams,
	LowBushParams, MonsterGrassParams, OrchardParams, RiparianGeneralParams, RiverineGreenParams,
	RollingOaksParams, SpottyBushesParams, StorytellersParams, StrangeOasisParams,
	TemperateLowerMassivesParams, TemperateMassivesParams, TradeWindsParams, TropicalThicketParams,
	UnendingJungleParams, VineyardParams, WanderingAcaciaParams,
};
use chico_grove_render_items::palm_shade::PalmShadeStd;
use chico_grove_render_items::riparian_mix::RiparianMixStd;
use chico_grove_render_items::shamanhome::ShamanhomeStd;
use chico_grove_render_items::tall_grass::TallGrassStd;
use chico_grove_render_items::tropical_tufts::TropicalTuftsStd;
use chico_grove_render_items::tropical_undergrowth::TropicalUndergrowthStd;
use chico_grove_render_items::wild_grass::WildGrassStd;
use chico_sbs_trees::braid_oak_tree::BraidOakTreeParams;
use chico_sbs_trees::date_palm::DatePalmParams;
use chico_sbs_trees::friends_conifer::FriendsConifer;
use chico_sbs_trees::honu_banyan::HonuBanyanParams;
use chico_sbs_trees::jungle_storybook_tree::JungleStorybookTreeParams;
use chico_sbs_trees::kamakura_torch::KamakuraTorchParams;
use chico_sbs_trees::liams_conifer::LiamsConiferParams;
use chico_sbs_trees::northern_conifer::NorthernConiferParams;
use chico_sbs_trees::palm_bush::PalmBushParams;
use chico_sbs_trees::penmarch_torch::PenmarchTorchParams;
use chico_sbs_trees::rorys_head_trained::RorysHeadTrainedParams;
use chico_sbs_trees::sopes_banyan::SopesBanyanParams;
use chico_vegetation_components::{
	spawn_lod_scene_host, spawn_vegetation_components, vegetation_bounds, VegetationComponents,
};
use chico_sbs_trees::storybook_tree::StorybookTreeParams;
use chico_sbs_trees::temperate_conifer::TemperateConiferParams;
use chico_sbs_trees::tuft_patch::TuftPatchParams;
use chico_sbs_trees::vase_tree::VaseTreeParams;
use chico_sbs_trees::waialea_palm::WaialeaPalmParams;
use chico_sbs_trees::SkippedLeafMeshMaterial;
use chico_sbs_trees::SkippedStickMeshMaterial;
use chico_tree_components::{
	HighBushShoots, JungleGrowth, SkippedBodyMeshMaterial, SkippedFoliageMeshMaterial,
};
use chico_vegetation_shaders::{ChicoLeafMaterial, ChicoStickMaterial};
use chunk::cascade::CascadeChunk;
use render_item::RenderItem;

/// [`SopesBanyan`] configured for this playground (LodScene / VegetationComponents).
pub type RenderSopesBanyan = SopesBanyanParams;

/// [`HonuBanyan`] — wide spreading banyan ([#250](https://github.com/ramate-io/maybraid/issues/250)).
pub type RenderHonuBanyan = HonuBanyanParams;

/// [`LiamsConifer`] — VegetationComponents / LodScene.
pub type RenderLiamsConifer = LiamsConiferParams;

/// [`FriendsConifer`] — log-profile conifer with plane-splay foliage ([#236](https://github.com/ramate-io/maybraid/issues/236)).
pub type RenderFriendsConifer = FriendsConifer<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// [`NorthernConifer`] — Liam's geometry with plane-splay foliage ([#232](https://github.com/ramate-io/maybraid/issues/232)).
pub type RenderNorthernConifer = NorthernConiferParams;

/// [`TemperateConifer`] — Friend's log-profile conifer with joint fronds ([#238](https://github.com/ramate-io/maybraid/issues/238)).
pub type RenderTemperateConifer = TemperateConiferParams;

/// [`DatePalm`] — columnar trunk + stacked frond crown ([#256](https://github.com/ramate-io/maybraid/issues/256)).
pub type RenderDatePalm = DatePalmParams;

/// [`WaialeaPalm`] — arched trunk + light upward frond crown ([#255](https://github.com/ramate-io/maybraid/issues/255)).
pub type RenderWaialeaPalm = WaialeaPalmParams;

/// [`PalmBush`] — trunkless ground-anchored frond cluster ([#231](https://github.com/ramate-io/maybraid/issues/231)).
pub type RenderPalmBush = PalmBushParams;

/// [`TuftPatch`] — blade tufts scattered over a small ground area (VegetationComponents).
pub type RenderTuftPatch = TuftPatchParams;

/// [`StorybookTree`] — default broadleaf stalk + log-tapered radial canopy ([#230](https://github.com/ramate-io/maybraid/issues/230)).
pub type RenderStorybookTree = StorybookTreeParams;

/// [`PenmarchTorch`] — vase-profile upward flame tree ([#248](https://github.com/ramate-io/maybraid/issues/248)).
pub type RenderPenmarchTorch = PenmarchTorchParams;

/// [`KamakuraTorch`] — stashed near-vertical flame (linear crown bias).
pub type RenderKamakuraTorch = KamakuraTorchParams;

/// [`RorysHeadTrained`] — single high horizontal canopy ring ([#254](https://github.com/ramate-io/maybraid/issues/254)).
pub type RenderRorysHeadTrained = RorysHeadTrainedParams;

/// [`VaseTree`] — upward-opening vase-profile broadleaf ([#246](https://github.com/ramate-io/maybraid/issues/246)).
pub type RenderVaseTree = VaseTreeParams;

/// [`BraidOakTree`] — gnarled broadleaf with crook-cylinder branches ([#234](https://github.com/ramate-io/maybraid/issues/234)).
pub type RenderBraidOakTree = BraidOakTreeParams;

/// [`JungleStorybookTree`] — dense Storybook construction ([#235](https://github.com/ramate-io/maybraid/issues/235)).
pub type RenderJungleStorybookTree = JungleStorybookTreeParams;

pub type RenderSucculentTuft =
	SucculentTuft<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;
pub type RenderBladeTuft = BladeTuft<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;
pub type RenderSpearTuft = SpearTuft<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;
pub type RenderBuddhaHandTuft =
	BuddhaHandTuft<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;
pub type RenderWeepingTuft =
	WeepingTuft<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;
pub type RenderFrondCrown = FrondCrown<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;
pub type RenderModerateLodFrondCrown =
	ModerateLodFrondCrown<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;

/// [`HighBushShoots`] — trunkless radial shoots from a ground anchor ([#225](https://github.com/ramate-io/maybraid/issues/225)).
pub type RenderHighBushShoots = HighBushShoots<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
>;

/// [`JungleGrowth`] — bark/dirt inner mass + drooping tuft foliage ([#226](https://github.com/ramate-io/maybraid/issues/226)).
pub type RenderJungleGrowth = JungleGrowth<
	ChicoStickMaterial,
	SkippedBodyMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedFoliageMeshMaterial<StandardMaterial>,
>;

/// [`BraidGrassStd`] — understory blade-tuft grove ([#306](https://github.com/ramate-io/maybraid/issues/306)).
pub type RenderBraidGrass = BraidGrassStd;

/// [`TropicalTuftsStd`] — sparse tuft grove with palm companions ([#305](https://github.com/ramate-io/maybraid/issues/305)).
pub type RenderTropicalTufts = TropicalTuftsStd;

/// [`CommonTuftsStd`] — sparse low grass-clump grove ([#301](https://github.com/ramate-io/maybraid/issues/301)).
pub type RenderCommonTufts = CommonTuftsStd;

/// [`BushScrubStd`] — sparse tuft-and-bush grove ([#303](https://github.com/ramate-io/maybraid/issues/303)).
pub type RenderBushScrub = BushScrubStd;

/// [`TropicalUndergrowthStd`] — moderate-to-dense hybrid tropical understory ([#315](https://github.com/ramate-io/maybraid/issues/315)).
pub type RenderTropicalUndergrowth = TropicalUndergrowthStd;

/// [`TropicalThicketParams`] — dense tropical understory thicket ([#317](https://github.com/ramate-io/maybraid/issues/317)).
pub type RenderTropicalThicket = TropicalThicketParams;

/// [`JerrysChaparralStd`] — moderately dense dry scrub chaparral ([#318](https://github.com/ramate-io/maybraid/issues/318)).
pub type RenderJerrysChaparral = JerrysChaparralStd;

/// [`LevantineScrubParams`] — dry Mediterranean scrub understory ([#320](https://github.com/ramate-io/maybraid/issues/320)).
pub type RenderLevantineScrub = LevantineScrubParams;

/// [`TallGrassStd`] — dense mid-height tuft grove ([#302](https://github.com/ramate-io/maybraid/issues/302)).
pub type RenderTallGrass = TallGrassStd;

/// [`WildGrassStd`] — dense colorful tall-tuft grove ([#304](https://github.com/ramate-io/maybraid/issues/304)).
pub type RenderWildGrass = WildGrassStd;

/// [`MonsterGrassParams`] — oversized understory blade-wall grove ([#308](https://github.com/ramate-io/maybraid/issues/308)).
pub type RenderMonsterGrass = MonsterGrassParams;

/// [`RiverineGreenParams`] — sparse wet shrub understory grove ([#307](https://github.com/ramate-io/maybraid/issues/307)).
pub type RenderRiverineGreen = RiverineGreenParams;

/// [`LowBushParams`] — moderate low shrub understory grove ([#310](https://github.com/ramate-io/maybraid/issues/310)).
pub type RenderLowBush = LowBushParams;

/// [`HighBushParams`] — moderate tall shrub understory grove ([#312](https://github.com/ramate-io/maybraid/issues/312)).
pub type RenderHighBush = HighBushParams;

/// [`SpottyBushesParams`] — very sparse High Bush punctuation grove ([#321](https://github.com/ramate-io/maybraid/issues/321)).
pub type RenderSpottyBushes = SpottyBushesParams;

/// [`UnendingJungleParams`] — moderate lower-canopy jungle grove ([#322](https://github.com/ramate-io/maybraid/issues/322)).
pub type RenderUnendingJungle = UnendingJungleParams;

/// [`StrangeOasisParams`] — sparse oasis lower-canopy grove ([#323](https://github.com/ramate-io/maybraid/issues/323)).
pub type RenderStrangeOasis = StrangeOasisParams;

/// [`ShamanhomeStd`] — moderate sacred lower-canopy grove ([#324](https://github.com/ramate-io/maybraid/issues/324)).
pub type RenderShamanhome = ShamanhomeStd;

/// [`GoettingenFollowParams`] — low-density temperate follow-layer grove ([#325](https://github.com/ramate-io/maybraid/issues/325)).
pub type RenderGoettingenFollow = GoettingenFollowParams;

/// [`ConiferSaplingStd`] — moderate young conifer lower-canopy grove ([#326](https://github.com/ramate-io/maybraid/issues/326)).
pub type RenderConiferSapling = ConiferSaplingStd;

/// [`AridConiferSaplingStd`] — sparse dry young conifer lower-canopy grove ([#327](https://github.com/ramate-io/maybraid/issues/327)).
pub type RenderAridConiferSapling = AridConiferSaplingStd;

/// [`JungleLowerMassivesParams`] — moderate massive jungle lower-canopy grove ([#328](https://github.com/ramate-io/maybraid/issues/328)).
pub type RenderJungleLowerMassives = JungleLowerMassivesParams;

/// [`JungleMassivesParams`] — moderate giant jungle upper-canopy grove ([#331](https://github.com/ramate-io/maybraid/issues/331)).
pub type RenderJungleMassives = JungleMassivesParams;

/// [`TemperateLowerMassivesParams`] — low-density massive temperate lower-canopy grove ([#330](https://github.com/ramate-io/maybraid/issues/330)).
pub type RenderTemperateLowerMassives = TemperateLowerMassivesParams;

/// [`PalmShadeStd`] — sparse Waialea and Date Palm upper-canopy grove ([#332](https://github.com/ramate-io/maybraid/issues/332)).
pub type RenderPalmShade = PalmShadeStd;

/// [`RiparianMixStd`] — mixed riparian upper-canopy grove ([#333](https://github.com/ramate-io/maybraid/issues/333)).
pub type RenderRiparianMix = RiparianMixStd;

/// [`AlpineStd`] — cold upland conifer upper-canopy grove ([#334](https://github.com/ramate-io/maybraid/issues/334)).
pub type RenderAlpine = AlpineStd;

/// [`DrylandParams`] — very-low-density arid upper-canopy grove ([#335](https://github.com/ramate-io/maybraid/issues/335)).
pub type RenderDryland = DrylandParams;

/// [`StorytellersParams`] — colorful Storybook and Braid Oak upper-canopy grove ([#336](https://github.com/ramate-io/maybraid/issues/336)).
pub type RenderStorytellers = StorytellersParams;

/// [`TradeWindsParams`] — low-density tropical upper-canopy grove ([#337](https://github.com/ramate-io/maybraid/issues/337)).
pub type RenderTradeWinds = TradeWindsParams;

/// [`WanderingAcaciaParams`] — very-low-density dry open upper-canopy grove ([#338](https://github.com/ramate-io/maybraid/issues/338)).
pub type RenderWanderingAcacia = WanderingAcaciaParams;

/// [`LeewardParams`] — moderate-density sheltered upper-canopy grove ([#339](https://github.com/ramate-io/maybraid/issues/339)).
pub type RenderLeeward = LeewardParams;

/// [`ChristmasTaigaStd`] — moderate-density cold Northern Conifer upper-canopy grove ([#341](https://github.com/ramate-io/maybraid/issues/341)).
pub type RenderChristmasTaiga = ChristmasTaigaStd;

/// [`ConiferMassivesStd`] — moderate giant conifer upper-canopy grove.
pub type RenderConiferMassives = ConiferMassivesStd;

/// [`TemperateMassivesParams`] — moderate giant temperate upper-canopy grove.
pub type RenderTemperateMassives = TemperateMassivesParams;

/// [`RiparianGeneralParams`] — mixed riparian upper-canopy grove.
pub type RenderRiparianGeneral = RiparianGeneralParams;

/// [`RollingOaksParams`] — rolling oak upper-canopy grove.
pub type RenderRollingOaks = RollingOaksParams;

/// [`ForlornSavannaParams`] — sparse dry savanna upper-canopy grove.
pub type RenderForlornSavanna = ForlornSavannaParams;

/// [`OrchardParams`] — cultivated orchard upper-canopy grove.
pub type RenderOrchard = OrchardParams;

/// [`VineyardParams`] — cultivated vineyard upper-canopy grove.
pub type RenderVineyard = VineyardParams;

/// [`DateGroveStd`] — date palm upper-canopy grove.
pub type RenderDateGrove = DateGroveStd;

/// The configured render item currently shown in the scene.
///
/// This is the typed scene state behind [`RenderConfig`]: material patching
/// ([`crate::render_materials`]) and respawning ([`sync_render`]) both need concrete item types,
/// which the CLI layer (`crate::commands::render::Render`) discards once it resolves its
/// transform/resolution arguments into a [`RenderConfig`].
#[derive(Clone)]
pub enum RenderSubject {
	SopesBanyan(RenderSopesBanyan),
	HonuBanyan(RenderHonuBanyan),
	LiamsConifer(RenderLiamsConifer),
	FriendsConifer(RenderFriendsConifer),
	NorthernConifer(RenderNorthernConifer),
	TemperateConifer(RenderTemperateConifer),
	DatePalm(RenderDatePalm),
	WaialeaPalm(RenderWaialeaPalm),
	PalmBush(RenderPalmBush),
	StorybookTree(RenderStorybookTree),
	PenmarchTorch(RenderPenmarchTorch),
	KamakuraTorch(RenderKamakuraTorch),
	RorysHeadTrained(RenderRorysHeadTrained),
	VaseTree(RenderVaseTree),
	BraidOakTree(RenderBraidOakTree),
	JungleStorybookTree(RenderJungleStorybookTree),
	SucculentTuft(RenderSucculentTuft),
	BladeTuft(RenderBladeTuft),
	TuftPatch(RenderTuftPatch),
	BraidGrass(RenderBraidGrass),
	TropicalTufts(RenderTropicalTufts),
	CommonTufts(RenderCommonTufts),
	BushScrub(RenderBushScrub),
	TropicalUndergrowth(RenderTropicalUndergrowth),
	TropicalThicket(RenderTropicalThicket),
	JerrysChaparral(RenderJerrysChaparral),
	LevantineScrub(RenderLevantineScrub),
	TallGrass(RenderTallGrass),
	WildGrass(RenderWildGrass),
	MonsterGrass(RenderMonsterGrass),
	RiverineGreen(RenderRiverineGreen),
	LowBush(RenderLowBush),
	HighBush(RenderHighBush),
	SpottyBushes(RenderSpottyBushes),
	UnendingJungle(RenderUnendingJungle),
	StrangeOasis(RenderStrangeOasis),
	Shamanhome(RenderShamanhome),
	GoettingenFollow(RenderGoettingenFollow),
	ConiferSapling(RenderConiferSapling),
	AridConiferSapling(RenderAridConiferSapling),
	JungleLowerMassives(RenderJungleLowerMassives),
	JungleMassives(RenderJungleMassives),
	TemperateLowerMassives(RenderTemperateLowerMassives),
	PalmShade(RenderPalmShade),
	RiparianMix(RenderRiparianMix),
	Alpine(RenderAlpine),
	Dryland(RenderDryland),
	Storytellers(RenderStorytellers),
	TradeWinds(RenderTradeWinds),
	WanderingAcacia(RenderWanderingAcacia),
	Leeward(RenderLeeward),
	ChristmasTaiga(RenderChristmasTaiga),
	ConiferMassives(RenderConiferMassives),
	TemperateMassives(RenderTemperateMassives),
	RiparianGeneral(RenderRiparianGeneral),
	RollingOaks(RenderRollingOaks),
	ForlornSavanna(RenderForlornSavanna),
	Orchard(RenderOrchard),
	Vineyard(RenderVineyard),
	DateGrove(RenderDateGrove),
	SpearTuft(RenderSpearTuft),
	BuddhaHandTuft(RenderBuddhaHandTuft),
	WeepingTuft(RenderWeepingTuft),
	HighBushShoots(RenderHighBushShoots),
	JungleGrowth(RenderJungleGrowth),
	FrondCrown(RenderFrondCrown),
	ModerateLodFrondCrown(RenderModerateLodFrondCrown),
}

impl RenderSubject {
	pub fn label(&self) -> &'static str {
		match self {
			Self::SopesBanyan(_) => "SopesBanyan",
			Self::HonuBanyan(_) => "HonuBanyan",
			Self::LiamsConifer(_) => "LiamsConifer",
			Self::FriendsConifer(_) => "FriendsConifer",
			Self::NorthernConifer(_) => "NorthernConifer",
			Self::TemperateConifer(_) => "TemperateConifer",
			Self::DatePalm(_) => "DatePalm",
			Self::WaialeaPalm(_) => "WaialeaPalm",
			Self::PalmBush(_) => "PalmBush",
			Self::StorybookTree(_) => "StorybookTree",
			Self::PenmarchTorch(_) => "PenmarchTorch",
			Self::KamakuraTorch(_) => "KamakuraTorch",
			Self::RorysHeadTrained(_) => "RorysHeadTrained",
			Self::VaseTree(_) => "VaseTree",
			Self::BraidOakTree(_) => "BraidOakTree",
			Self::JungleStorybookTree(_) => "JungleStorybookTree",
			Self::SucculentTuft(_) => "SucculentTuft",
			Self::BladeTuft(_) => "BladeTuft",
			Self::TuftPatch(_) => "TuftPatch",
			Self::BraidGrass(_) => "BraidGrass",
			Self::TropicalTufts(_) => "TropicalTufts",
			Self::CommonTufts(_) => "CommonTufts",
			Self::BushScrub(_) => "BushScrub",
			Self::TropicalUndergrowth(_) => "TropicalUndergrowth",
			Self::TropicalThicket(_) => "TropicalThicket",
			Self::JerrysChaparral(_) => "JerrysChaparral",
			Self::LevantineScrub(_) => "LevantineScrub",
			Self::TallGrass(_) => "TallGrass",
			Self::WildGrass(_) => "WildGrass",
			Self::MonsterGrass(_) => "MonsterGrass",
			Self::RiverineGreen(_) => "RiverineGreen",
			Self::LowBush(_) => "LowBush",
			Self::HighBush(_) => "HighBush",
			Self::SpottyBushes(_) => "SpottyBushes",
			Self::UnendingJungle(_) => "UnendingJungle",
			Self::StrangeOasis(_) => "StrangeOasis",
			Self::Shamanhome(_) => "Shamanhome",
			Self::GoettingenFollow(_) => "GoettingenFollow",
			Self::ConiferSapling(_) => "ConiferSapling",
			Self::AridConiferSapling(_) => "AridConiferSapling",
			Self::JungleLowerMassives(_) => "JungleLowerMassives",
			Self::JungleMassives(_) => "JungleMassives",
			Self::TemperateLowerMassives(_) => "TemperateLowerMassives",
			Self::PalmShade(_) => "PalmShade",
			Self::RiparianMix(_) => "RiparianMix",
			Self::Alpine(_) => "Alpine",
			Self::Dryland(_) => "Dryland",
			Self::Storytellers(_) => "Storytellers",
			Self::TradeWinds(_) => "TradeWinds",
			Self::WanderingAcacia(_) => "WanderingAcacia",
			Self::Leeward(_) => "Leeward",
			Self::ChristmasTaiga(_) => "ChristmasTaiga",
			Self::ConiferMassives(_) => "ConiferMassives",
			Self::TemperateMassives(_) => "TemperateMassives",
			Self::RiparianGeneral(_) => "RiparianGeneral",
			Self::RollingOaks(_) => "RollingOaks",
			Self::ForlornSavanna(_) => "ForlornSavanna",
			Self::Orchard(_) => "Orchard",
			Self::Vineyard(_) => "Vineyard",
			Self::DateGrove(_) => "DateGrove",
			Self::SpearTuft(_) => "SpearTuft",
			Self::BuddhaHandTuft(_) => "BuddhaHandTuft",
			Self::WeepingTuft(_) => "WeepingTuft",
			Self::HighBushShoots(_) => "HighBushShoots",
			Self::JungleGrowth(_) => "JungleGrowth",
			Self::FrondCrown(_) => "FrondCrown",
			Self::ModerateLodFrondCrown(_) => "ModerateLodFrondCrown",
		}
	}

	/// CLI / shape parameters that should trigger a mesh rebuild.
	pub fn sync_param_key(&self) -> String {
		match self {
			Self::SopesBanyan(t) => format!("{:?}", t.geometry),
			Self::HonuBanyan(t) => format!("{:?}", t.geometry),
			Self::LiamsConifer(t) => format!("{:?}", t.geometry),
			Self::FriendsConifer(t) => {
				format!("{:?}|splay={}", t.geometry, t.splay_radius_fraction_of_height)
			}
			Self::NorthernConifer(t) => {
				format!(
					"{:?}|splay={}|spawn={}|apex={}",
					t.geometry,
					t.splay_radius_fraction_of_height,
					t.splay_spawn_fraction,
					t.apex_canopy_spawn_fraction
				)
			}
			Self::TemperateConifer(t) => {
				format!(
					"{:?}|fronds={:?}|len={:?}|spawn={}",
					t.geometry.inner,
					t.fronds_per_joint,
					t.frond_length_fraction,
					t.frond_spawn_fraction
				)
			}
			Self::DatePalm(t) => format!("{:?}", t.geometry),
			Self::WaialeaPalm(t) => format!("{:?}", t.geometry),
			Self::PalmBush(t) => format!("{:?}", t.geometry),
			Self::StorybookTree(t) => format!("{:?}", t.geometry),
			Self::PenmarchTorch(t) => format!("{:?}", t.geometry),
			Self::KamakuraTorch(t) => format!("{:?}", t.geometry),
			Self::RorysHeadTrained(t) => format!("{:?}", t.geometry),
			Self::VaseTree(t) => format!("{:?}", t.geometry),
			Self::BraidOakTree(t) => format!("{:?}", t.geometry),
			Self::JungleStorybookTree(t) => format!("{:?}", t.geometry),
			Self::SucculentTuft(t) => format!("{:?}", t.shape),
			Self::BladeTuft(t) => format!("{:?}", t.shape),
			Self::TuftPatch(t) => {
				format!(
					"{:?}|clumps={}|patch_extent_xz={}",
					t.shape, t.clump_count, t.patch_extent_xz
				)
			}
			Self::BraidGrass(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|foliage={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.foliage_noise
				)
			}
			Self::TropicalTufts(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|foliage={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.foliage_noise
				)
			}
			Self::CommonTufts(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|foliage={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.foliage_noise
				)
			}
			Self::BushScrub(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.bush_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::TropicalUndergrowth(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::TropicalThicket(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.bush_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::JerrysChaparral(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::LevantineScrub(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::TallGrass(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|foliage={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.foliage_noise
				)
			}
			Self::WildGrass(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|foliage={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.foliage_noise
				)
			}
			Self::MonsterGrass(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|foliage={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.foliage_noise
				)
			}
			Self::RiverineGreen(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.bush_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::LowBush(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.bush_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::HighBush(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.bush_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::SpottyBushes(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.bush_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::UnendingJungle(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::JungleLowerMassives(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::JungleMassives(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::TemperateLowerMassives(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::PalmShade(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::RiparianMix(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::Alpine(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::Dryland(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::Storytellers(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::TradeWinds(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::WanderingAcacia(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.bush_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::Leeward(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::ChristmasTaiga(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::ConiferMassives(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::TemperateMassives(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::RiparianGeneral(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::RollingOaks(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::ForlornSavanna(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::Orchard(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::Vineyard(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::DateGrove(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::StrangeOasis(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.leaf_surface_noise
				)
			}
			Self::Shamanhome(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::GoettingenFollow(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::ConiferSapling(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::AridConiferSapling(g) => {
				format!(
					"{:?}|extent={:?}|cell_extent_xz={:?}|terrain={:?}|chain={:?}|stick={:?}|leaf={:?}",
					g.grove,
					g.extent,
					g.cell_extent_xz(),
					g.terrain,
					g.tree_chain_noise,
					g.stick_surface_noise,
					g.leaf_surface_noise
				)
			}
			Self::SpearTuft(t) => format!("{:?}", t.shape),
			Self::BuddhaHandTuft(t) => format!("{:?}", t.shape),
			Self::WeepingTuft(t) => format!("{:?}", t.shape),
			Self::HighBushShoots(t) => format!("{:?}", t.shape),
			Self::JungleGrowth(t) => format!("{:?}", t.shape),
			Self::FrondCrown(t) => format!("{:?}", t.shape),
			Self::ModerateLodFrondCrown(t) => format!("{:?}", t.shape),
		}
	}

	/// Spawns this subject's render items, returning the top-level entities.
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		chunk: &CascadeChunk,
		transform: Transform,
	) -> Vec<Entity> {
		match self {
			Self::SopesBanyan(item) => {
				let tree = item.build();
				let bounds = vegetation_bounds(&tree);
				spawn_vegetation_components(commands, &tree, transform, bounds)
			}
			Self::HonuBanyan(item) => {
				let tree = item.build();
				let bounds = vegetation_bounds(&tree);
				spawn_vegetation_components(commands, &tree, transform, bounds)
			}
			Self::LiamsConifer(item) => {
				let tree = item.build();
				let bounds = vegetation_bounds(&tree);
				spawn_vegetation_components(commands, &tree, transform, bounds)
			}
			Self::FriendsConifer(item) => item.spawn_render_items(commands, chunk, transform),
			Self::NorthernConifer(item) => {
				let tree = item.build();
				let bounds = vegetation_bounds(&tree);
				spawn_vegetation_components(commands, &tree, transform, bounds)
			}
			Self::TemperateConifer(item) => {
				let tree = item.build();
				let bounds = vegetation_bounds(&tree);
				spawn_vegetation_components(commands, &tree, transform, bounds)
			}
			Self::DatePalm(item) => {
				let tree = item.build();
				let bounds = vegetation_bounds(&tree);
				spawn_vegetation_components(commands, &tree, transform, bounds)
			}
			Self::WaialeaPalm(item) => {
				let tree = item.build();
				let bounds = vegetation_bounds(&tree);
				spawn_vegetation_components(commands, &tree, transform, bounds)
			}
			Self::PalmBush(item) => {
				let bush = item.build();
				let bounds = vegetation_bounds(&bush);
				spawn_vegetation_components(commands, &bush, transform, bounds)
			}
			Self::StorybookTree(item) => {
				let tree = item.build();
				let bounds = vegetation_bounds(&tree);
				spawn_vegetation_components(commands, &tree, transform, bounds)
			}
			Self::PenmarchTorch(item) => {
				let tree = item.build();
				let bounds = vegetation_bounds(&tree);
				spawn_vegetation_components(commands, &tree, transform, bounds)
			}
			Self::KamakuraTorch(item) => {
				let tree = item.build();
				let bounds = vegetation_bounds(&tree);
				spawn_vegetation_components(commands, &tree, transform, bounds)
			}
			Self::RorysHeadTrained(item) => {
				let tree = item.build();
				let bounds = vegetation_bounds(&tree);
				spawn_vegetation_components(commands, &tree, transform, bounds)
			}
			Self::VaseTree(item) => {
				let tree = item.build();
				let bounds = vegetation_bounds(&tree);
				spawn_vegetation_components(commands, &tree, transform, bounds)
			}
			Self::BraidOakTree(item) => {
				let tree = item.build();
				let bounds = vegetation_bounds(&tree);
				spawn_vegetation_components(commands, &tree, transform, bounds)
			}
			Self::JungleStorybookTree(item) => {
				let tree = item.build();
				let bounds = vegetation_bounds(&tree);
				spawn_vegetation_components(commands, &tree, transform, bounds)
			}
			Self::SucculentTuft(item) => item.spawn_render_items(commands, chunk, transform),
			Self::BladeTuft(item) => item.spawn_render_items(commands, chunk, transform),
			Self::TuftPatch(item) => {
				let patch = item.build();
				let bounds = vegetation_bounds(&patch);
				spawn_vegetation_components(commands, &patch, transform, bounds)
			}
			Self::BraidGrass(item) => item.spawn_render_items(commands, chunk, transform),
			Self::TropicalTufts(item) => item.spawn_render_items(commands, chunk, transform),
			Self::CommonTufts(item) => item.spawn_render_items(commands, chunk, transform),
			Self::BushScrub(item) => item.spawn_render_items(commands, chunk, transform),
			Self::TropicalUndergrowth(item) => item.spawn_render_items(commands, chunk, transform),
			Self::TropicalThicket(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::JerrysChaparral(item) => item.spawn_render_items(commands, chunk, transform),
			Self::LevantineScrub(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::TallGrass(item) => item.spawn_render_items(commands, chunk, transform),
			Self::WildGrass(item) => item.spawn_render_items(commands, chunk, transform),
			Self::MonsterGrass(item) => {
				let grove = item.build();
				let bounds = vegetation_bounds(&grove);
				spawn_vegetation_components(commands, &grove, transform, bounds)
			}
			Self::RiverineGreen(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			},
			Self::LowBush(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::HighBush(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::SpottyBushes(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::UnendingJungle(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::StrangeOasis(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::Shamanhome(item) => item.spawn_render_items(commands, chunk, transform),
			Self::GoettingenFollow(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::ConiferSapling(item) => item.spawn_render_items(commands, chunk, transform),
			Self::AridConiferSapling(item) => item.spawn_render_items(commands, chunk, transform),
			Self::JungleLowerMassives(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::JungleMassives(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::TemperateLowerMassives(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::PalmShade(item) => item.spawn_render_items(commands, chunk, transform),
			Self::RiparianMix(item) => item.spawn_render_items(commands, chunk, transform),
			Self::Alpine(item) => item.spawn_render_items(commands, chunk, transform),
			Self::Dryland(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::Storytellers(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			},
			Self::TradeWinds(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			},
			Self::WanderingAcacia(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			},
			Self::Leeward(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::ChristmasTaiga(item) => item.spawn_render_items(commands, chunk, transform),
			Self::ConiferMassives(item) => item.spawn_render_items(commands, chunk, transform),
			Self::TemperateMassives(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			},
			Self::RiparianGeneral(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::RollingOaks(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::ForlornSavanna(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::Orchard(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::Vineyard(item) => {
				let grove = item.build();
				let bounds = grove
					.structural_lod()
					.map(|p| p.footprint_aabb())
					.unwrap_or_else(|| vegetation_bounds(&grove));
				spawn_lod_scene_host(commands, &grove, transform, bounds)
			}
			Self::DateGrove(item) => item.spawn_render_items(commands, chunk, transform),
			Self::SpearTuft(item) => item.spawn_render_items(commands, chunk, transform),
			Self::BuddhaHandTuft(item) => item.spawn_render_items(commands, chunk, transform),
			Self::WeepingTuft(item) => item.spawn_render_items(commands, chunk, transform),
			Self::HighBushShoots(item) => item.spawn_render_items(commands, chunk, transform),
			Self::JungleGrowth(item) => item.spawn_render_items(commands, chunk, transform),
			Self::FrondCrown(item) => item.spawn_render_items(commands, chunk, transform),
			Self::ModerateLodFrondCrown(item) => {
				item.spawn_render_items(commands, chunk, transform)
			}
		}
	}
}

/// Top-level entity spawned by the render pipeline ([`RenderItem::spawn_render_items`] return).
#[derive(Component)]
pub struct SbsRenderItem;

#[derive(Resource, Clone)]
pub struct RenderConfig {
	pub subject: RenderSubject,
	pub res_2: u8,
	pub transform: Transform,
}

impl Default for RenderConfig {
	fn default() -> Self {
		Self {
			subject: RenderSubject::LiamsConifer(RenderLiamsConifer::default()),
			res_2: 4,
			transform: Transform::default(),
		}
	}
}

fn render_sync_key(config: &RenderConfig) -> String {
	format!(
		"{}|params={}|res_2={}|t={:?}|s={:?}|r={:?}",
		config.subject.label(),
		config.subject.sync_param_key(),
		config.res_2,
		config.transform.translation,
		config.transform.scale,
		config.transform.rotation,
	)
}

/// Despawns the previous scene and respawns the subject when [`RenderConfig`] changes.
pub fn sync_render(
	mut commands: Commands,
	config: Res<RenderConfig>,
	show_config: Res<crate::commands::show::ShowConfig>,
	mut synced: Local<Option<String>>,
	item_q: Query<Entity, (With<SbsRenderItem>, Without<ChildOf>)>,
	show_roots: Query<Entity, With<crate::commands::show::ShowRoot>>,
) {
	// `/show` owns the scene while a subject is set.
	if show_config.subject.is_some() {
		for entity in &item_q {
			commands.entity(entity).despawn();
		}
		*synced = None;
		return;
	}

	let key = render_sync_key(&config);
	if synced.as_deref() == Some(&key) {
		return;
	}

	for entity in &item_q {
		commands.entity(entity).despawn();
	}
	for entity in &show_roots {
		commands.entity(entity).despawn();
	}

	let chunk = CascadeChunk::unit_center_chunk().with_res_2(config.res_2);
	for entity in config.subject.spawn_render_items(&mut commands, &chunk, config.transform) {
		commands.entity(entity).insert(SbsRenderItem);
	}

	*synced = Some(key);
}
