use bevy::prelude::*;
use chico_ball_components::{FrondCrown, ModerateLodFrondCrown};
use chico_ball_components::tuft::{
	BuddhaHandTuft, BladeTuft, SpearTuft, SucculentTuft, WeepingTuft,
};
use chico_sbs_trees::date_palm::DatePalm;
use chico_sbs_trees::waialea_palm::WaialeaPalm;
use chico_sbs_trees::liams_conifer::LiamsConifer;
use chico_sbs_trees::temperate_conifer::TemperateConifer;
use chico_sbs_trees::jungle_storybook_tree::JungleStorybookTree;
use chico_sbs_trees::braid_oak_tree::BraidOakTree;
use chico_sbs_trees::storybook_tree::StorybookTree;
use chico_sbs_trees::sopes_banyan::SopesBanyan;
use chico_sbs_trees::SkippedLeafMeshMaterial;
use chico_sbs_trees::SkippedStickMeshMaterial;
use chico_tree_components::{
	JungleGrowth, SkippedBodyMeshMaterial, SkippedFoliageMeshMaterial,
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

/// [`LiamsConifer`] configured for this playground (green [`StandardMaterial`] tufts for shape debugging).
pub type RenderLiamsConifer = LiamsConifer<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedLeafMeshMaterial<StandardMaterial>,
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

/// [`StorybookTree`] — default broadleaf stalk + log-tapered radial canopy ([#230](https://github.com/ramate-io/maybraid/issues/230)).
pub type RenderStorybookTree = StorybookTree<
	ChicoStickMaterial,
	SkippedStickMeshMaterial<ChicoStickMaterial>,
	ChicoLeafMaterial,
	SkippedLeafMeshMaterial<ChicoLeafMaterial>,
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

use chico_sbs_trees::{
	SkippedInnerLeafMeshMaterial, SkippedOuterLeafMeshMaterial,
};

pub type RenderSucculentTuft =
	SucculentTuft<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;
pub type RenderBladeTuft = BladeTuft<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;
pub type RenderSpearTuft = SpearTuft<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;
pub type RenderBuddhaHandTuft =
	BuddhaHandTuft<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;
pub type RenderWeepingTuft =
	WeepingTuft<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;
pub type RenderFrondCrown =
	FrondCrown<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;
pub type RenderModerateLodFrondCrown =
	ModerateLodFrondCrown<StandardMaterial, SkippedLeafMeshMaterial<StandardMaterial>>;

/// [`JungleGrowth`] — bark/dirt inner mass + drooping tuft foliage ([#226](https://github.com/ramate-io/maybraid/issues/226)).
pub type RenderJungleGrowth = JungleGrowth<
	ChicoStickMaterial,
	SkippedBodyMeshMaterial<ChicoStickMaterial>,
	StandardMaterial,
	SkippedFoliageMeshMaterial<StandardMaterial>,
>;

#[derive(Clone)]
pub enum RenderSubject {
	SopesBanyan(RenderSopesBanyan),
	LiamsConifer(RenderLiamsConifer),
	TemperateConifer(RenderTemperateConifer),
	DatePalm(RenderDatePalm),
	WaialeaPalm(RenderWaialeaPalm),
	StorybookTree(RenderStorybookTree),
	BraidOakTree(RenderBraidOakTree),
	JungleStorybookTree(RenderJungleStorybookTree),
	SucculentTuft(RenderSucculentTuft),
	BladeTuft(RenderBladeTuft),
	SpearTuft(RenderSpearTuft),
	BuddhaHandTuft(RenderBuddhaHandTuft),
	WeepingTuft(RenderWeepingTuft),
	JungleGrowth(RenderJungleGrowth),
	FrondCrown(RenderFrondCrown),
	ModerateLodFrondCrown(RenderModerateLodFrondCrown),
}

impl RenderSubject {
	pub fn label(&self) -> &'static str {
		match self {
			Self::SopesBanyan(_) => "SopesBanyan",
			Self::LiamsConifer(_) => "LiamsConifer",
			Self::TemperateConifer(_) => "TemperateConifer",
			Self::DatePalm(_) => "DatePalm",
			Self::WaialeaPalm(_) => "WaialeaPalm",
			Self::StorybookTree(_) => "StorybookTree",
			Self::BraidOakTree(_) => "BraidOakTree",
			Self::JungleStorybookTree(_) => "JungleStorybookTree",
			Self::SucculentTuft(_) => "SucculentTuft",
			Self::BladeTuft(_) => "BladeTuft",
			Self::SpearTuft(_) => "SpearTuft",
			Self::BuddhaHandTuft(_) => "BuddhaHandTuft",
			Self::WeepingTuft(_) => "WeepingTuft",
			Self::JungleGrowth(_) => "JungleGrowth",
			Self::FrondCrown(_) => "FrondCrown",
			Self::ModerateLodFrondCrown(_) => "ModerateLodFrondCrown",
		}
	}

	/// CLI / shape parameters that should trigger a mesh rebuild.
	pub fn sync_param_key(&self) -> String {
		match self {
			Self::SopesBanyan(t) => format!("{:?}", t.geometry),
			Self::LiamsConifer(t) => format!("{:?}", t.geometry),
			Self::TemperateConifer(t) => {
				format!(
					"{:?}|fronds={:?}|len={:?}|spawn={}",
					t.geometry, t.fronds_per_joint, t.frond_length_fraction, t.frond_spawn_fraction
				)
			}
			Self::DatePalm(t) => format!("{:?}", t.geometry),
			Self::WaialeaPalm(t) => format!("{:?}", t.geometry),
			Self::StorybookTree(t) => format!("{:?}", t.geometry),
			Self::BraidOakTree(t) => format!("{:?}", t.geometry),
			Self::JungleStorybookTree(t) => format!("{:?}", t.geometry),
			Self::SucculentTuft(t) => format!("{:?}", t.shape),
			Self::BladeTuft(t) => format!("{:?}", t.shape),
			Self::SpearTuft(t) => format!("{:?}", t.shape),
			Self::BuddhaHandTuft(t) => format!("{:?}", t.shape),
			Self::WeepingTuft(t) => format!("{:?}", t.shape),
			Self::JungleGrowth(t) => format!("{:?}", t.shape),
			Self::FrondCrown(t) => format!("{:?}", t.shape),
			Self::ModerateLodFrondCrown(t) => format!("{:?}", t.shape),
		}
	}

	fn dispatch_item(&self) -> RenderDispatch {
		match self {
			Self::SopesBanyan(tree) => RenderDispatch::SopesBanyan(tree.clone()),
			Self::LiamsConifer(tree) => RenderDispatch::LiamsConifer(tree.clone()),
			Self::TemperateConifer(tree) => RenderDispatch::TemperateConifer(tree.clone()),
			Self::DatePalm(tree) => RenderDispatch::DatePalm(tree.clone()),
			Self::WaialeaPalm(tree) => RenderDispatch::WaialeaPalm(tree.clone()),
			Self::StorybookTree(tree) => RenderDispatch::StorybookTree(tree.clone()),
			Self::BraidOakTree(tree) => RenderDispatch::BraidOakTree(tree.clone()),
			Self::JungleStorybookTree(tree) => RenderDispatch::JungleStorybookTree(tree.clone()),
			Self::SucculentTuft(tuft) => RenderDispatch::SucculentTuft(tuft.clone()),
			Self::BladeTuft(tuft) => RenderDispatch::BladeTuft(tuft.clone()),
			Self::SpearTuft(tuft) => RenderDispatch::SpearTuft(tuft.clone()),
			Self::BuddhaHandTuft(tuft) => RenderDispatch::BuddhaHandTuft(tuft.clone()),
			Self::WeepingTuft(tuft) => RenderDispatch::WeepingTuft(tuft.clone()),
			Self::JungleGrowth(growth) => RenderDispatch::JungleGrowth(growth.clone()),
			Self::FrondCrown(crown) => RenderDispatch::FrondCrown(crown.clone()),
			Self::ModerateLodFrondCrown(crown) => RenderDispatch::ModerateLodFrondCrown(crown.clone()),
		}
	}
}

#[derive(Clone)]
enum RenderDispatch {
	SopesBanyan(RenderSopesBanyan),
	LiamsConifer(RenderLiamsConifer),
	TemperateConifer(RenderTemperateConifer),
	DatePalm(RenderDatePalm),
	WaialeaPalm(RenderWaialeaPalm),
	StorybookTree(RenderStorybookTree),
	BraidOakTree(RenderBraidOakTree),
	JungleStorybookTree(RenderJungleStorybookTree),
	SucculentTuft(RenderSucculentTuft),
	BladeTuft(RenderBladeTuft),
	SpearTuft(RenderSpearTuft),
	BuddhaHandTuft(RenderBuddhaHandTuft),
	WeepingTuft(RenderWeepingTuft),
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
		RenderDispatch::LiamsConifer(tree) => {
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
		RenderDispatch::StorybookTree(tree) => {
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
		RenderDispatch::SpearTuft(tuft) => {
			commands.spawn((bundle, DispatchRenderItem::new(tuft)));
		}
		RenderDispatch::BuddhaHandTuft(tuft) => {
			commands.spawn((bundle, DispatchRenderItem::new(tuft)));
		}
		RenderDispatch::WeepingTuft(tuft) => {
			commands.spawn((bundle, DispatchRenderItem::new(tuft)));
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
