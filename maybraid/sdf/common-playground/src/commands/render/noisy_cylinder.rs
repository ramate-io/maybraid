//! Noisy-cylinder render subcommand types (`render noisy-cylinder …`).

pub mod plugin;

use bevy::prelude::*;
use clap::Args;

use super::RenderHelper;
use sdf_common::{NoiseParams, TaperedCylinder, UnitCylinderNoiseParams};

/// Inner clap args for noisy cylinder (cylinder + noise flattened).
#[derive(Debug, Clone, Args, Component)]
#[command(rename_all = "kebab-case")]
pub struct NoisyCylinderArgs {
	#[command(flatten)]
	pub cylinder: TaperedCylinder,
	/// Use [`UnitCylinderNoiseParams`] instead of the flattened noise flags (Perlin, amp 0.05, freq 5, 1 octave).
	#[arg(long, default_value_t = false)]
	pub suggested: bool,
	#[command(flatten)]
	pub noise: NoiseParams,
}

impl NoisyCylinderArgs {
	/// Noise actually applied after CLI parse (`--suggested` overrides flattened `noise`).
	pub fn resolved_noise(&self) -> NoiseParams {
		if self.suggested {
			UnitCylinderNoiseParams.into()
		} else {
			self.noise
		}
	}
}

pub type NoisyCylinderHelper = RenderHelper<NoisyCylinderArgs>;
