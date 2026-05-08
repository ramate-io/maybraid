//! Noisy-cylinder render subcommand types (`render noisy-cylinder …`).

pub mod plugin;

use bevy::prelude::*;
use clap::Args;

use super::RenderHelper;
use sdf_common::{NoiseParams, TaperedCylinder};

/// Inner clap args for noisy cylinder (cylinder + noise flattened).
#[derive(Debug, Clone, Args, Component)]
#[command(rename_all = "kebab-case")]
pub struct NoisyCylinderArgs {
	#[command(flatten)]
	pub cylinder: TaperedCylinder,
	#[command(flatten)]
	pub noise: NoiseParams,
}

pub type NoisyCylinderHelper = RenderHelper<NoisyCylinderArgs>;
