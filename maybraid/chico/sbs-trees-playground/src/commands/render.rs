pub mod blade_tuft;
pub mod buddha_hand_tuft;
pub mod jungle_growth;
pub mod liams_conifer;
pub mod plugin;

pub use plugin::RenderCommandsPlugin;
pub mod sopes_banyan;
pub mod spear_tuft;
pub mod succulent_tuft;
pub mod weeping_tuft;

use bevy::prelude::*;
use clap::Subcommand;

use crate::render::{RenderConfig, RenderSubject};
pub use blade_tuft::BladeTuftRenderHelper;
pub use buddha_hand_tuft::BuddhaHandTuftRenderHelper;
pub use jungle_growth::JungleGrowthRenderHelper;
pub use liams_conifer::LiamsConiferRenderHelper;
pub use sopes_banyan::SopesBanyanRenderHelper;
pub use spear_tuft::SpearTuftRenderHelper;
pub use succulent_tuft::SucculentTuftRenderHelper;
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

#[derive(Clone, Subcommand, Component)]
#[command(rename_all = "kebab-case")]
pub enum Render {
	SopesBanyan(SopesBanyanRenderHelper),
	LiamsConifer(LiamsConiferRenderHelper),
	SucculentTuft(SucculentTuftRenderHelper),
	BladeTuft(BladeTuftRenderHelper),
	SpearTuft(SpearTuftRenderHelper),
	BuddhaHandTuft(BuddhaHandTuftRenderHelper),
	WeepingTuft(WeepingTuftRenderHelper),
	JungleGrowth(JungleGrowthRenderHelper),
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
			Self::LiamsConifer(h) => RenderConfig {
				subject: RenderSubject::LiamsConifer(h.inner.clone()),
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

	#[test]
	fn spear_tuft_command_preserves_shape_params() -> Result<()> {
		let cmd = crate::commands::PlaygroundCommand::parse_line("render spear-tuft --spear-count 20")
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
}
