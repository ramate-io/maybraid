pub mod liams_conifer;
pub mod plugin;
pub mod sopes_banyan;

use bevy::prelude::*;
use clap::Subcommand;

use crate::preview::{PreviewConfig, PreviewTree};
pub use liams_conifer::LiamsConiferRenderHelper;
pub use sopes_banyan::SopesBanyanRenderHelper;

#[derive(Clone, clap::Args, Component)]
#[command(rename_all = "kebab-case")]
pub struct RenderHelper<T: clap::Args + Clone> {
	#[command(flatten)]
	pub inner: T,

	/// Preview cascade resolution exponent.
	#[arg(long, default_value_t = 5)]
	#[arg(help_heading = "Preview")]
	pub res_2: u8,

	/// Scale factors `x,y,z`.
	#[arg(long, default_value = "1,1,1", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z", help_heading = "Preview Transform")]
	pub scale: Vec3,

	/// Translation `x,y,z` in world units.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z", help_heading = "Preview Transform")]
	pub translate: Vec3,

	/// Euler rotation in degrees around X, then Y, then Z.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z", help_heading = "Preview Transform")]
	pub rotate_euler: Vec3,
}

impl<T: clap::Args + Clone> RenderHelper<T> {
	pub fn preview_transform(&self) -> Transform {
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
		}
	}

	pub fn into_preview_config(&self) -> PreviewConfig {
		match self {
			Self::SopesBanyan(h) => PreviewConfig {
				tree: PreviewTree::SopesBanyan(h.inner.clone()),
				res_2: h.res_2,
				transform: h.preview_transform(),
			},
			Self::LiamsConifer(h) => PreviewConfig {
				tree: PreviewTree::LiamsConifer(h.inner.clone()),
				res_2: h.res_2,
				transform: h.preview_transform(),
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
