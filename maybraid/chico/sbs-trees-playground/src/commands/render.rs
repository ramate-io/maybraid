pub mod blade_tuft;
pub mod liams_conifer;
pub mod plugin;
pub mod sopes_banyan;
pub mod succulent_tuft;
pub mod weeping_tuft;

use bevy::prelude::*;
use clap::Subcommand;

use crate::render::{RenderConfig, RenderSubject};
pub use blade_tuft::BladeTuftRenderHelper;
pub use liams_conifer::LiamsConiferRenderHelper;
pub use sopes_banyan::SopesBanyanRenderHelper;
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
	WeepingTuft(WeepingTuftRenderHelper),
}

impl Render {
	pub fn react(self, commands: &mut Commands) {
		commands.spawn(self.clone());
		match self {
			Self::SopesBanyan(h) => {
				commands.spawn(h);
			}
			Self::LiamsConifer(h) => {
				commands.spawn(h);
			}
			Self::SucculentTuft(h) => {
				commands.spawn(h);
			}
			Self::BladeTuft(h) => {
				commands.spawn(h);
			}
			Self::WeepingTuft(h) => {
				commands.spawn(h);
			}
		}
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
			Self::WeepingTuft(h) => RenderConfig {
				subject: RenderSubject::WeepingTuft(h.inner.clone().into()),
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
