pub mod noisy_cylinder;
pub mod plugin;
pub mod tapered_cylinder;

use bevy::prelude::*;
use clap::Subcommand;
use sdf_common::NoisySurface;

use crate::preview::PreviewConfig;
use crate::primitive::PlaygroundPrimitive;

pub use noisy_cylinder::{NoisyCylinderArgs, NoisyCylinderHelper};
pub use tapered_cylinder::TaperedCylinderHelper;

/// Shared chunk / transform flags for any geometry variant (flatten inner SDF args + optional noise).
#[derive(Debug, Clone, clap::Args, Component)]
#[command(rename_all = "kebab-case")]
pub struct RenderHelper<T: clap::Args> {
	#[command(flatten)]
	pub inner: T,

	#[arg(long, default_value_t = 4)]
	pub res_2: u8,

	#[arg(long, default_value_t = 1.0)]
	pub scale_x: f32,

	#[arg(long, default_value_t = 1.0)]
	pub scale_y: f32,

	#[arg(long, default_value_t = 1.0)]
	pub scale_z: f32,

	#[arg(long, default_value_t = 0.0)]
	pub translate_x: f32,

	#[arg(long, default_value_t = 0.0)]
	pub translate_y: f32,

	#[arg(long, default_value_t = 0.0)]
	pub translate_z: f32,
}

impl<T: clap::Args> RenderHelper<T> {
	pub fn preview_transform(&self) -> Transform {
		Transform::from_translation(Vec3::new(self.translate_x, self.translate_y, self.translate_z))
			.with_scale(Vec3::new(self.scale_x, self.scale_y, self.scale_z))
	}
}

#[derive(Debug, Clone, Subcommand, Component)]
pub enum Render {
	/// Smooth tapered cylinder (no procedural surface displacement).
	TaperedCylinder(TaperedCylinderHelper),
	/// Tapered cylinder with [`sdf_common::NoiseParams`] surface displacement.
	NoisyCylinder(NoisyCylinderHelper),
}

impl Render {
	/// Spawn a [`Render`] entity for systems that react to [`Added<Render>`](bevy::prelude::Added).
	pub fn react(self, commands: &mut Commands) {
		commands.spawn(self.clone());

		match self {
			Self::TaperedCylinder(render_helper) => commands.spawn(render_helper),
			Self::NoisyCylinder(render_helper) => commands.spawn(render_helper),
		};
	}

	pub fn into_preview_config(&self) -> PreviewConfig {
		match self {
			Self::TaperedCylinder(h) => PreviewConfig {
				primitive: PlaygroundPrimitive::TaperedCylinder(h.inner),
				res_2: h.res_2,
				transform: h.preview_transform(),
			},
			Self::NoisyCylinder(h) => {
				let cyl = h.inner.cylinder;
				let noise = h.inner.noise;
				PreviewConfig {
					primitive: PlaygroundPrimitive::NoisyCylinder(NoisySurface::from_params(cyl, noise)),
					res_2: h.res_2,
					transform: h.preview_transform(),
				}
			}
		}
	}
}
