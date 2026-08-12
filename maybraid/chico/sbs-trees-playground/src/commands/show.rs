//! `/show` — LodScene presentation (VegetationComponents).

use bevy::prelude::*;
use chico_groves::{
	ForlornSavannaParams, GroveExtent, LevantineScrubParams, MonsterGrassParams, OrchardParams,
	RiparianGeneralParams, RollingOaksParams, StrangeOasisParams, TropicalThicketParams,
	DEFAULT_GROVE_EXTENT_XZ,
};
use crate::monster_grass_plain::spawn_monster_grass_plain;
use chico_sbs_trees::{
	BraidOakTreeParams, DatePalmParams, HighBushShootsParams, HonuBanyanParams,
	JungleStorybookTreeParams, KamakuraTorchParams, LiamsConiferParams, NorthernConiferParams,
	PalmBushParams, PalmCrownParams, PenmarchTorchParams, RorysHeadTrainedParams,
	SimplemansHedgeParams, SopesBanyanParams, StorybookTreeParams, TemperateConiferParams,
	TuftPatchParams, VaseTreeParams, WaialeaPalmParams,
};
use chico_vegetation_components::{
	spawn_lod_scene_host, spawn_vegetation_components, vegetation_bounds, VegetationComponents,
};
use lod::gen::LodScene;
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
	/// Monster Grass grove via VegetationComponents / LodScene.
	MonsterGrass(ShowMonsterGrass),
	/// Centered radius-10 tile of default Monster Grass groves (21×21).
	MonsterGrassPlains,
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
	/// Riparian General grove via VegetationComponents / LodScene.
	RiparianGeneral(ShowRiparianGeneral),
	/// Forlorn Savanna grove via VegetationComponents / LodScene.
	ForlornSavanna(ShowForlornSavanna),
	/// High Bush Shoots via VegetationComponents / LodScene.
	HighBushShoots(ShowHighBushShoots),
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
pub struct ShowHighBushShoots {
	#[command(flatten)]
	pub bush: HighBushShootsParams,
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
			Self::LevantineScrub(args) => ShowSubject::LevantineScrub(args.configured()),
			Self::StrangeOasis(args) => ShowSubject::StrangeOasis(args.configured()),
			Self::TropicalThicket(args) => ShowSubject::TropicalThicket(args.configured()),
			Self::RollingOaks(args) => ShowSubject::RollingOaks(args.configured()),
			Self::Orchard(args) => ShowSubject::Orchard(args.configured()),
			Self::RiparianGeneral(args) => ShowSubject::RiparianGeneral(args.configured()),
			Self::ForlornSavanna(args) => ShowSubject::ForlornSavanna(args.configured()),
			Self::HighBushShoots(args) => ShowSubject::HighBushShoots(args.bush),
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
	LevantineScrub(LevantineScrubParams),
	StrangeOasis(StrangeOasisParams),
	TropicalThicket(TropicalThicketParams),
	RollingOaks(RollingOaksParams),
	Orchard(OrchardParams),
	RiparianGeneral(RiparianGeneralParams),
	ForlornSavanna(ForlornSavannaParams),
	HighBushShoots(HighBushShootsParams),
}

#[derive(Component)]
pub struct ShowRoot;

fn spawn_show_tree<T>(commands: &mut Commands, tree: &T)
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
{
	let bounds = vegetation_bounds(tree);
	let entities = spawn_vegetation_components(commands, tree, Transform::IDENTITY, bounds);
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
		Some(ShowSubject::MonsterGrass(g)) => Some(format!(
			"monster-grass:extent={:?}|cell={:?}|terrain={:?}|foliage={:?}",
			g.extent,
			g.cell_extent_xz(),
			g.terrain,
			g.foliage_noise
		)),
		Some(ShowSubject::MonsterGrassPlains) => Some("monster-grass-plains".into()),
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
		Some(ShowSubject::HighBushShoots(b)) => Some(format!("high-bush-shoots:{:?}", b.shape)),
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
		ShowSubject::MonsterGrass(params) => spawn_show_tree(&mut commands, &params.build()),
		ShowSubject::MonsterGrassPlains => {
			for entity in spawn_monster_grass_plain(&mut commands, Transform::IDENTITY) {
				commands.entity(entity).insert(ShowRoot);
			}
		}
		ShowSubject::LevantineScrub(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::StrangeOasis(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::TropicalThicket(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::RollingOaks(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::Orchard(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::RiparianGeneral(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::ForlornSavanna(params) => spawn_show_grove(&mut commands, &params.build()),
		ShowSubject::HighBushShoots(params) => spawn_show_tree(&mut commands, &params.build()),
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
		assert!(matches!(
			cmd,
			crate::commands::PlaygroundCommand::Show(Show::MonsterGrassPlains)
		));
		Ok(())
	}

	#[test]
	fn show_refactored_groves_parse_and_build() -> Result<()> {
		for line in [
			"show levantine-scrub --grove-extent-xz 20",
			"show strange-oasis --grove-extent-xz 20",
			"show tropical-thicket --grove-extent-xz 20",
			"show rolling-oaks --grove-extent-xz 260 --elevation 0.40",
			"show orchard --grove-extent-xz 160",
			"show riparian-general --grove-extent-xz 200",
			"show forlorn-savanna --grove-extent-xz 300",
			"show high-bush-shoots",
		] {
			let cmd = crate::commands::PlaygroundCommand::parse_line(line)
				.map_err(|e| anyhow::anyhow!("{line}: {e}"))?;
			match cmd {
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
				crate::commands::PlaygroundCommand::Show(Show::HighBushShoots(args)) => {
					let _ = args.bush.build();
				}
				_ => anyhow::bail!("unexpected command for {line}"),
			}
		}
		Ok(())
	}
}
