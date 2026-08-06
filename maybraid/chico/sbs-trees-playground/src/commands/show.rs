//! `/show` — LodScene presentation (VegetationComponents).

use bevy::prelude::*;
use chico_sbs_trees::{
	BraidOakTreeParams, DatePalmParams, HonuBanyanParams, JungleStorybookTreeParams,
	KamakuraTorchParams, LiamsConiferParams, NorthernConiferParams, PalmBushParams,
	PalmCrownParams, PenmarchTorchParams, RorysHeadTrainedParams, SimplemansHedgeParams,
	SopesBanyanParams, StorybookTreeParams, TemperateConiferParams, TuftPatchParams,
	VaseTreeParams, WaialeaPalmParams,
};
use chico_vegetation_components::{
	spawn_vegetation_components, vegetation_bounds, VegetationComponents,
};
use clap::{Args, Subcommand};

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
	/// Palm Crown via VegetationComponents / LodScene (fronds; layered ball at Low).
	PalmCrown(ShowPalmCrown),
	/// Date Palm via VegetationComponents / LodScene.
	DatePalm(ShowDatePalm),
	/// Waialea Palm via VegetationComponents / LodScene.
	WaialeaPalm(ShowWaialeaPalm),
	/// Palm Bush via VegetationComponents / LodScene.
	PalmBush(ShowPalmBush),
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
}

#[derive(Component)]
pub struct ShowRoot;

fn spawn_show_tree(
	commands: &mut Commands,
	tree: &impl VegetationComponents,
) {
	let bounds = vegetation_bounds(tree);
	let entities = spawn_vegetation_components(commands, tree, Transform::IDENTITY, bounds);
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
		Some(ShowSubject::NorthernConifer(t)) => {
			Some(format!(
				"northern-conifer:{:?}|splay={}|spawn={}|apex={}",
				t.geometry,
				t.splay_radius_fraction_of_height,
				t.splay_spawn_fraction,
				t.apex_canopy_spawn_fraction
			))
		}
		Some(ShowSubject::LiamsConifer(t)) => Some(format!("liams-conifer:{:?}", t.geometry)),
		Some(ShowSubject::TemperateConifer(t)) => Some(format!(
			"temperate-conifer:{:?}|fronds={:?}|len={:?}|spawn={}",
			t.geometry.inner,
			t.fronds_per_joint,
			t.frond_length_fraction,
			t.frond_spawn_fraction
		)),
		Some(ShowSubject::HonuBanyan(t)) => Some(format!(
			"honu-banyan:{:?}|growth={}",
			t.geometry, t.growth_spawn_fraction
		)),
		Some(ShowSubject::JungleStorybookTree(t)) => Some(format!(
			"jungle-storybook-tree:{:?}|growth={}",
			t.geometry, t.growth_spawn_fraction
		)),
		Some(ShowSubject::BraidOakTree(t)) => Some(format!(
			"braid-oak-tree:{:?}|stick={:?}",
			t.geometry, t.stick_surface_noise
		)),
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
	}
}
