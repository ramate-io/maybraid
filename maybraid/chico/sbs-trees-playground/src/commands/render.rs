pub mod blade_tuft;
pub mod braid_grass;
pub mod braid_oak_tree;
pub mod buddha_hand_tuft;
pub mod common_high_bush;
pub mod date_palm;
pub mod friends_conifer;
pub mod frond_crown;
pub mod high_bush_shoots;
pub mod jungle_growth;
pub mod jungle_storybook_tree;
pub mod kamakura_torch;
pub mod liams_conifer;
pub mod moderate_lod_frond_crown;
pub mod northern_conifer;
pub mod palm_bush;
pub mod penmarch_torch;
pub mod plugin;
pub mod rorys_head_trained;
pub mod storybook_tree;
pub mod temperate_conifer;
pub mod vase_tree;
pub mod waialea_palm;

pub use plugin::RenderCommandsPlugin;
pub mod honu_banyan;
pub mod sopes_banyan;
pub mod spear_tuft;
pub mod succulent_tuft;
pub mod weeping_tuft;

use bevy::prelude::*;
use chico_groves::BraidGrassDefinition;
use clap::Subcommand;

use crate::render::{RenderConfig, RenderSubject};
pub use blade_tuft::BladeTuftRenderHelper;
pub use braid_grass::BraidGrassRenderHelper;
pub use braid_oak_tree::BraidOakTreeRenderHelper;
pub use buddha_hand_tuft::BuddhaHandTuftRenderHelper;
pub use common_high_bush::CommonHighBushRenderHelper;
pub use date_palm::DatePalmRenderHelper;
pub use friends_conifer::FriendsConiferRenderHelper;
pub use frond_crown::FrondCrownRenderHelper;
pub use high_bush_shoots::HighBushShootsRenderHelper;
pub use honu_banyan::HonuBanyanRenderHelper;
pub use jungle_growth::JungleGrowthRenderHelper;
pub use jungle_storybook_tree::JungleStorybookTreeRenderHelper;
pub use kamakura_torch::KamakuraTorchRenderHelper;
pub use liams_conifer::LiamsConiferRenderHelper;
pub use moderate_lod_frond_crown::ModerateLodFrondCrownRenderHelper;
pub use northern_conifer::NorthernConiferRenderHelper;
pub use palm_bush::PalmBushRenderHelper;
pub use penmarch_torch::PenmarchTorchRenderHelper;
pub use rorys_head_trained::RorysHeadTrainedRenderHelper;
pub use sopes_banyan::SopesBanyanRenderHelper;
pub use spear_tuft::SpearTuftRenderHelper;
pub use storybook_tree::StorybookTreeRenderHelper;
pub use succulent_tuft::SucculentTuftRenderHelper;
pub use temperate_conifer::TemperateConiferRenderHelper;
pub use vase_tree::VaseTreeRenderHelper;
pub use waialea_palm::WaialeaPalmRenderHelper;
pub use weeping_tuft::WeepingTuftRenderHelper;

#[derive(Clone, clap::Args, Component)]
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
}

/// Wraps [`RenderHelper`] with square grove extent settings for grove [`RenderItem`] commands.
#[derive(Clone, clap::Args, Component)]
#[command(rename_all = "kebab-case")]
pub struct CellRenderHelper<T: clap::Args + Clone> {
	#[command(flatten)]
	pub render: RenderHelper<T>,

	#[arg(long, default_value_t = BraidGrassDefinition::DEFAULT_GROVE_EXTENT_XZ, help_heading = "Grove Extent")]
	pub grove_extent_xz: f32,
}

impl<T: clap::Args + Clone> CellRenderHelper<T> {
	pub fn render_transform(&self) -> Transform {
		self.render.render_transform()
	}

	pub fn res_2(&self) -> u8 {
		self.render.res_2
	}
}

#[derive(Clone, Subcommand, Component)]
#[command(rename_all = "kebab-case")]
pub enum Render {
	SopesBanyan(SopesBanyanRenderHelper),
	HonuBanyan(HonuBanyanRenderHelper),
	LiamsConifer(LiamsConiferRenderHelper),
	FriendsConifer(FriendsConiferRenderHelper),
	NorthernConifer(NorthernConiferRenderHelper),
	TemperateConifer(TemperateConiferRenderHelper),
	DatePalm(DatePalmRenderHelper),
	WaialeaPalm(WaialeaPalmRenderHelper),
	PalmBush(PalmBushRenderHelper),
	StorybookTree(StorybookTreeRenderHelper),
	PenmarchTorch(PenmarchTorchRenderHelper),
	KamakuraTorch(KamakuraTorchRenderHelper),
	RorysHeadTrained(RorysHeadTrainedRenderHelper),
	VaseTree(VaseTreeRenderHelper),
	BraidOakTree(BraidOakTreeRenderHelper),
	JungleStorybookTree(JungleStorybookTreeRenderHelper),
	SucculentTuft(SucculentTuftRenderHelper),
	BladeTuft(BladeTuftRenderHelper),
	BraidGrass(BraidGrassRenderHelper),
	SpearTuft(SpearTuftRenderHelper),
	BuddhaHandTuft(BuddhaHandTuftRenderHelper),
	WeepingTuft(WeepingTuftRenderHelper),
	JungleGrowth(JungleGrowthRenderHelper),
	HighBushShoots(HighBushShootsRenderHelper),
	CommonHighBush(CommonHighBushRenderHelper),
	FrondCrown(FrondCrownRenderHelper),
	ModerateLodFrondCrown(ModerateLodFrondCrownRenderHelper),
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
			Self::SopesBanyan(h) => RenderConfig {
				subject: RenderSubject::SopesBanyan(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::HonuBanyan(h) => RenderConfig {
				subject: RenderSubject::HonuBanyan(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::LiamsConifer(h) => RenderConfig {
				subject: RenderSubject::LiamsConifer(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::FriendsConifer(h) => RenderConfig {
				subject: RenderSubject::FriendsConifer(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::NorthernConifer(h) => RenderConfig {
				subject: RenderSubject::NorthernConifer(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::TemperateConifer(h) => RenderConfig {
				subject: RenderSubject::TemperateConifer(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::DatePalm(h) => RenderConfig {
				subject: RenderSubject::DatePalm(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::WaialeaPalm(h) => RenderConfig {
				subject: RenderSubject::WaialeaPalm(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::PalmBush(h) => RenderConfig {
				subject: RenderSubject::PalmBush(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::StorybookTree(h) => RenderConfig {
				subject: RenderSubject::StorybookTree(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::PenmarchTorch(h) => RenderConfig {
				subject: RenderSubject::PenmarchTorch(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::KamakuraTorch(h) => RenderConfig {
				subject: RenderSubject::KamakuraTorch(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::RorysHeadTrained(h) => RenderConfig {
				subject: RenderSubject::RorysHeadTrained(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::VaseTree(h) => RenderConfig {
				subject: RenderSubject::VaseTree(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::BraidOakTree(h) => RenderConfig {
				subject: RenderSubject::BraidOakTree(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::JungleStorybookTree(h) => RenderConfig {
				subject: RenderSubject::JungleStorybookTree(h.inner.clone()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::SucculentTuft(h) => RenderConfig {
				subject: RenderSubject::SucculentTuft(h.inner.clone().into()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::BladeTuft(h) => RenderConfig {
				subject: RenderSubject::BladeTuft(h.inner.clone().into()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::BraidGrass(h) => RenderConfig {
				subject: RenderSubject::BraidGrass(h.configured_braid_grass()),
				res_2: h.res_2(),
				transform: h.render_transform(),
			},
			Self::SpearTuft(h) => RenderConfig {
				subject: RenderSubject::SpearTuft(h.inner.clone().into()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::BuddhaHandTuft(h) => RenderConfig {
				subject: RenderSubject::BuddhaHandTuft(h.inner.clone().into()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::WeepingTuft(h) => RenderConfig {
				subject: RenderSubject::WeepingTuft(h.inner.clone().into()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::JungleGrowth(h) => RenderConfig {
				subject: RenderSubject::JungleGrowth(h.inner.clone().into()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::HighBushShoots(h) => RenderConfig {
				subject: RenderSubject::HighBushShoots(h.inner.clone().into()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::CommonHighBush(h) => RenderConfig {
				subject: RenderSubject::CommonHighBush(h.inner.clone().into()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::FrondCrown(h) => RenderConfig {
				subject: RenderSubject::FrondCrown(h.inner.clone().into()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
			Self::ModerateLodFrondCrown(h) => RenderConfig {
				subject: RenderSubject::ModerateLodFrondCrown(h.inner.clone().into()),
				res_2: h.res_2,
				transform: h.render_transform(),
			},
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
		assert_eq!(helper.inner.shape.spear_count, 20);
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
		assert!((helper.inner.shape.inner_ball_scale - 0.9).abs() < 1e-5);
		assert_eq!(helper.inner.shape.seed, 42);
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
		assert_eq!(helper.inner.shape.frond_count, 15);
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
	fn braid_grass_command_preserves_grove_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line(
			"render braid-grass --variant-weights 0.0,9.0,x,x,x --elevation 0.4 --grove-extent-xz 12.75 --cell-extent-xz 4.25,4.25",
		)
		.map_err(|e| anyhow::anyhow!("{e}"))?;
		let crate::commands::PlaygroundCommand::Render(Render::BraidGrass(helper)) = cmd else {
			anyhow::bail!("expected braid-grass render command");
		};
		assert!((helper.grove_extent_xz - 12.75).abs() < 1e-5);
		assert_eq!(helper.render.inner.grove.cell_extent_xz, Vec2::splat(4.25));
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
