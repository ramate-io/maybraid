pub mod crook_cylinder;
pub mod noisy_cylinder;
pub mod plugin;
pub mod tapered_cylinder;
mod vec3_args;

use bevy::prelude::*;
use clap::Subcommand;
use sdf_common::NoisySurface;

use crate::preview::PreviewConfig;
use crate::primitive::PlaygroundPrimitive;

pub use crook_cylinder::CrookCylinderHelper;
pub use noisy_cylinder::{NoisyCylinderArgs, NoisyCylinderHelper};
pub use tapered_cylinder::TaperedCylinderHelper;

use vec3_args::parse_vec3_csv;

/// Shared chunk / transform flags for any geometry variant (flatten inner SDF args + optional noise).
#[derive(Debug, Clone, clap::Args, Component)]
#[command(rename_all = "kebab-case")]
pub struct RenderHelper<T: clap::Args> {
	#[command(flatten)]
	pub inner: T,

	#[arg(long, default_value_t = 5)]
	pub res_2: u8,

	/// Uniform scale factors `x,y,z` (e.g. `1.0,2.0,3.0`).
	#[arg(long, default_value = "1,1,1", value_parser = parse_vec3_csv)]
	pub scale: Vec3,

	/// Translation `x,y,z` in world units.
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv)]
	pub translate: Vec3,

	/// Euler rotation in **degrees** around X, then Y, then Z ([`EulerRot::XYZ`]).
	#[arg(long, default_value = "0,0,0", value_parser = parse_vec3_csv)]
	pub rotate_euler: Vec3,
}

impl<T: clap::Args> RenderHelper<T> {
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

#[derive(Debug, Clone, Subcommand, Component)]
pub enum Render {
	/// Smooth tapered cylinder (no procedural surface displacement).
	TaperedCylinder(TaperedCylinderHelper),
	/// Tapered cylinder with [`sdf_common::NoiseParams`] surface displacement.
	NoisyCylinder(NoisyCylinderHelper),
	/// Tapered segment with smooth sinusoidal centerline ([RFC-183 3.1.1.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/01-stick-and-stalk-components/02-crook-cylinder/README.md)).
	CrookCylinder(CrookCylinderHelper),
}

impl Render {
	/// Spawn a [`Render`] entity for systems that react to [`Added<Render>`](bevy::prelude::Added).
	pub fn react(self, commands: &mut Commands) {
		commands.spawn(self.clone());

		match self {
			Self::TaperedCylinder(render_helper) => commands.spawn(render_helper),
			Self::NoisyCylinder(render_helper) => commands.spawn(render_helper),
			Self::CrookCylinder(render_helper) => commands.spawn(render_helper),
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
				let noise = h.inner.resolved_noise();
				PreviewConfig {
					primitive: PlaygroundPrimitive::NoisyCylinder(NoisySurface::from_params(
						cyl, noise,
					)),
					res_2: h.res_2,
					transform: h.preview_transform(),
				}
			}
			Self::CrookCylinder(h) => PreviewConfig {
				primitive: PlaygroundPrimitive::CrookCylinder(h.inner),
				res_2: h.res_2,
				transform: h.preview_transform(),
			},
		}
	}
}
