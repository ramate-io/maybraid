use bevy::prelude::*;
use chico_ball_components::tuft::{
	BladeTuft, BuddhaHandTuft, SpearTuft, SucculentTuft, WeepingTuft,
};
use chico_ball_components::{FrondCrown, ModerateLodFrondCrown};
use chico_groves::braid_grass::BraidGrassStd;
use chico_sbs_trees::braid_oak_tree::BraidOakTree;
use chico_sbs_trees::date_palm::DatePalm;
use chico_sbs_trees::friends_conifer::FriendsConifer;
use chico_sbs_trees::honu_banyan::HonuBanyan;
use chico_sbs_trees::jungle_storybook_tree::JungleStorybookTree;
use chico_sbs_trees::kamakura_torch::KamakuraTorch;
use chico_sbs_trees::liams_conifer::LiamsConifer;
use chico_sbs_trees::northern_conifer::NorthernConifer;
use chico_sbs_trees::palm_bush::PalmBush;
use chico_sbs_trees::penmarch_torch::PenmarchTorch;
use chico_sbs_trees::rorys_head_trained::RorysHeadTrained;
use chico_sbs_trees::sopes_banyan::SopesBanyan;
use chico_sbs_trees::storybook_tree::StorybookTree;
use chico_sbs_trees::temperate_conifer::TemperateConifer;
use chico_sbs_trees::vase_tree::VaseTree;
use chico_sbs_trees::waialea_palm::WaialeaPalm;
use chico_sbs_trees::SkippedLeafMeshMaterial;
use chico_sbs_trees::SkippedStickMeshMaterial;
use chico_tree_components::{
	HighBushShoots, JungleGrowth, SkippedBodyMeshMaterial, SkippedFoliageMeshMaterial,
};
use chico_vegetation_shaders::{ChicoLeafMaterial, ChicoStickMaterial};
use chunk::cascade::CascadeChunk;
use render_item::{DispatchRenderItem, RenderItem};

/// [`SopesBanyan`] configured for this playground.
pub type RenderSopesBanyan = SopesBanyan<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// [`HonuBanyan`] — wide spreading banyan ([#250](https://github.com/ramate-io/maybraid/issues/250)).
pub type RenderHonuBanyan = HonuBanyan<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedInnerLeafMeshMaterial<ChicoLeafMaterial>,
	ChicoLeafMaterial,
	SkippedOuterLeafMeshMaterial<ChicoLeafMaterial>,
	ChicoStickMaterial,
	SkippedBodyMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedFoliageMeshMaterial<StandardMaterial>,
>;

/// [`LiamsConifer`] configured for this playground (green [`StandardMaterial`] tufts for shape debugging).
pub type RenderLiamsConifer = LiamsConifer<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
>;

/// [`FriendsConifer`] — log-profile conifer with plane-splay foliage ([#236](https://github.com/ramate-io/maybraid/issues/236)).
pub type RenderFriendsConifer = FriendsConifer<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// [`NorthernConifer`] — Liam's geometry with plane-splay foliage ([#232](https://github.com/ramate-io/maybraid/issues/232)).
pub type RenderNorthernConifer = NorthernConifer<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// [`TemperateConifer`] — Friend's log-profile conifer with joint fronds ([#238](https://github.com/ramate-io/maybraid/issues/238)).
pub type RenderTemperateConifer = TemperateConifer<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// [`DatePalm`] — columnar trunk + stacked frond crown ([#256](https://github.com/ramate-io/maybraid/issues/256)).
pub type RenderDatePalm = DatePalm<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// [`WaialeaPalm`] — arched trunk + light upward frond crown ([#255](https://github.com/ramate-io/maybraid/issues/255)).
pub type RenderWaialeaPalm = WaialeaPalm<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// [`PalmBush`] — trunkless ground-anchored frond cluster ([#231](https://github.com/ramate-io/maybraid/issues/231)).
pub type RenderPalmBush = PalmBush<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;

/// [`StorybookTree`] — default broadleaf stalk + log-tapered radial canopy ([#230](https://github.com/ramate-io/maybraid/issues/230)).
pub type RenderStorybookTree = StorybookTree<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// [`PenmarchTorch`] — vase-profile upward flame tree ([#248](https://github.com/ramate-io/maybraid/issues/248)).
pub type RenderPenmarchTorch = PenmarchTorch<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// [`KamakuraTorch`] — stashed near-vertical flame (linear crown bias).
pub type RenderKamakuraTorch = KamakuraTorch<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// [`RorysHeadTrained`] — single high horizontal canopy ring ([#254](https://github.com/ramate-io/maybraid/issues/254)).
pub type RenderRorysHeadTrained = RorysHeadTrained<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// [`VaseTree`] — upward-opening vase-profile broadleaf ([#246](https://github.com/ramate-io/maybraid/issues/246)).
pub type RenderVaseTree = VaseTree<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedInnerLeafMeshMaterial<ChicoLeafMaterial>,
	ChicoLeafMaterial,
	SkippedOuterLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// [`BraidOakTree`] — gnarled broadleaf with crook-cylinder branches ([#234](https://github.com/ramate-io/maybraid/issues/234)).
pub type RenderBraidOakTree = BraidOakTree<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedInnerLeafMeshMaterial<ChicoLeafMaterial>,
	ChicoLeafMaterial,
	SkippedOuterLeafMeshMaterial<ChicoLeafMaterial>,
>;

/// [`JungleStorybookTree`] — dense Storybook construction ([#235](https://github.com/ramate-io/maybraid/issues/235)).
pub type RenderJungleStorybookTree = JungleStorybookTree<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedInnerLeafMeshMaterial<ChicoLeafMaterial>,
	ChicoLeafMaterial,
	SkippedOuterLeafMeshMaterial<ChicoLeafMaterial>,
	ChicoStickMaterial,
	SkippedBodyMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedFoliageMeshMaterial<StandardMaterial>,
>;

use chico_sbs_trees::{SkippedInnerLeafMeshMaterial, SkippedOuterLeafMeshMaterial};

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
	BraidGrass(RenderBraidGrass),
	SpearTuft(RenderSpearTuft),
	BuddhaHandTuft(RenderBuddhaHandTuft),
	WeepingTuft(RenderWeepingTuft),
	HighBushShoots(RenderHighBushShoots),
	CommonHighBush(RenderHighBushShoots),
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
			Self::BraidGrass(_) => "BraidGrass",
			Self::SpearTuft(_) => "SpearTuft",
			Self::BuddhaHandTuft(_) => "BuddhaHandTuft",
			Self::WeepingTuft(_) => "WeepingTuft",
			Self::HighBushShoots(_) => "HighBushShoots",
			Self::CommonHighBush(_) => "CommonHighBush",
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
			Self::BraidGrass(g) => {
				format!(
					"{:?}|extent={:?}|terrain={:?}|foliage={:?}",
					g.grove, g.extent, g.terrain, g.foliage_noise
				)
			}
			Self::SpearTuft(t) => format!("{:?}", t.shape),
			Self::BuddhaHandTuft(t) => format!("{:?}", t.shape),
			Self::WeepingTuft(t) => format!("{:?}", t.shape),
			Self::HighBushShoots(t) | Self::CommonHighBush(t) => {
				format!("{:?}|style={:?}", t.shape, t.shape.foliage_style)
			}
			Self::JungleGrowth(t) => format!("{:?}", t.shape),
			Self::FrondCrown(t) => format!("{:?}", t.shape),
			Self::ModerateLodFrondCrown(t) => format!("{:?}", t.shape),
		}
	}

	fn dispatch_item(&self) -> RenderDispatch {
		match self {
			Self::SopesBanyan(tree) => RenderDispatch::SopesBanyan(tree.clone()),
			Self::HonuBanyan(tree) => RenderDispatch::HonuBanyan(tree.clone()),
			Self::LiamsConifer(tree) => RenderDispatch::LiamsConifer(tree.clone()),
			Self::FriendsConifer(tree) => RenderDispatch::FriendsConifer(tree.clone()),
			Self::NorthernConifer(tree) => RenderDispatch::NorthernConifer(tree.clone()),
			Self::TemperateConifer(tree) => RenderDispatch::TemperateConifer(tree.clone()),
			Self::DatePalm(tree) => RenderDispatch::DatePalm(tree.clone()),
			Self::WaialeaPalm(tree) => RenderDispatch::WaialeaPalm(tree.clone()),
			Self::PalmBush(tree) => RenderDispatch::PalmBush(tree.clone()),
			Self::StorybookTree(tree) => RenderDispatch::StorybookTree(tree.clone()),
			Self::PenmarchTorch(tree) => RenderDispatch::PenmarchTorch(tree.clone()),
			Self::KamakuraTorch(tree) => RenderDispatch::KamakuraTorch(tree.clone()),
			Self::RorysHeadTrained(tree) => RenderDispatch::RorysHeadTrained(tree.clone()),
			Self::VaseTree(tree) => RenderDispatch::VaseTree(tree.clone()),
			Self::BraidOakTree(tree) => RenderDispatch::BraidOakTree(tree.clone()),
			Self::JungleStorybookTree(tree) => RenderDispatch::JungleStorybookTree(tree.clone()),
			Self::SucculentTuft(tuft) => RenderDispatch::SucculentTuft(tuft.clone()),
			Self::BladeTuft(tuft) => RenderDispatch::BladeTuft(tuft.clone()),
			Self::BraidGrass(grove) => RenderDispatch::BraidGrass(grove.clone()),
			Self::SpearTuft(tuft) => RenderDispatch::SpearTuft(tuft.clone()),
			Self::BuddhaHandTuft(tuft) => RenderDispatch::BuddhaHandTuft(tuft.clone()),
			Self::WeepingTuft(tuft) => RenderDispatch::WeepingTuft(tuft.clone()),
			Self::HighBushShoots(shoots) => RenderDispatch::HighBushShoots(shoots.clone()),
			Self::CommonHighBush(shoots) => RenderDispatch::CommonHighBush(shoots.clone()),
			Self::JungleGrowth(growth) => RenderDispatch::JungleGrowth(growth.clone()),
			Self::FrondCrown(crown) => RenderDispatch::FrondCrown(crown.clone()),
			Self::ModerateLodFrondCrown(crown) => {
				RenderDispatch::ModerateLodFrondCrown(crown.clone())
			}
		}
	}
}

#[derive(Clone)]
enum RenderDispatch {
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
	BraidGrass(RenderBraidGrass),
	SpearTuft(RenderSpearTuft),
	BuddhaHandTuft(RenderBuddhaHandTuft),
	WeepingTuft(RenderWeepingTuft),
	HighBushShoots(RenderHighBushShoots),
	CommonHighBush(RenderHighBushShoots),
	JungleGrowth(RenderJungleGrowth),
	FrondCrown(RenderFrondCrown),
	ModerateLodFrondCrown(RenderModerateLodFrondCrown),
}

/// Dispatch anchor for the active `/render` command.
#[derive(Component)]
pub struct SbsRenderRoot;

/// Top-level entity spawned by the render pipeline (dispatch root or [`RenderItem::spawn_render_items`] return).
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

/// Despawns the previous scene and spawns a fresh dispatch entity when [`RenderConfig`] changes.
pub fn sync_render(
	mut commands: Commands,
	config: Res<RenderConfig>,
	mut synced: Local<Option<String>>,
	item_q: Query<Entity, (With<SbsRenderItem>, Without<ChildOf>)>,
) {
	let key = render_sync_key(&config);
	if synced.as_deref() == Some(&key) {
		return;
	}

	for entity in &item_q {
		commands.entity(entity).despawn();
	}

	let bundle = (
		SbsRenderRoot,
		SbsRenderItem,
		CascadeChunk::unit_center_chunk().with_res_2(config.res_2),
		config.transform,
	);

	match config.subject.dispatch_item() {
		RenderDispatch::SopesBanyan(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::HonuBanyan(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::LiamsConifer(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::FriendsConifer(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::NorthernConifer(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::TemperateConifer(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::DatePalm(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::WaialeaPalm(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::PalmBush(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::StorybookTree(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::PenmarchTorch(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::KamakuraTorch(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::RorysHeadTrained(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::VaseTree(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::BraidOakTree(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::JungleStorybookTree(tree) => {
			commands.spawn((bundle, DispatchRenderItem::new(tree)));
		}
		RenderDispatch::SucculentTuft(tuft) => {
			commands.spawn((bundle, DispatchRenderItem::new(tuft)));
		}
		RenderDispatch::BladeTuft(tuft) => {
			commands.spawn((bundle, DispatchRenderItem::new(tuft)));
		}
		RenderDispatch::BraidGrass(grove) => {
			commands.spawn((bundle, DispatchRenderItem::new(grove)));
		}
		RenderDispatch::SpearTuft(tuft) => {
			commands.spawn((bundle, DispatchRenderItem::new(tuft)));
		}
		RenderDispatch::BuddhaHandTuft(tuft) => {
			commands.spawn((bundle, DispatchRenderItem::new(tuft)));
		}
		RenderDispatch::WeepingTuft(tuft) => {
			commands.spawn((bundle, DispatchRenderItem::new(tuft)));
		}
		RenderDispatch::HighBushShoots(shoots) | RenderDispatch::CommonHighBush(shoots) => {
			commands.spawn((bundle, DispatchRenderItem::new(shoots)));
		}
		RenderDispatch::JungleGrowth(growth) => {
			commands.spawn((bundle, DispatchRenderItem::new(growth)));
		}
		RenderDispatch::FrondCrown(crown) => {
			commands.spawn((bundle, DispatchRenderItem::new(crown)));
		}
		RenderDispatch::ModerateLodFrondCrown(crown) => {
			commands.spawn((bundle, DispatchRenderItem::new(crown)));
		}
	}

	*synced = Some(key);
}

/// Runs [`RenderItem::spawn_render_items`] for new dispatch roots and tags top-level spawned entities.
pub fn dispatch_render_items<T: RenderItem + Send + Sync + 'static>(
	mut commands: Commands,
	query: Query<
		(Entity, &DispatchRenderItem<T>, &CascadeChunk, &Transform),
		(Added<DispatchRenderItem<T>>, With<SbsRenderRoot>, With<SbsRenderItem>),
	>,
) {
	for (_root, dispatch, chunk, transform) in &query {
		for entity in dispatch.spawn_render_items(&mut commands, chunk, *transform) {
			commands.entity(entity).insert(SbsRenderItem);
		}
	}
}
