use bevy::prelude::*;
use clap::{Args, Subcommand};
use sdf_common::{NoiseParams, NoisySurface, TaperedCylinder};

use crate::preview::PreviewConfig;
use crate::primitive::PlaygroundPrimitive;

/// Shared chunk / transform flags for any geometry variant (flatten inner SDF args + optional noise).
#[derive(Debug, Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct RenderHelper<T: Args> {
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

impl<T: Args> RenderHelper<T> {
	pub fn preview_transform(&self) -> Transform {
		Transform::from_translation(Vec3::new(self.translate_x, self.translate_y, self.translate_z))
			.with_scale(Vec3::new(self.scale_x, self.scale_y, self.scale_z))
	}
}

/// Tapered trunk/cylinder plus noise params (combinator lives in the render helper’s `inner`).
#[derive(Debug, Clone, Args)]
#[command(rename_all = "kebab-case")]
pub struct NoisyCylinderArgs {
	#[command(flatten)]
	pub cylinder: TaperedCylinder,
	#[command(flatten)]
	pub noise: NoiseParams,
}

#[derive(Debug, Clone, Subcommand, Component)]
pub enum Render {
	/// Smooth tapered cylinder (no procedural surface displacement).
	TaperedCylinder(RenderHelper<TaperedCylinder>),
	/// Tapered cylinder with [`NoiseParams`] surface displacement.
	NoisyCylinder(RenderHelper<NoisyCylinderArgs>),
}

impl Render {
	/// Spawn a [`Render`] entity for systems that react to [`Added<Render>`](bevy::prelude::Added).
	pub fn react(self, commands: &mut Commands) {
		commands.spawn(self);
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
