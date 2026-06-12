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
	RenderBladeTuft, RenderBraidGrass, RenderBraidOakTree, RenderBuddhaHandTuft,
	RenderCommonTufts, RenderConfig, RenderDatePalm, RenderFriendsConifer, RenderFrondCrown,
	RenderHighBushShoots, RenderHonuBanyan, RenderJungleGrowth, RenderJungleStorybookTree,
	RenderKamakuraTorch, RenderLiamsConifer, RenderModerateLodFrondCrown, RenderNorthernConifer,
	RenderPalmBush, RenderPenmarchTorch, RenderRorysHeadTrained, RenderSopesBanyan,
	RenderSpearTuft, RenderStorybookTree, RenderSubject, RenderSucculentTuft,
	RenderTemperateConifer, RenderTropicalTufts, RenderTuftPatch, RenderVaseTree,
	RenderWaialeaPalm, RenderWeepingTuft,
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
			Self::BraidGrass(h) => h
				.render
				.config_with(RenderSubject::BraidGrass(h.configured_braid_grass())),
			Self::TropicalTufts(h) => h
				.render
				.config_with(RenderSubject::TropicalTufts(h.configured_tropical_tufts())),
			Self::CommonTufts(h) => h
				.render
				.config_with(RenderSubject::CommonTufts(h.configured_common_tufts())),
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
			Self::ModerateLodFrondCrown(h) => {
				h.config_with(RenderSubject::ModerateLodFrondCrown(
					RenderModerateLodFrondCrown::from_shape(h.inner.clone(), Default::default()),
				))
			}
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
			"render braid-grass --variant-weights 0.0,9.0,x,x,x,x,x --elevation 0.4 --grove-extent-xz 12.75 --cell-extent-xz 4.25,4.25",
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
