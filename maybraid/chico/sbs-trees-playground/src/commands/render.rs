//! `/render` subcommand: one variant per previewable item.
//!
//! Trees in `chico-sbs-trees` are clap-parseable and flatten directly into [`RenderHelper`].
//! Lower-level components (tufts, frond crowns, high bush, jungle growth) deliberately stay
//! clap-free, so their helpers flatten the *shape* structs and [`Render::into_render_config`]
//! builds the render item with default (skipped) materials — `sync_render_material_handles`
//! patches in the curated handles before spawning.

use bevy::prelude::*;
use chico_ball_components::tuft::{
	BladeTuftShape, BuddhaHandTuftShape, SpearTuftShape, SucculentTuftShape, WeepingTuftShape,
};
use chico_ball_components::{FrondCrownShape, ModerateLodFrondCrownShape};
use chico_groves::{GroveExtent, DEFAULT_GROVE_EXTENT_XZ};
use chico_tree_components::{HighBushShootsShape, JungleGrowthShape};
use clap::Subcommand;
use procedural_common::{noise_params_from_scalar_str, NoiseParams};

use crate::render::{
	RenderBladeTuft, RenderBraidGrass, RenderBraidOakTree, RenderBuddhaHandTuft, RenderBushScrub,
	RenderCommonTufts, RenderConfig, RenderDatePalm, RenderFriendsConifer, RenderFrondCrown,
	RenderHighBush, RenderHighBushShoots, RenderHonuBanyan, RenderJerrysChaparral,
	RenderJungleGrowth, RenderJungleStorybookTree, RenderKamakuraTorch, RenderLevantineScrub,
	RenderLiamsConifer, RenderLowBush, RenderModerateLodFrondCrown, RenderMonsterGrass,
	RenderNorthernConifer, RenderPalmBush, RenderPenmarchTorch, RenderRiverineGreen,
	RenderRorysHeadTrained, RenderSopesBanyan, RenderSpearTuft, RenderSpottyBushes,
	RenderStorybookTree, RenderSubject, RenderSucculentTuft, RenderTallGrass,
	RenderTemperateConifer, RenderTropicalThicket, RenderTropicalTufts, RenderTropicalUndergrowth,
	RenderTuftPatch, RenderUnendingJungle, RenderStrangeOasis, RenderShamanhome,
	RenderGoettingenFollow, RenderConiferSapling, RenderAridConiferSapling,
	RenderJungleLowerMassives, RenderJungleMassives, RenderTemperateLowerMassives, RenderPalmShade,
	RenderRiparianMix, RenderAlpine, RenderDryland, RenderStorytellers, RenderTradeWinds,
	RenderWanderingAcacia, RenderLeeward, RenderChristmasTaiga,
	RenderVaseTree, RenderWaialeaPalm,
	RenderWeepingTuft, RenderWildGrass,
};

/// Shared render flags (resolution + scene transform) wrapped around per-item args.
#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct RenderHelper<T: clap::Args + Clone> {
	#[command(flatten)]
	pub inner: T,

	/// Render cascade resolution exponent.
	#[arg(long, default_value_t = 5)]
	#[arg(help_heading = "Render")]
	pub res_2: u8,

	/// Scale factors `x,y,z`.
	#[arg(long, default_value = "1,1,1", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z", help_heading = "Render Transform")]
	pub scale: Vec3,

	/// Translation `x,y,z` in world units.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z", help_heading = "Render Transform")]
	pub translate: Vec3,

	/// Euler rotation in degrees around X, then Y, then Z.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z", help_heading = "Render Transform")]
	pub rotate_euler: Vec3,
}

impl<T: clap::Args + Clone> RenderHelper<T> {
	pub fn render_transform(&self) -> Transform {
		let rot = Quat::from_euler(
			EulerRot::XYZ,
			self.rotate_euler.x.to_radians(),
			self.rotate_euler.y.to_radians(),
			self.rotate_euler.z.to_radians(),
		);
		Transform::from_translation(self.translate)
			.with_rotation(rot)
			.with_scale(self.scale)
	}

	fn config_with(&self, subject: RenderSubject) -> RenderConfig {
		RenderConfig { subject, res_2: self.res_2, transform: self.render_transform() }
	}
}

/// Wraps [`RenderHelper`] with square grove extent settings for grove [`RenderItem`] commands.
#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct CellRenderHelper<T: clap::Args + Clone> {
	#[command(flatten)]
	pub render: RenderHelper<T>,

	#[arg(long, default_value_t = DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl<T: clap::Args + Clone> CellRenderHelper<T> {
	pub fn render_transform(&self) -> Transform {
		self.render.render_transform()
	}

	pub fn res_2(&self) -> u8 {
		self.render.res_2
	}

	/// Square preview extent covering at least one grove cell.
	fn grove_extent(&self, cell_extent_xz: Vec2) -> GroveExtent {
		let span = self.grove_extent_xz.max(cell_extent_xz.x).max(cell_extent_xz.y);
		GroveExtent::new(Vec3::ZERO, Vec3::new(span, 1.0, span))
	}
}

impl CellRenderHelper<RenderBraidGrass> {
	pub fn configured_braid_grass(&self) -> RenderBraidGrass {
		let mut grass = self.render.inner.clone();
		grass.extent = self.grove_extent(grass.cell_extent_xz());
		grass
	}
}

impl CellRenderHelper<RenderTropicalTufts> {
	pub fn configured_tropical_tufts(&self) -> RenderTropicalTufts {
		let mut tufts = self.render.inner.clone();
		tufts.extent = self.grove_extent(tufts.cell_extent_xz());
		tufts
	}
}

impl CellRenderHelper<RenderCommonTufts> {
	pub fn configured_common_tufts(&self) -> RenderCommonTufts {
		let mut tufts = self.render.inner.clone();
		tufts.extent = self.grove_extent(tufts.cell_extent_xz());
		tufts
	}
}

impl CellRenderHelper<RenderBushScrub> {
	pub fn configured_bush_scrub(&self) -> RenderBushScrub {
		let mut scrub = self.render.inner.clone();
		scrub.extent = self.grove_extent(scrub.cell_extent_xz());
		scrub
	}
}

impl CellRenderHelper<RenderTropicalUndergrowth> {
	pub fn configured_tropical_undergrowth(&self) -> RenderTropicalUndergrowth {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderTropicalThicket> {
	pub fn configured_tropical_thicket(&self) -> RenderTropicalThicket {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderJerrysChaparral> {
	pub fn configured_jerrys_chaparral(&self) -> RenderJerrysChaparral {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderLevantineScrub> {
	pub fn configured_levantine_scrub(&self) -> RenderLevantineScrub {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderTallGrass> {
	pub fn configured_tall_grass(&self) -> RenderTallGrass {
		let mut grass = self.render.inner.clone();
		grass.extent = self.grove_extent(grass.cell_extent_xz());
		grass
	}
}

impl CellRenderHelper<RenderWildGrass> {
	pub fn configured_wild_grass(&self) -> RenderWildGrass {
		let mut grass = self.render.inner.clone();
		grass.extent = self.grove_extent(grass.cell_extent_xz());
		grass
	}
}

impl CellRenderHelper<RenderMonsterGrass> {
	pub fn configured_monster_grass(&self) -> RenderMonsterGrass {
		let mut grass = self.render.inner.clone();
		grass.extent = self.grove_extent(grass.cell_extent_xz());
		grass
	}
}

impl CellRenderHelper<RenderRiverineGreen> {
	pub fn configured_riverine_green(&self) -> RenderRiverineGreen {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderLowBush> {
	pub fn configured_low_bush(&self) -> RenderLowBush {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderHighBush> {
	pub fn configured_high_bush(&self) -> RenderHighBush {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderSpottyBushes> {
	pub fn configured_spotty_bushes(&self) -> RenderSpottyBushes {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderUnendingJungle> {
	pub fn configured_unending_jungle(&self) -> RenderUnendingJungle {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderStrangeOasis> {
	pub fn configured_strange_oasis(&self) -> RenderStrangeOasis {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderShamanhome> {
	pub fn configured_shamanhome(&self) -> RenderShamanhome {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderGoettingenFollow> {
	pub fn configured_goettingen_follow(&self) -> RenderGoettingenFollow {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderConiferSapling> {
	pub fn configured_conifer_sapling(&self) -> RenderConiferSapling {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderAridConiferSapling> {
	pub fn configured_arid_conifer_sapling(&self) -> RenderAridConiferSapling {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderJungleLowerMassives> {
	pub fn configured_jungle_lower_massives(&self) -> RenderJungleLowerMassives {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderJungleMassives> {
	pub fn configured_jungle_massives(&self) -> RenderJungleMassives {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderTemperateLowerMassives> {
	pub fn configured_temperate_lower_massives(&self) -> RenderTemperateLowerMassives {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderPalmShade> {
	pub fn configured_palm_shade(&self) -> RenderPalmShade {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderRiparianMix> {
	pub fn configured_riparian_mix(&self) -> RenderRiparianMix {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderAlpine> {
	pub fn configured_alpine(&self) -> RenderAlpine {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderDryland> {
	pub fn configured_dryland(&self) -> RenderDryland {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderStorytellers> {
	pub fn configured_storytellers(&self) -> RenderStorytellers {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderTradeWinds> {
	pub fn configured_trade_winds(&self) -> RenderTradeWinds {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderWanderingAcacia> {
	pub fn configured_wandering_acacia(&self) -> RenderWanderingAcacia {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderLeeward> {
	pub fn configured_leeward(&self) -> RenderLeeward {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

impl CellRenderHelper<RenderChristmasTaiga> {
	pub fn configured_christmas_taiga(&self) -> RenderChristmasTaiga {
		let mut grove = self.render.inner.clone();
		grove.extent = self.grove_extent(grove.cell_extent_xz());
		grove
	}
}

/// High bush shape plus the surface-noise flags that live on the render item (not the shape).
#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct HighBushShootsArgs {
	#[command(flatten)]
	pub shape: HighBushShootsShape,

	#[arg(
		long,
		default_value = "0,1,0.05,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Surface Noise"
	)]
	pub stick_surface_noise: NoiseParams,

	#[arg(
		long,
		default_value = "0,1,0.06,1",
		value_parser = noise_params_from_scalar_str,
		value_name = "SEED,FREQUENCY,AMPLITUDE,OCTAVES[,TYPE]",
		help_heading = "Surface Noise"
	)]
	pub leaf_surface_noise: NoiseParams,
}

impl HighBushShootsArgs {
	fn to_render(&self) -> RenderHighBushShoots {
		let mut shoots = RenderHighBushShoots::default();
		shoots.shape = self.shape.clone();
		shoots.stick_surface_noise = self.stick_surface_noise;
		shoots.leaf_surface_noise = self.leaf_surface_noise;
		shoots
	}
}

fn jungle_growth_from_shape(shape: JungleGrowthShape) -> RenderJungleGrowth {
	let mut growth = RenderJungleGrowth::default();
	growth.shape = shape;
	growth
}

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Render {
	SopesBanyan(RenderHelper<RenderSopesBanyan>),
	HonuBanyan(RenderHelper<RenderHonuBanyan>),
	LiamsConifer(RenderHelper<RenderLiamsConifer>),
	FriendsConifer(RenderHelper<RenderFriendsConifer>),
	NorthernConifer(RenderHelper<RenderNorthernConifer>),
	TemperateConifer(RenderHelper<RenderTemperateConifer>),
	DatePalm(RenderHelper<RenderDatePalm>),
	WaialeaPalm(RenderHelper<RenderWaialeaPalm>),
	PalmBush(RenderHelper<RenderPalmBush>),
	StorybookTree(RenderHelper<RenderStorybookTree>),
	PenmarchTorch(RenderHelper<RenderPenmarchTorch>),
	KamakuraTorch(RenderHelper<RenderKamakuraTorch>),
	RorysHeadTrained(RenderHelper<RenderRorysHeadTrained>),
	VaseTree(RenderHelper<RenderVaseTree>),
	BraidOakTree(RenderHelper<RenderBraidOakTree>),
	JungleStorybookTree(RenderHelper<RenderJungleStorybookTree>),
	SucculentTuft(RenderHelper<SucculentTuftShape>),
	BladeTuft(RenderHelper<BladeTuftShape>),
	TuftPatch(RenderHelper<RenderTuftPatch>),
	BraidGrass(CellRenderHelper<RenderBraidGrass>),
	TropicalTufts(CellRenderHelper<RenderTropicalTufts>),
	CommonTufts(CellRenderHelper<RenderCommonTufts>),
	BushScrub(CellRenderHelper<RenderBushScrub>),
	TropicalUndergrowth(CellRenderHelper<RenderTropicalUndergrowth>),
	TropicalThicket(CellRenderHelper<RenderTropicalThicket>),
	JerrysChaparral(CellRenderHelper<RenderJerrysChaparral>),
	LevantineScrub(CellRenderHelper<RenderLevantineScrub>),
	TallGrass(CellRenderHelper<RenderTallGrass>),
	WildGrass(CellRenderHelper<RenderWildGrass>),
	MonsterGrass(CellRenderHelper<RenderMonsterGrass>),
	RiverineGreen(CellRenderHelper<RenderRiverineGreen>),
	LowBush(CellRenderHelper<RenderLowBush>),
	HighBush(CellRenderHelper<RenderHighBush>),
	SpottyBushes(CellRenderHelper<RenderSpottyBushes>),
	UnendingJungle(CellRenderHelper<RenderUnendingJungle>),
	StrangeOasis(CellRenderHelper<RenderStrangeOasis>),
	Shamanhome(CellRenderHelper<RenderShamanhome>),
	GoettingenFollow(CellRenderHelper<RenderGoettingenFollow>),
	ConiferSapling(CellRenderHelper<RenderConiferSapling>),
	AridConiferSapling(CellRenderHelper<RenderAridConiferSapling>),
	JungleLowerMassives(CellRenderHelper<RenderJungleLowerMassives>),
	JungleMassives(CellRenderHelper<RenderJungleMassives>),
	TemperateLowerMassives(CellRenderHelper<RenderTemperateLowerMassives>),
	PalmShade(CellRenderHelper<RenderPalmShade>),
	RiparianMix(CellRenderHelper<RenderRiparianMix>),
	Alpine(CellRenderHelper<RenderAlpine>),
	Dryland(CellRenderHelper<RenderDryland>),
	Storytellers(CellRenderHelper<RenderStorytellers>),
	TradeWinds(CellRenderHelper<RenderTradeWinds>),
	WanderingAcacia(CellRenderHelper<RenderWanderingAcacia>),
	Leeward(CellRenderHelper<RenderLeeward>),
	ChristmasTaiga(CellRenderHelper<RenderChristmasTaiga>),
	SpearTuft(RenderHelper<SpearTuftShape>),
	BuddhaHandTuft(RenderHelper<BuddhaHandTuftShape>),
	WeepingTuft(RenderHelper<WeepingTuftShape>),
	JungleGrowth(RenderHelper<JungleGrowthShape>),
	HighBushShoots(RenderHelper<HighBushShootsArgs>),
	/// Alias of `high-bush-shoots`; the Common High Bush preset is always applied at render
	/// ([#233](https://github.com/ramate-io/maybraid/issues/233)).
	CommonHighBush(RenderHelper<HighBushShootsArgs>),
	FrondCrown(RenderHelper<FrondCrownShape>),
	ModerateLodFrondCrown(RenderHelper<ModerateLodFrondCrownShape>),
}

impl Render {
	pub fn react(self, commands: &mut Commands) {
		let config = self.into_render_config();
		commands.queue(move |world: &mut World| {
			*world.resource_mut::<RenderConfig>() = config;
		});
	}

	pub fn into_render_config(&self) -> RenderConfig {
		match self {
			Self::SopesBanyan(h) => h.config_with(RenderSubject::SopesBanyan(h.inner.clone())),
			Self::HonuBanyan(h) => h.config_with(RenderSubject::HonuBanyan(h.inner.clone())),
			Self::LiamsConifer(h) => h.config_with(RenderSubject::LiamsConifer(h.inner.clone())),
			Self::FriendsConifer(h) => {
				h.config_with(RenderSubject::FriendsConifer(h.inner.clone()))
			}
			Self::NorthernConifer(h) => {
				h.config_with(RenderSubject::NorthernConifer(h.inner.clone()))
			}
			Self::TemperateConifer(h) => {
				h.config_with(RenderSubject::TemperateConifer(h.inner.clone()))
			}
			Self::DatePalm(h) => h.config_with(RenderSubject::DatePalm(h.inner.clone())),
			Self::WaialeaPalm(h) => h.config_with(RenderSubject::WaialeaPalm(h.inner.clone())),
			Self::PalmBush(h) => h.config_with(RenderSubject::PalmBush(h.inner.clone())),
			Self::StorybookTree(h) => h.config_with(RenderSubject::StorybookTree(h.inner.clone())),
			Self::PenmarchTorch(h) => h.config_with(RenderSubject::PenmarchTorch(h.inner.clone())),
			Self::KamakuraTorch(h) => h.config_with(RenderSubject::KamakuraTorch(h.inner.clone())),
			Self::RorysHeadTrained(h) => {
				h.config_with(RenderSubject::RorysHeadTrained(h.inner.clone()))
			}
			Self::VaseTree(h) => h.config_with(RenderSubject::VaseTree(h.inner.clone())),
			Self::BraidOakTree(h) => h.config_with(RenderSubject::BraidOakTree(h.inner.clone())),
			Self::JungleStorybookTree(h) => {
				h.config_with(RenderSubject::JungleStorybookTree(h.inner.clone()))
			}
			Self::SucculentTuft(h) => h.config_with(RenderSubject::SucculentTuft(
				RenderSucculentTuft::from_shape(h.inner.clone(), Default::default()),
			)),
			Self::BladeTuft(h) => h.config_with(RenderSubject::BladeTuft(
				RenderBladeTuft::from_shape(h.inner.clone(), Default::default()),
			)),
			Self::TuftPatch(h) => h.config_with(RenderSubject::TuftPatch(h.inner.clone())),
			Self::BraidGrass(h) => {
				h.render.config_with(RenderSubject::BraidGrass(h.configured_braid_grass()))
			}
			Self::TropicalTufts(h) => h
				.render
				.config_with(RenderSubject::TropicalTufts(h.configured_tropical_tufts())),
			Self::CommonTufts(h) => {
				h.render.config_with(RenderSubject::CommonTufts(h.configured_common_tufts()))
			}
			Self::BushScrub(h) => {
				h.render.config_with(RenderSubject::BushScrub(h.configured_bush_scrub()))
			}
			Self::TropicalUndergrowth(h) => h.render.config_with(
				RenderSubject::TropicalUndergrowth(h.configured_tropical_undergrowth()),
			),
			Self::TropicalThicket(h) => h
				.render
				.config_with(RenderSubject::TropicalThicket(h.configured_tropical_thicket())),
			Self::JerrysChaparral(h) => h
				.render
				.config_with(RenderSubject::JerrysChaparral(h.configured_jerrys_chaparral())),
			Self::LevantineScrub(h) => h
				.render
				.config_with(RenderSubject::LevantineScrub(h.configured_levantine_scrub())),
			Self::TallGrass(h) => {
				h.render.config_with(RenderSubject::TallGrass(h.configured_tall_grass()))
			}
			Self::WildGrass(h) => {
				h.render.config_with(RenderSubject::WildGrass(h.configured_wild_grass()))
			}
			Self::MonsterGrass(h) => {
				h.render.config_with(RenderSubject::MonsterGrass(h.configured_monster_grass()))
			}
			Self::RiverineGreen(h) => h
				.render
				.config_with(RenderSubject::RiverineGreen(h.configured_riverine_green())),
			Self::LowBush(h) => {
				h.render.config_with(RenderSubject::LowBush(h.configured_low_bush()))
			}
			Self::HighBush(h) => {
				h.render.config_with(RenderSubject::HighBush(h.configured_high_bush()))
			}
			Self::SpottyBushes(h) => {
				h.render.config_with(RenderSubject::SpottyBushes(h.configured_spotty_bushes()))
			}
			Self::UnendingJungle(h) => h
				.render
				.config_with(RenderSubject::UnendingJungle(h.configured_unending_jungle())),
			Self::StrangeOasis(h) => h
				.render
				.config_with(RenderSubject::StrangeOasis(h.configured_strange_oasis())),
			Self::Shamanhome(h) => h
				.render
				.config_with(RenderSubject::Shamanhome(h.configured_shamanhome())),
			Self::GoettingenFollow(h) => h.render.config_with(RenderSubject::GoettingenFollow(
				h.configured_goettingen_follow(),
			)),
			Self::ConiferSapling(h) => h.render.config_with(RenderSubject::ConiferSapling(
				h.configured_conifer_sapling(),
			)),
			Self::AridConiferSapling(h) => h.render.config_with(RenderSubject::AridConiferSapling(
				h.configured_arid_conifer_sapling(),
			)),
			Self::JungleLowerMassives(h) => h.render.config_with(
				RenderSubject::JungleLowerMassives(h.configured_jungle_lower_massives()),
			),
			Self::JungleMassives(h) => h.render.config_with(
				RenderSubject::JungleMassives(h.configured_jungle_massives()),
			),
			Self::TemperateLowerMassives(h) => h.render.config_with(
				RenderSubject::TemperateLowerMassives(h.configured_temperate_lower_massives()),
			),
			Self::PalmShade(h) => h.render.config_with(
				RenderSubject::PalmShade(h.configured_palm_shade()),
			),
			Self::RiparianMix(h) => h.render.config_with(
				RenderSubject::RiparianMix(h.configured_riparian_mix()),
			),
			Self::Alpine(h) => h.render.config_with(
				RenderSubject::Alpine(h.configured_alpine()),
			),
			Self::Dryland(h) => h.render.config_with(
				RenderSubject::Dryland(h.configured_dryland()),
			),
			Self::Storytellers(h) => h.render.config_with(
				RenderSubject::Storytellers(h.configured_storytellers()),
			),
			Self::TradeWinds(h) => h.render.config_with(
				RenderSubject::TradeWinds(h.configured_trade_winds()),
			),
			Self::WanderingAcacia(h) => h.render.config_with(
				RenderSubject::WanderingAcacia(h.configured_wandering_acacia()),
			),
			Self::Leeward(h) => h.render.config_with(RenderSubject::Leeward(h.configured_leeward())),
			Self::ChristmasTaiga(h) => h.render.config_with(
				RenderSubject::ChristmasTaiga(h.configured_christmas_taiga()),
			),
			Self::SpearTuft(h) => h.config_with(RenderSubject::SpearTuft(
				RenderSpearTuft::from_shape(h.inner.clone(), Default::default()),
			)),
			Self::BuddhaHandTuft(h) => h.config_with(RenderSubject::BuddhaHandTuft(
				RenderBuddhaHandTuft::from_shape(h.inner.clone(), Default::default()),
			)),
			Self::WeepingTuft(h) => h.config_with(RenderSubject::WeepingTuft(
				RenderWeepingTuft::from_shape(h.inner.clone(), Default::default()),
			)),
			Self::JungleGrowth(h) => h.config_with(RenderSubject::JungleGrowth(
				jungle_growth_from_shape(h.inner.clone()),
			)),
			Self::HighBushShoots(h) | Self::CommonHighBush(h) => {
				h.config_with(RenderSubject::HighBushShoots(h.inner.to_render()))
			}
			Self::FrondCrown(h) => h.config_with(RenderSubject::FrondCrown(
				RenderFrondCrown::from_shape(h.inner.clone(), Default::default()),
			)),
			Self::ModerateLodFrondCrown(h) => h.config_with(RenderSubject::ModerateLodFrondCrown(
				RenderModerateLodFrondCrown::from_shape(h.inner.clone(), Default::default()),
			)),
		}
	}
}

fn parse_vec3_csv(s: &str) -> Result<Vec3, String> {
	let parts: Vec<&str> = s.split(',').map(str::trim).collect();
	if parts.len() != 3 {
		return Err(format!("expected x,y,z, got {s:?}"));
	}
	let x = parts[0].parse::<f32>().map_err(|e| e.to_string())?;
	let y = parts[1].parse::<f32>().map_err(|e| e.to_string())?;
	let z = parts[2].parse::<f32>().map_err(|e| e.to_string())?;
	Ok(Vec3::new(x, y, z))
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use chico_sbs_geometry::FriendsConiferSbs;

	#[test]
	fn spear_tuft_command_preserves_shape_params() -> Result<()> {
		let cmd =
			crate::commands::PlaygroundCommand::parse_line("render spear-tuft --spear-count 20")
				.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::SpearTuft(helper)) = cmd else {
			anyhow::bail!("expected spear-tuft render command");
		};
		assert_eq!(helper.inner.spear_count, 20);
		let cfg = Render::SpearTuft(helper).into_render_config();
		let RenderSubject::SpearTuft(tuft) = cfg.subject else {
			anyhow::bail!("expected spear subject");
		};
		assert_eq!(tuft.shape.spear_count, 20);
		Ok(())
	}

	#[test]
	fn jungle_growth_command_preserves_shape_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render jungle-growth --inner-ball-scale 0.9 --seed 42",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::JungleGrowth(helper)) = cmd else {
			anyhow::bail!("expected jungle-growth render command");
		};
		assert!((helper.inner.inner_ball_scale - 0.9).abs() < 1e-5);
		assert_eq!(helper.inner.seed, 42);
		let cfg = Render::JungleGrowth(helper).into_render_config();
		let RenderSubject::JungleGrowth(growth) = cfg.subject else {
			anyhow::bail!("expected jungle growth subject");
		};
		assert!((growth.shape.inner_ball_scale - 0.9).abs() < 1e-5);
		assert_eq!(growth.shape.seed, 42);
		Ok(())
	}

	#[test]
	fn palm_bush_command_preserves_shape_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render palm-bush --ring-count 9 --fronds-per-ring 14",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::PalmBush(helper)) = cmd else {
			anyhow::bail!("expected palm-bush render command");
		};
		assert_eq!(helper.inner.geometry.crown.ring_count, 9);
		assert_eq!(helper.inner.geometry.crown.fronds_per_ring, 14);
		let cfg = Render::PalmBush(helper).into_render_config();
		let RenderSubject::PalmBush(bush) = cfg.subject else {
			anyhow::bail!("expected palm bush subject");
		};
		assert_eq!(bush.geometry.crown.ring_count, 9);
		assert_eq!(bush.geometry.crown.fronds_per_ring, 14);
		Ok(())
	}

	#[test]
	fn date_palm_command_preserves_shape_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render date-palm --ring-count 8")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::DatePalm(helper)) = cmd else {
			anyhow::bail!("expected date-palm render command");
		};
		assert_eq!(helper.inner.geometry.crown.ring_count, 8);
		let cfg = Render::DatePalm(helper).into_render_config();
		let RenderSubject::DatePalm(palm) = cfg.subject else {
			anyhow::bail!("expected date palm subject");
		};
		assert_eq!(palm.geometry.crown.ring_count, 8);
		Ok(())
	}

	#[test]
	fn waialea_palm_command_preserves_shape_params() -> Result<()> {
		let cmd =
			crate::commands::PlaygroundCommand::parse_line("render waialea-palm --ring-count 3")
				.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::WaialeaPalm(helper)) = cmd else {
			anyhow::bail!("expected waialea-palm render command");
		};
		assert_eq!(helper.inner.geometry.crown.ring_count, 3);
		let cfg = Render::WaialeaPalm(helper).into_render_config();
		let RenderSubject::WaialeaPalm(palm) = cfg.subject else {
			anyhow::bail!("expected waialea palm subject");
		};
		assert_eq!(palm.geometry.crown.ring_count, 3);
		Ok(())
	}

	#[test]
	fn waialea_palm_command_preserves_arch_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render waialea-palm --arch-lateral-fraction 0.18 --arch-yaw-degrees 45",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::WaialeaPalm(helper)) = cmd else {
			anyhow::bail!("expected waialea-palm render command");
		};
		assert!((helper.inner.geometry.trunk.arch_lateral_fraction - 0.18).abs() < 1e-5);
		assert!((helper.inner.geometry.trunk.arch_yaw_degrees - 45.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn storybook_tree_command_preserves_shape_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render storybook-tree --tree-height 18 --branch-depth 4 --ring-heights 0.30..1.0",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::StorybookTree(helper)) = cmd else {
			anyhow::bail!("expected storybook-tree render command");
		};
		assert!((helper.inner.geometry.scale.tree_height - 18.0).abs() < 1e-5);
		assert_eq!(helper.inner.geometry.growth.branch_depth, 4);
		let cfg = Render::StorybookTree(helper).into_render_config();
		let RenderSubject::StorybookTree(tree) = cfg.subject else {
			anyhow::bail!("expected storybook tree subject");
		};
		assert!((tree.geometry.scale.tree_height - 18.0).abs() < 1e-5);
		assert_eq!(tree.geometry.growth.branch_depth, 4);
		Ok(())
	}

	#[test]
	fn braid_oak_tree_command_preserves_shape_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render braid-oak-tree --tree-height 18 --branch-depth 4 --ring-heights 0.20..1.0",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::BraidOakTree(helper)) = cmd else {
			anyhow::bail!("expected braid-oak-tree render command");
		};
		assert!((helper.inner.geometry.storybook.scale.tree_height - 18.0).abs() < 1e-5);
		assert_eq!(helper.inner.geometry.storybook.growth.branch_depth, 4);
		let mut geometry = helper.inner.geometry.clone();
		geometry.apply_braid_preset();
		assert!(
			(geometry.storybook.scale.stalk_height_fraction
				- chico_sbs_geometry::anchors::braid_oak::BRAID_STALK_HEIGHT_FRACTION)
				.abs() < 1e-5
		);
		let cfg = Render::BraidOakTree(helper).into_render_config();
		let RenderSubject::BraidOakTree(tree) = cfg.subject else {
			anyhow::bail!("expected braid oak tree subject");
		};
		assert!((tree.geometry.storybook.scale.tree_height - 18.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn jungle_storybook_tree_command_preserves_shape_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render jungle-storybook-tree --tree-height 18 --branch-depth 4",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::JungleStorybookTree(helper)) = cmd
		else {
			anyhow::bail!("expected jungle-storybook-tree render command");
		};
		assert!((helper.inner.geometry.storybook.scale.tree_height - 18.0).abs() < 1e-5);
		assert_eq!(helper.inner.geometry.storybook.growth.branch_depth, 4);
		let mut geometry = helper.inner.geometry.clone();
		geometry.apply_jungle_preset();
		assert!((geometry.storybook.growth.branch_base_radius_fraction_of_stalk
			- chico_sbs_geometry::sbs::jungle_storybook_tree::JUNGLE_BRANCH_BASE_RADIUS_FRACTION_OF_STALK)
			.abs()
			< 1e-5);
		Ok(())
	}

	#[test]
	fn kamakura_torch_command_preserves_shape_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render kamakura-torch --tree-height 20 --torch-bias-low-degrees 50 --torch-bias-high-degrees 72",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::KamakuraTorch(helper)) = cmd else {
			anyhow::bail!("expected kamakura-torch render command");
		};
		assert!((helper.inner.geometry.scale.tree_height - 20.0).abs() < 1e-5);
		assert!((helper.inner.geometry.growth.torch_bias_low_degrees - 50.0).abs() < 1e-5);
		assert!((helper.inner.geometry.growth.torch_bias_high_degrees - 72.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn rorys_head_trained_command_preserves_shape_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render rorys-head-trained --tree-height 20 --projection 0.35..0.50 --canopy-ring-unit-height 1.0",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::RorysHeadTrained(helper)) = cmd
		else {
			anyhow::bail!("expected rorys-head-trained render command");
		};
		assert!((helper.inner.geometry.scale.tree_height - 20.0).abs() < 1e-5);
		assert!(
			(helper.inner.geometry.projection.span_fraction_of_height.start - 0.35).abs() < 1e-5
		);
		assert!((helper.inner.geometry.rings.canopy_ring_unit_height - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn vase_tree_command_preserves_shape_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render vase-tree --tree-height 20 --branch-depth 4 --bias-elevation-lo-degrees 40",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::VaseTree(helper)) = cmd else {
			anyhow::bail!("expected vase-tree render command");
		};
		assert!((helper.inner.geometry.scale.tree_height - 20.0).abs() < 1e-5);
		assert!((helper.inner.geometry.growth.bias_elevation_lo_degrees - 40.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn penmarch_torch_command_preserves_shape_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render penmarch-torch --tree-height 24 --projection 0.10..0.45",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::PenmarchTorch(helper)) = cmd else {
			anyhow::bail!("expected penmarch-torch render command");
		};
		assert!((helper.inner.geometry.scale.tree_height - 24.0).abs() < 1e-5);
		assert!(
			(helper.inner.geometry.projection.span_fraction_of_height.start - 0.10).abs() < 1e-5
		);
		Ok(())
	}

	#[test]
	fn northern_conifer_command_preserves_shape_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render northern-conifer --stalk-height 28 --ring-heights 0.12..0.95 --splay-radius-fraction-of-height 0.02",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::NorthernConifer(helper)) = cmd
		else {
			anyhow::bail!("expected northern-conifer render command");
		};
		assert!((helper.inner.geometry.scale.stalk_height - 28.0).abs() < 1e-5);
		assert!((helper.inner.geometry.rings.height_range.start - 0.12).abs() < 1e-5);
		assert!((helper.inner.geometry.rings.height_range.end - 0.95).abs() < 1e-5);
		assert!((helper.inner.splay_radius_fraction_of_height - 0.02).abs() < 1e-5);
		let mut geometry = helper.inner.geometry.clone();
		geometry.apply_northern_preset();
		assert!(
			(geometry.liams.projection.length_fraction_of_height.start
				- chico_sbs_geometry::sbs::northern_conifer::NORTHERN_MAX_PROJECTION_FRACTION_OF_HEIGHT)
				.abs()
				< 1e-5
		);
		Ok(())
	}

	#[test]
	fn high_bush_shoots_command_preserves_shape_params() -> Result<()> {
		use chico_tree_components::HighBushFoliageStyle;

		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render high-bush-shoots --height 12 --shoot-count 8 --foliage-style tuft",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::HighBushShoots(helper)) = cmd else {
			anyhow::bail!("expected high-bush-shoots render command");
		};
		assert!((helper.inner.shape.height - 12.0).abs() < 1e-5);
		assert_eq!(helper.inner.shape.shoot_count, 8);
		assert_eq!(helper.inner.shape.foliage_style, HighBushFoliageStyle::Tuft);
		let mut shape = helper.inner.shape.clone();
		chico_tree_components::apply_common_high_bush_preset(&mut shape);
		assert_eq!(shape.shoot_count, 8);
		Ok(())
	}

	#[test]
	fn common_high_bush_command_matches_high_bush_shoots() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render common-high-bush --height 12 --shoot-count 8",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(render) = cmd else {
			anyhow::bail!("expected render command");
		};
		let cfg = render.into_render_config();
		let RenderSubject::HighBushShoots(shoots) = cfg.subject else {
			anyhow::bail!("expected high bush shoots subject");
		};
		assert_eq!(shoots.shape.shoot_count, 8);
		Ok(())
	}

	#[test]
	fn friends_conifer_command_preserves_shape_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render friends-conifer --stalk-height 22 --angle-tolerance-degrees 32 --projection 0.12..0.03",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::FriendsConifer(helper)) = cmd else {
			anyhow::bail!("expected friends-conifer render command");
		};
		assert!((helper.inner.geometry.scale.stalk_height - 22.0).abs() < 1e-5);
		assert!((helper.inner.geometry.growth.angle_tolerance_degrees - 32.0).abs() < 1e-5);
		assert!(
			(helper.inner.geometry.projection.length_fraction_of_height.start - 0.12).abs() < 1e-5
		);
		Ok(())
	}

	#[test]
	fn temperate_conifer_command_preserves_shape_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render temperate-conifer --stalk-height 18 --fronds-per-joint 1..2 --frond-length-fraction 0.04..0.06 --frond-spawn-fraction 0.7",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::TemperateConifer(helper)) = cmd
		else {
			anyhow::bail!("expected temperate-conifer render command");
		};
		assert!((helper.inner.geometry.inner.scale.stalk_height - 18.0).abs() < 1e-5);
		assert!((helper.inner.fronds_per_joint.start - 1.0).abs() < 1e-5);
		assert!((helper.inner.frond_length_fraction.start - 0.04).abs() < 1e-5);
		assert!((helper.inner.frond_spawn_fraction - 0.7).abs() < 1e-5);
		let cfg = Render::TemperateConifer(helper).into_render_config();
		let RenderSubject::TemperateConifer(tree) = cfg.subject else {
			anyhow::bail!("expected temperate conifer subject");
		};
		assert!((tree.geometry.inner.scale.stalk_height - 18.0).abs() < 1e-5);
		assert!((tree.frond_spawn_fraction - 0.7).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn temperate_preset_shortens_limbs_vs_friends() {
		let friends = FriendsConiferSbs::default();
		let mut temperate = FriendsConiferSbs::default();
		temperate.apply_temperate_preset();
		assert!(
			temperate.projection.length_fraction_of_height.start
				< friends.projection.length_fraction_of_height.start
		);
		assert!(temperate.growth.angle_tolerance_degrees > friends.growth.angle_tolerance_degrees);
	}

	#[test]
	fn frond_crown_command_preserves_shape_params() -> Result<()> {
		let cmd =
			crate::commands::PlaygroundCommand::parse_line("render frond-crown --frond-count 15")
				.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::FrondCrown(helper)) = cmd else {
			anyhow::bail!("expected frond-crown render command");
		};
		assert_eq!(helper.inner.frond_count, 15);
		let cfg = Render::FrondCrown(helper).into_render_config();
		let RenderSubject::FrondCrown(crown) = cfg.subject else {
			anyhow::bail!("expected frond crown subject");
		};
		assert_eq!(crown.shape.frond_count, 15);
		Ok(())
	}

	#[test]
	fn braid_grass_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render braid-grass")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::BraidGrass(helper)) = cmd else {
			anyhow::bail!("expected braid-grass render command");
		};
		let grass = helper.configured_braid_grass();
		assert!(grass.grove.variant_weights.is_none());
		let placements = grass.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert_eq!(grass.placement_cells().len(), 48 * 48);
		assert!(
			placements.len() >= 8,
			"expected a visible braid-grass preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn tropical_tufts_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render tropical-tufts")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::TropicalTufts(helper)) = cmd else {
			anyhow::bail!("expected tropical-tufts render command");
		};
		let tufts = helper.configured_tropical_tufts();
		assert!(tufts.grove.variant_weights.is_none());
		let placements = tufts.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert_eq!(tufts.placement_cells().len(), 31 * 31);
		assert!(
			!placements.is_empty(),
			"expected a visible tropical-tufts preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn common_tufts_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render common-tufts")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::CommonTufts(helper)) = cmd else {
			anyhow::bail!("expected common-tufts render command");
		};
		let tufts = helper.configured_common_tufts();
		assert!(tufts.grove.variant_weights.is_none());
		let placements = tufts.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert_eq!(tufts.placement_cells().len(), 50 * 50);
		assert!(
			!placements.is_empty(),
			"expected a visible common-tufts preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn common_tufts_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render common-tufts --elevation 0.4 --grove-extent-xz 8 --cell-extent-xz 2,2",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::CommonTufts(helper)) = cmd else {
			anyhow::bail!("expected common-tufts render command");
		};
		assert!((helper.grove_extent_xz - 8.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(2.0)));
		let tufts = helper.configured_common_tufts();
		assert_eq!(tufts.placement_cells().len(), 16);
		assert!((tufts.terrain.elevation - 0.4).abs() < 1e-5);
		assert!(!tufts.placements().is_empty());
		let cfg = Render::CommonTufts(helper).into_render_config();
		let RenderSubject::CommonTufts(subject) = cfg.subject else {
			anyhow::bail!("expected common tufts subject");
		};
		assert_eq!(subject.placement_cells().len(), 16);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn wild_grass_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render wild-grass")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::WildGrass(helper)) = cmd else {
			anyhow::bail!("expected wild-grass render command");
		};
		let grass = helper.configured_wild_grass();
		assert!(grass.grove.variant_weights.is_none());
		let placements = grass.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible wild-grass preview with default flags, got {} placements",
			placements.len()
		);
		// Dense grove: most cells should place vegetation.
		let cells = grass.placement_cells().len();
		assert!(
			placements.len() * 2 >= cells,
			"expected dense wild grass (got {} placements in {} cells)",
			placements.len(),
			cells
		);
		Ok(())
	}

	#[test]
	fn tall_grass_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render tall-grass")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::TallGrass(helper)) = cmd else {
			anyhow::bail!("expected tall-grass render command");
		};
		let grass = helper.configured_tall_grass();
		assert!(grass.grove.variant_weights.is_none());
		let placements = grass.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible tall-grass preview with default flags, got {} placements",
			placements.len()
		);
		let cells = grass.placement_cells().len();
		assert!(
			placements.len() * 2 >= cells,
			"expected dense tall grass (got {} placements in {} cells)",
			placements.len(),
			cells
		);
		Ok(())
	}

	#[test]
	fn tall_grass_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render tall-grass --elevation 0.40 --grove-extent-xz 14 --cell-extent-xz 1.75,1.75",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::TallGrass(helper)) = cmd else {
			anyhow::bail!("expected tall-grass render command");
		};
		assert!((helper.grove_extent_xz - 14.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(1.75)));
		let grass = helper.configured_tall_grass();
		assert_eq!(grass.placement_cells().len(), 64);
		assert!((grass.terrain.elevation - 0.40).abs() < 1e-5);
		assert!(!grass.placements().is_empty());
		let cfg = Render::TallGrass(helper).into_render_config();
		let RenderSubject::TallGrass(subject) = cfg.subject else {
			anyhow::bail!("expected tall grass subject");
		};
		assert_eq!(subject.placement_cells().len(), 64);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn wild_grass_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render wild-grass --elevation 0.35 --grove-extent-xz 14 --cell-extent-xz 1.75,1.75",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::WildGrass(helper)) = cmd else {
			anyhow::bail!("expected wild-grass render command");
		};
		assert!((helper.grove_extent_xz - 14.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(1.75)));
		let grass = helper.configured_wild_grass();
		assert_eq!(grass.placement_cells().len(), 64);
		assert!((grass.terrain.elevation - 0.35).abs() < 1e-5);
		assert!(!grass.placements().is_empty());
		let cfg = Render::WildGrass(helper).into_render_config();
		let RenderSubject::WildGrass(subject) = cfg.subject else {
			anyhow::bail!("expected wild grass subject");
		};
		assert_eq!(subject.placement_cells().len(), 64);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn monster_grass_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render monster-grass")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::MonsterGrass(helper)) = cmd else {
			anyhow::bail!("expected monster-grass render command");
		};
		let grass = helper.configured_monster_grass();
		assert!(grass.grove.variant_weights.is_none());
		let placements = grass.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible monster-grass preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn monster_grass_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render monster-grass --elevation 0.35 --grove-extent-xz 25 --cell-extent-xz 2.5,2.5",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::MonsterGrass(helper)) = cmd else {
			anyhow::bail!("expected monster-grass render command");
		};
		assert!((helper.grove_extent_xz - 25.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(2.5)));
		let grass = helper.configured_monster_grass();
		assert_eq!(grass.placement_cells().len(), 100);
		assert!((grass.terrain.elevation - 0.35).abs() < 1e-5);
		assert!(!grass.placements().is_empty());
		let cfg = Render::MonsterGrass(helper).into_render_config();
		let RenderSubject::MonsterGrass(subject) = cfg.subject else {
			anyhow::bail!("expected monster grass subject");
		};
		assert_eq!(subject.placement_cells().len(), 100);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn riverine_green_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render riverine-green")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::RiverineGreen(helper)) = cmd else {
			anyhow::bail!("expected riverine-green render command");
		};
		let grove = helper.configured_riverine_green();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible riverine-green preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn riverine_green_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render riverine-green --elevation 0.25 --grove-extent-xz 28 --cell-extent-xz 7.0,7.0",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::RiverineGreen(helper)) = cmd else {
			anyhow::bail!("expected riverine-green render command");
		};
		assert!((helper.grove_extent_xz - 28.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(7.0)));
		let grove = helper.configured_riverine_green();
		assert_eq!(grove.placement_cells().len(), 16);
		assert!((grove.terrain.elevation - 0.25).abs() < 1e-5);
		assert!(!grove.placements().is_empty());
		let cfg = Render::RiverineGreen(helper).into_render_config();
		let RenderSubject::RiverineGreen(subject) = cfg.subject else {
			anyhow::bail!("expected riverine green subject");
		};
		assert_eq!(subject.placement_cells().len(), 16);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn low_bush_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render low-bush")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::LowBush(helper)) = cmd else {
			anyhow::bail!("expected low-bush render command");
		};
		let grove = helper.configured_low_bush();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible low-bush preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn low_bush_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render low-bush --elevation 0.30 --grove-extent-xz 34 --cell-extent-xz 4.25,4.25",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::LowBush(helper)) = cmd else {
			anyhow::bail!("expected low-bush render command");
		};
		assert!((helper.grove_extent_xz - 34.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(4.25)));
		let grove = helper.configured_low_bush();
		assert_eq!(grove.placement_cells().len(), 64);
		assert!((grove.terrain.elevation - 0.30).abs() < 1e-5);
		assert!(!grove.placements().is_empty());
		let cfg = Render::LowBush(helper).into_render_config();
		let RenderSubject::LowBush(subject) = cfg.subject else {
			anyhow::bail!("expected low bush subject");
		};
		assert_eq!(subject.placement_cells().len(), 64);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn high_bush_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render high-bush")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::HighBush(helper)) = cmd else {
			anyhow::bail!("expected high-bush render command");
		};
		let grove = helper.configured_high_bush();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible high-bush preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn high_bush_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render high-bush --elevation 0.35 --grove-extent-xz 46 --cell-extent-xz 5.75,5.75",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::HighBush(helper)) = cmd else {
			anyhow::bail!("expected high-bush render command");
		};
		assert!((helper.grove_extent_xz - 46.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(5.75)));
		let grove = helper.configured_high_bush();
		assert_eq!(grove.placement_cells().len(), 64);
		assert!((grove.terrain.elevation - 0.35).abs() < 1e-5);
		assert!(!grove.placements().is_empty());
		let cfg = Render::HighBush(helper).into_render_config();
		let RenderSubject::HighBush(subject) = cfg.subject else {
			anyhow::bail!("expected high bush subject");
		};
		assert_eq!(subject.placement_cells().len(), 64);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn spotty_bushes_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render spotty-bushes")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::SpottyBushes(helper)) = cmd else {
			anyhow::bail!("expected spotty-bushes render command");
		};
		let grove = helper.configured_spotty_bushes();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible spotty-bushes preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn spotty_bushes_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render spotty-bushes --elevation 0.35 --grove-extent-xz 39 --cell-extent-xz 8.5,8.5",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::SpottyBushes(helper)) = cmd else {
			anyhow::bail!("expected spotty-bushes render command");
		};
		assert!((helper.grove_extent_xz - 39.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(8.5)));
		let grove = helper.configured_spotty_bushes();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 25);
		assert!((grove.terrain.elevation - 0.35).abs() < 1e-5);
		assert!(!grove.placements().is_empty());
		let cfg = Render::SpottyBushes(helper).into_render_config();
		let RenderSubject::SpottyBushes(subject) = cfg.subject else {
			anyhow::bail!("expected spotty bushes subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn unending_jungle_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render unending-jungle")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::UnendingJungle(helper)) = cmd else {
			anyhow::bail!("expected unending-jungle render command");
		};
		let grove = helper.configured_unending_jungle();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible unending-jungle preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn unending_jungle_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render unending-jungle --elevation 0.35 --grove-extent-xz 39 --cell-extent-xz 10.5,10.5",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::UnendingJungle(helper)) = cmd else {
			anyhow::bail!("expected unending-jungle render command");
		};
		assert!((helper.grove_extent_xz - 39.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(10.5)));
		let grove = helper.configured_unending_jungle();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 16);
		assert!((grove.terrain.elevation - 0.35).abs() < 1e-5);
		assert!(!grove.placements().is_empty());
		let cfg = Render::UnendingJungle(helper).into_render_config();
		let RenderSubject::UnendingJungle(subject) = cfg.subject else {
			anyhow::bail!("expected unending jungle subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn strange_oasis_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render strange-oasis")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::StrangeOasis(helper)) = cmd else {
			anyhow::bail!("expected strange-oasis render command");
		};
		let grove = helper.configured_strange_oasis();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible strange-oasis preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn strange_oasis_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render strange-oasis --elevation 0.25 --grove-extent-xz 39 --cell-extent-xz 12,12",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::StrangeOasis(helper)) = cmd else {
			anyhow::bail!("expected strange-oasis render command");
		};
		assert!((helper.grove_extent_xz - 39.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(12.0)));
		let grove = helper.configured_strange_oasis();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 16);
		assert!((grove.terrain.elevation - 0.25).abs() < 1e-5);
		assert!(!grove.placements().is_empty());
		let cfg = Render::StrangeOasis(helper).into_render_config();
		let RenderSubject::StrangeOasis(subject) = cfg.subject else {
			anyhow::bail!("expected strange oasis subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn shamanhome_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render shamanhome")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::Shamanhome(helper)) = cmd else {
			anyhow::bail!("expected shamanhome render command");
		};
		let grove = helper.configured_shamanhome();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible shamanhome preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn shamanhome_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render shamanhome --elevation 0.25 --grove-extent-xz 39 --cell-extent-xz 10.5,10.5",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::Shamanhome(helper)) = cmd else {
			anyhow::bail!("expected shamanhome render command");
		};
		assert!((helper.grove_extent_xz - 39.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(10.5)));
		let grove = helper.configured_shamanhome();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 16);
		assert!((grove.terrain.elevation - 0.25).abs() < 1e-5);
		assert!(!grove.placements().is_empty());
		let cfg = Render::Shamanhome(helper).into_render_config();
		let RenderSubject::Shamanhome(subject) = cfg.subject else {
			anyhow::bail!("expected shamanhome subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn goettingen_follow_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render goettingen-follow")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::GoettingenFollow(helper)) = cmd else {
			anyhow::bail!("expected goettingen-follow render command");
		};
		let grove = helper.configured_goettingen_follow();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible goettingen-follow preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn goettingen_follow_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render goettingen-follow --elevation 0.25 --grove-extent-xz 39 --cell-extent-xz 9,9",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::GoettingenFollow(helper)) = cmd else {
			anyhow::bail!("expected goettingen-follow render command");
		};
		assert!((helper.grove_extent_xz - 39.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(9.0)));
		let grove = helper.configured_goettingen_follow();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 25);
		assert!((grove.terrain.elevation - 0.25).abs() < 1e-5);
		assert!(!grove.placements().is_empty());
		let cfg = Render::GoettingenFollow(helper).into_render_config();
		let RenderSubject::GoettingenFollow(subject) = cfg.subject else {
			anyhow::bail!("expected goettingen follow subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn conifer_sapling_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render conifer-sapling")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::ConiferSapling(helper)) = cmd else {
			anyhow::bail!("expected conifer-sapling render command");
		};
		let grove = helper.configured_conifer_sapling();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible conifer-sapling preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn conifer_sapling_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render conifer-sapling --elevation 0.55 --grove-extent-xz 39 --cell-extent-xz 10.5,10.5",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::ConiferSapling(helper)) = cmd else {
			anyhow::bail!("expected conifer-sapling render command");
		};
		assert!((helper.grove_extent_xz - 39.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(10.5)));
		let grove = helper.configured_conifer_sapling();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 16);
		assert!((grove.terrain.elevation - 0.55).abs() < 1e-5);
		assert!(!grove.placements().is_empty());
		let cfg = Render::ConiferSapling(helper).into_render_config();
		let RenderSubject::ConiferSapling(subject) = cfg.subject else {
			anyhow::bail!("expected conifer sapling subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn arid_conifer_sapling_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render arid-conifer-sapling")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::AridConiferSapling(helper)) = cmd
		else {
			anyhow::bail!("expected arid-conifer-sapling render command");
		};
		let grove = helper.configured_arid_conifer_sapling();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible arid-conifer-sapling preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn arid_conifer_sapling_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render arid-conifer-sapling --grove-extent-xz 39 --cell-extent-xz 13.5,13.5",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::AridConiferSapling(helper)) = cmd
		else {
			anyhow::bail!("expected arid-conifer-sapling render command");
		};
		assert!((helper.grove_extent_xz - 39.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(13.5)));
		let grove = helper.configured_arid_conifer_sapling();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 9);
		assert!(!grove.placements().is_empty());
		let cfg = Render::AridConiferSapling(helper).into_render_config();
		let RenderSubject::AridConiferSapling(subject) = cfg.subject else {
			anyhow::bail!("expected arid conifer sapling subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn jungle_lower_massives_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render jungle-lower-massives")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::JungleLowerMassives(helper)) = cmd
		else {
			anyhow::bail!("expected jungle-lower-massives render command");
		};
		let grove = helper.configured_jungle_lower_massives();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible jungle-lower-massives preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn jungle_lower_massives_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render jungle-lower-massives --grove-extent-xz 92 --cell-extent-xz 23,23",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::JungleLowerMassives(helper)) = cmd
		else {
			anyhow::bail!("expected jungle-lower-massives render command");
		};
		assert!((helper.grove_extent_xz - 92.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(23.0)));
		let grove = helper.configured_jungle_lower_massives();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 16);
		assert!(!grove.placements().is_empty());
		let cfg = Render::JungleLowerMassives(helper).into_render_config();
		let RenderSubject::JungleLowerMassives(subject) = cfg.subject else {
			anyhow::bail!("expected jungle lower massives subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn jungle_massives_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render jungle-massives --grove-extent-xz 220",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::JungleMassives(helper)) = cmd else {
			anyhow::bail!("expected jungle-massives render command");
		};
		let grove = helper.configured_jungle_massives();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert!((helper.grove_extent_xz - 220.0).abs() < 1e-5);
		assert!(
			!placements.is_empty(),
			"expected a visible jungle-massives preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn jungle_massives_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render jungle-massives --grove-extent-xz 220 --cell-extent-xz 44,44",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::JungleMassives(helper)) = cmd else {
			anyhow::bail!("expected jungle-massives render command");
		};
		assert!((helper.grove_extent_xz - 220.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(44.0)));
		let grove = helper.configured_jungle_massives();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 25);
		assert!(!grove.placements().is_empty());
		let cfg = Render::JungleMassives(helper).into_render_config();
		let RenderSubject::JungleMassives(subject) = cfg.subject else {
			anyhow::bail!("expected jungle massives subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn temperate_lower_massives_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render temperate-lower-massives --elevation 0.35",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::TemperateLowerMassives(helper)) = cmd
		else {
			anyhow::bail!("expected temperate-lower-massives render command");
		};
		let grove = helper.configured_temperate_lower_massives();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible temperate-lower-massives preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn temperate_lower_massives_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render temperate-lower-massives --elevation 0.35 --grove-extent-xz 92 --cell-extent-xz 26,26",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::TemperateLowerMassives(helper)) = cmd
		else {
			anyhow::bail!("expected temperate-lower-massives render command");
		};
		assert!((helper.grove_extent_xz - 92.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(26.0)));
		let grove = helper.configured_temperate_lower_massives();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 16);
		assert!(!grove.placements().is_empty());
		let cfg = Render::TemperateLowerMassives(helper).into_render_config();
		let RenderSubject::TemperateLowerMassives(subject) = cfg.subject else {
			anyhow::bail!("expected temperate lower massives subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn palm_shade_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render palm-shade --grove-extent-xz 220",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::PalmShade(helper)) = cmd else {
			anyhow::bail!("expected palm-shade render command");
		};
		let grove = helper.configured_palm_shade();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert!((helper.grove_extent_xz - 220.0).abs() < 1e-5);
		assert!(
			!placements.is_empty(),
			"expected a visible palm-shade preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn palm_shade_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render palm-shade --grove-extent-xz 220 --cell-extent-xz 24,24",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::PalmShade(helper)) = cmd else {
			anyhow::bail!("expected palm-shade render command");
		};
		assert!((helper.grove_extent_xz - 220.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(24.0)));
		let grove = helper.configured_palm_shade();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 100);
		assert!(!grove.placements().is_empty());
		let cfg = Render::PalmShade(helper).into_render_config();
		let RenderSubject::PalmShade(subject) = cfg.subject else {
			anyhow::bail!("expected palm shade subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn riparian_mix_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render riparian-mix --grove-extent-xz 180",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::RiparianMix(helper)) = cmd else {
			anyhow::bail!("expected riparian-mix render command");
		};
		let grove = helper.configured_riparian_mix();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert!((helper.grove_extent_xz - 180.0).abs() < 1e-5);
		assert!(
			!placements.is_empty(),
			"expected a visible riparian-mix preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn riparian_mix_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render riparian-mix --grove-extent-xz 180 --cell-extent-xz 17,17",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::RiparianMix(helper)) = cmd else {
			anyhow::bail!("expected riparian-mix render command");
		};
		assert!((helper.grove_extent_xz - 180.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(17.0)));
		let grove = helper.configured_riparian_mix();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 121);
		assert!(!grove.placements().is_empty());
		let cfg = Render::RiparianMix(helper).into_render_config();
		let RenderSubject::RiparianMix(subject) = cfg.subject else {
			anyhow::bail!("expected riparian mix subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn alpine_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render alpine --grove-extent-xz 220",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::Alpine(helper)) = cmd else {
			anyhow::bail!("expected alpine render command");
		};
		let grove = helper.configured_alpine();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert!((helper.grove_extent_xz - 220.0).abs() < 1e-5);
		assert!(
			!placements.is_empty(),
			"expected a visible alpine preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn alpine_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render alpine --grove-extent-xz 220 --cell-extent-xz 27,27",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::Alpine(helper)) = cmd else {
			anyhow::bail!("expected alpine render command");
		};
		assert!((helper.grove_extent_xz - 220.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(27.0)));
		let grove = helper.configured_alpine();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 81);
		assert!(!grove.placements().is_empty());
		let cfg = Render::Alpine(helper).into_render_config();
		let RenderSubject::Alpine(subject) = cfg.subject else {
			anyhow::bail!("expected alpine subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn dryland_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render dryland --grove-extent-xz 280",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::Dryland(helper)) = cmd else {
			anyhow::bail!("expected dryland render command");
		};
		let grove = helper.configured_dryland();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert!((helper.grove_extent_xz - 280.0).abs() < 1e-5);
		assert!(
			!placements.is_empty(),
			"expected a visible dryland preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn dryland_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render dryland --grove-extent-xz 280 --cell-extent-xz 35,35",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::Dryland(helper)) = cmd else {
			anyhow::bail!("expected dryland render command");
		};
		assert!((helper.grove_extent_xz - 280.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(35.0)));
		let grove = helper.configured_dryland();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 64);
		assert!(!grove.placements().is_empty());
		let cfg = Render::Dryland(helper).into_render_config();
		let RenderSubject::Dryland(subject) = cfg.subject else {
			anyhow::bail!("expected dryland subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn storytellers_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render storytellers --grove-extent-xz 220",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::Storytellers(helper)) = cmd else {
			anyhow::bail!("expected storytellers render command");
		};
		let grove = helper.configured_storytellers();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert!((helper.grove_extent_xz - 220.0).abs() < 1e-5);
		assert!(
			!placements.is_empty(),
			"expected a visible storytellers preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn storytellers_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render storytellers --grove-extent-xz 220 --cell-extent-xz 22,22",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::Storytellers(helper)) = cmd else {
			anyhow::bail!("expected storytellers render command");
		};
		assert!((helper.grove_extent_xz - 220.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(22.0)));
		let grove = helper.configured_storytellers();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 100);
		assert!(!grove.placements().is_empty());
		let cfg = Render::Storytellers(helper).into_render_config();
		let RenderSubject::Storytellers(subject) = cfg.subject else {
			anyhow::bail!("expected storytellers subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn trade_winds_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render trade-winds --grove-extent-xz 260",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::TradeWinds(helper)) = cmd else {
			anyhow::bail!("expected trade-winds render command");
		};
		let grove = helper.configured_trade_winds();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert!((helper.grove_extent_xz - 260.0).abs() < 1e-5);
		assert!(
			!placements.is_empty(),
			"expected a visible trade-winds preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn trade_winds_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render trade-winds --grove-extent-xz 260 --cell-extent-xz 26,26",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::TradeWinds(helper)) = cmd else {
			anyhow::bail!("expected trade-winds render command");
		};
		assert!((helper.grove_extent_xz - 260.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(26.0)));
		let grove = helper.configured_trade_winds();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 100);
		assert!(!grove.placements().is_empty());
		let cfg = Render::TradeWinds(helper).into_render_config();
		let RenderSubject::TradeWinds(subject) = cfg.subject else {
			anyhow::bail!("expected trade-winds subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn wandering_acacia_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render wandering-acacia --grove-extent-xz 300",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::WanderingAcacia(helper)) = cmd else {
			anyhow::bail!("expected wandering-acacia render command");
		};
		let grove = helper.configured_wandering_acacia();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert!((helper.grove_extent_xz - 300.0).abs() < 1e-5);
		assert!(
			!placements.is_empty(),
			"expected a visible wandering-acacia preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn wandering_acacia_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render wandering-acacia --grove-extent-xz 300 --cell-extent-xz 37,37",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::WanderingAcacia(helper)) = cmd else {
			anyhow::bail!("expected wandering-acacia render command");
		};
		assert!((helper.grove_extent_xz - 300.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(37.0)));
		let grove = helper.configured_wandering_acacia();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 81);
		assert!(!grove.placements().is_empty());
		let cfg = Render::WanderingAcacia(helper).into_render_config();
		let RenderSubject::WanderingAcacia(subject) = cfg.subject else {
			anyhow::bail!("expected wandering-acacia subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn leeward_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render leeward --grove-extent-xz 220",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::Leeward(helper)) = cmd else {
			anyhow::bail!("expected leeward render command");
		};
		let grove = helper.configured_leeward();
		let placements = grove.placements();
		assert!(
			!placements.is_empty(),
			"expected a visible leeward preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn leeward_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render leeward --grove-extent-xz 220 --cell-extent-xz 19,19",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::Leeward(helper)) = cmd else {
			anyhow::bail!("expected leeward render command");
		};
		assert!((helper.grove_extent_xz - 220.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(19.0)));
		let grove = helper.configured_leeward();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 144);
		assert!(!grove.placements().is_empty());
		let cfg = Render::Leeward(helper).into_render_config();
		let RenderSubject::Leeward(subject) = cfg.subject else {
			anyhow::bail!("expected leeward subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn christmas_taiga_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render christmas-taiga --grove-extent-xz 200",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::ChristmasTaiga(helper)) = cmd else {
			anyhow::bail!("expected christmas-taiga render command");
		};
		let grove = helper.configured_christmas_taiga();
		let placements = grove.placements();
		assert!(
			!placements.is_empty(),
			"expected a visible christmas-taiga preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn christmas_taiga_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render christmas-taiga --grove-extent-xz 200 --cell-extent-xz 16,16",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::ChristmasTaiga(helper)) = cmd else {
			anyhow::bail!("expected christmas-taiga render command");
		};
		assert!((helper.grove_extent_xz - 200.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(16.0)));
		let grove = helper.configured_christmas_taiga();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 169);
		assert!(!grove.placements().is_empty());
		let cfg = Render::ChristmasTaiga(helper).into_render_config();
		let RenderSubject::ChristmasTaiga(subject) = cfg.subject else {
			anyhow::bail!("expected christmas-taiga subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn bush_scrub_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render bush-scrub")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::BushScrub(helper)) = cmd else {
			anyhow::bail!("expected bush-scrub render command");
		};
		let scrub = helper.configured_bush_scrub();
		assert!(scrub.grove.variant_weights.is_none());
		let placements = scrub.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible bush-scrub preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn bush_scrub_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render bush-scrub --elevation 0.40 --grove-extent-xz 35 --cell-extent-xz 2.5,2.5",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::BushScrub(helper)) = cmd else {
			anyhow::bail!("expected bush-scrub render command");
		};
		assert!((helper.grove_extent_xz - 35.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(2.5)));
		let scrub = helper.configured_bush_scrub();
		assert_eq!(scrub.placement_cells().len(), 196);
		assert!((scrub.terrain.elevation - 0.40).abs() < 1e-5);
		assert!(!scrub.placements().is_empty());
		let cfg = Render::BushScrub(helper).into_render_config();
		let RenderSubject::BushScrub(subject) = cfg.subject else {
			anyhow::bail!("expected bush scrub subject");
		};
		assert_eq!(subject.placement_cells().len(), 196);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn tropical_undergrowth_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render tropical-undergrowth")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::TropicalUndergrowth(helper)) = cmd
		else {
			anyhow::bail!("expected tropical-undergrowth render command");
		};
		let grove = helper.configured_tropical_undergrowth();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible tropical-undergrowth preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn tropical_undergrowth_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render tropical-undergrowth --elevation 0.35 --grove-extent-xz 35 --cell-extent-xz 5,5",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::TropicalUndergrowth(helper)) = cmd
		else {
			anyhow::bail!("expected tropical-undergrowth render command");
		};
		assert!((helper.grove_extent_xz - 35.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(5.0)));
		let grove = helper.configured_tropical_undergrowth();
		assert_eq!(grove.placement_cells().len(), 49);
		assert!((grove.terrain.elevation - 0.35).abs() < 1e-5);
		assert!(!grove.placements().is_empty());
		let cfg = Render::TropicalUndergrowth(helper).into_render_config();
		let RenderSubject::TropicalUndergrowth(subject) = cfg.subject else {
			anyhow::bail!("expected tropical undergrowth subject");
		};
		assert_eq!(subject.placement_cells().len(), 49);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn tropical_thicket_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render tropical-thicket")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::TropicalThicket(helper)) = cmd
		else {
			anyhow::bail!("expected tropical-thicket render command");
		};
		let grove = helper.configured_tropical_thicket();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible tropical-thicket preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn tropical_thicket_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render tropical-thicket --elevation 0.35 --grove-extent-xz 39 --cell-extent-xz 6.5,6.5",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::TropicalThicket(helper)) = cmd
		else {
			anyhow::bail!("expected tropical-thicket render command");
		};
		assert!((helper.grove_extent_xz - 39.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(6.5)));
		let grove = helper.configured_tropical_thicket();
		assert_eq!(grove.placement_cells().len(), 36);
		assert!((grove.terrain.elevation - 0.35).abs() < 1e-5);
		assert!(!grove.placements().is_empty());
		let cfg = Render::TropicalThicket(helper).into_render_config();
		let RenderSubject::TropicalThicket(subject) = cfg.subject else {
			anyhow::bail!("expected tropical thicket subject");
		};
		assert_eq!(subject.placement_cells().len(), 36);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn jerrys_chaparral_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render jerrys-chaparral")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::JerrysChaparral(helper)) = cmd
		else {
			anyhow::bail!("expected jerrys-chaparral render command");
		};
		let grove = helper.configured_jerrys_chaparral();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible jerrys-chaparral preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn jerrys_chaparral_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render jerrys-chaparral --elevation 0.35 --grove-extent-xz 39 --cell-extent-xz 6.5,6.5",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::JerrysChaparral(helper)) = cmd
		else {
			anyhow::bail!("expected jerrys-chaparral render command");
		};
		assert!((helper.grove_extent_xz - 39.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(6.5)));
		let grove = helper.configured_jerrys_chaparral();
		assert_eq!(grove.placement_cells().len(), 36);
		assert!((grove.terrain.elevation - 0.35).abs() < 1e-5);
		assert!(!grove.placements().is_empty());
		let cfg = Render::JerrysChaparral(helper).into_render_config();
		let RenderSubject::JerrysChaparral(subject) = cfg.subject else {
			anyhow::bail!("expected jerrys chaparral subject");
		};
		assert_eq!(subject.placement_cells().len(), 36);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn levantine_scrub_defaults_spawn_placements() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render levantine-scrub")
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::LevantineScrub(helper)) = cmd else {
			anyhow::bail!("expected levantine-scrub render command");
		};
		let grove = helper.configured_levantine_scrub();
		assert!(grove.grove.variant_weights.is_none());
		let placements = grove.placements();
		assert_eq!(helper.grove_extent_xz, 100.0);
		assert!(
			!placements.is_empty(),
			"expected a visible levantine-scrub preview with default flags, got {} placements",
			placements.len()
		);
		Ok(())
	}

	#[test]
	fn levantine_scrub_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render levantine-scrub --elevation 0.25 --grove-extent-xz 39 --cell-extent-xz 5.75,5.75",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::LevantineScrub(helper)) = cmd else {
			anyhow::bail!("expected levantine-scrub render command");
		};
		assert!((helper.grove_extent_xz - 39.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(5.75)));
		let grove = helper.configured_levantine_scrub();
		let cell_count = grove.placement_cells().len();
		assert_eq!(cell_count, 49);
		assert!((grove.terrain.elevation - 0.25).abs() < 1e-5);
		assert!(!grove.placements().is_empty());
		let cfg = Render::LevantineScrub(helper).into_render_config();
		let RenderSubject::LevantineScrub(subject) = cfg.subject else {
			anyhow::bail!("expected levantine scrub subject");
		};
		assert_eq!(subject.placement_cells().len(), cell_count);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn tropical_tufts_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render tropical-tufts --elevation 0.4 --grove-extent-xz 26 --cell-extent-xz 3.25,3.25",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::TropicalTufts(helper)) = cmd else {
			anyhow::bail!("expected tropical-tufts render command");
		};
		assert!((helper.grove_extent_xz - 26.0).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(3.25)));
		let tufts = helper.configured_tropical_tufts();
		assert_eq!(tufts.placement_cells().len(), 64);
		assert!((tufts.terrain.elevation - 0.4).abs() < 1e-5);
		assert!(!tufts.placements().is_empty());
		let cfg = Render::TropicalTufts(helper).into_render_config();
		let RenderSubject::TropicalTufts(subject) = cfg.subject else {
			anyhow::bail!("expected tropical tufts subject");
		};
		assert_eq!(subject.placement_cells().len(), 64);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn braid_grass_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render braid-grass --variant-weights 0.0,9.0,x,x,x,x,x,x,x,x --elevation 0.4 --grove-extent-xz 12.75 --cell-extent-xz 4.25,4.25",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::BraidGrass(helper)) = cmd else {
			anyhow::bail!("expected braid-grass render command");
		};
		assert!((helper.grove_extent_xz - 12.75).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Some(Vec2::splat(4.25)));
		let grass = helper.configured_braid_grass();
		assert_eq!(grass.placement_cells().len(), 9);
		assert!((grass.terrain.elevation - 0.4).abs() < 1e-5);
		assert!(!grass.placements().is_empty());
		let cfg = Render::BraidGrass(helper).into_render_config();
		let RenderSubject::BraidGrass(subject) = cfg.subject else {
			anyhow::bail!("expected braid grass subject");
		};
		assert_eq!(subject.placement_cells().len(), 9);
		assert!(!subject.placements().is_empty());
		Ok(())
	}

	#[test]
	fn tuft_patch_command_preserves_patch_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render tuft-patch --clump-count 7 --patch-extent-xz 2.5 --blade-count 6 --seed 42",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::TuftPatch(helper)) = cmd else {
			anyhow::bail!("expected tuft-patch render command");
		};
		assert_eq!(helper.inner.clump_count, 7);
		assert!((helper.inner.patch_extent_xz - 2.5).abs() < 1e-5);
		assert_eq!(helper.inner.shape.blade_count, 6);
		let cfg = Render::TuftPatch(helper).into_render_config();
		let RenderSubject::TuftPatch(patch) = cfg.subject else {
			anyhow::bail!("expected tuft patch subject");
		};
		assert_eq!(patch.clump_anchors().len(), 7);
		Ok(())
	}

	#[test]
	fn render_honu_banyan_parses_geometry_flags() -> anyhow::Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render honu-banyan --tree-height 20 --rings 2x6",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::HonuBanyan(helper)) = cmd else {
			anyhow::bail!("expected honu banyan render");
		};
		let cfg = Render::HonuBanyan(helper).into_render_config();
		let RenderSubject::HonuBanyan(tree) = cfg.subject else {
			anyhow::bail!("expected honu banyan subject");
		};
		assert_eq!(tree.geometry.scale.tree_height, 20.0);
		assert_eq!(tree.geometry.rings.layout.first, 2);
		Ok(())
	}
}
