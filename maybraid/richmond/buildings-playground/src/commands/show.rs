//! `/show` subcommand: preview a partition leaf or the Wizard's Tower.

use bevy::prelude::*;
use clap::Subcommand;

use crate::preview::{PreviewConfig, PreviewSubject};

#[derive(Clone, Subcommand)]
#[command(rename_all = "kebab-case")]
pub enum Show {
	/// Straight rough-stonework linear segment (`rough_stonework_001.glb`).
	Linear {
		#[command(flatten)]
		transform: ShowTransform,
	},
	/// 90° rough-stonework arc (`rough_stonework_90_001.glb`).
	Arc90 {
		#[command(flatten)]
		transform: ShowTransform,
	},
	/// 180° rough-stonework arc (`rough_stonework_180_001.glb`).
	Arc180 {
		#[command(flatten)]
		transform: ShowTransform,
	},
	/// 90° header rough-stonework (`rough_stonework_90_header_001.glb`).
	Header90 {
		#[command(flatten)]
		transform: ShowTransform,
	},
	/// Full Wizard's Tower (noise-derived floor count).
	WizardsTower {
		/// Unit noise sample in \[0, 1\] for floor count (10..=30).
		#[arg(long, default_value_t = 0.5)]
		noise: f32,
		#[command(flatten)]
		transform: ShowTransform,
	},
}

#[derive(Clone, clap::Args)]
#[command(rename_all = "kebab-case")]
pub struct ShowTransform {
	/// Translation `x,y,z` in world units.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z")]
	pub translate: Vec3,

	/// Euler rotation in degrees around X, then Y, then Z.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z")]
	pub rotate_euler: Vec3,

	/// Scale factors `x,y,z`.
	#[arg(long, default_value = "1,1,1", value_parser = parse_vec3_csv)]
	#[arg(value_name = "X,Y,Z")]
	pub scale: Vec3,
}

impl ShowTransform {
	pub fn transform(&self) -> Transform {
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

impl Show {
	pub fn react(self, commands: &mut Commands) {
		let (subject, transform) = match self {
			Self::Linear { transform } => (PreviewSubject::Linear, transform.transform()),
			Self::Arc90 { transform } => (PreviewSubject::Arc90, transform.transform()),
			Self::Arc180 { transform } => (PreviewSubject::Arc180, transform.transform()),
			Self::Header90 { transform } => (PreviewSubject::Header90, transform.transform()),
			Self::WizardsTower { noise, transform } => (
				PreviewSubject::WizardsTower { noise },
				transform.transform(),
			),
		};
		commands.insert_resource(PreviewConfig { subject, transform });
	}
}

fn parse_vec3_csv(s: &str) -> Result<Vec3, String> {
	let parts: Vec<_> = s.split(',').collect();
	if parts.len() != 3 {
		return Err(format!("expected x,y,z got {s:?}"));
	}
	let x: f32 = parts[0]
		.trim()
		.parse()
		.map_err(|e| format!("x: {e}"))?;
	let y: f32 = parts[1]
		.trim()
		.parse()
		.map_err(|e| format!("y: {e}"))?;
	let z: f32 = parts[2]
		.trim()
		.parse()
		.map_err(|e| format!("z: {e}"))?;
	Ok(Vec3::new(x, y, z))
}
