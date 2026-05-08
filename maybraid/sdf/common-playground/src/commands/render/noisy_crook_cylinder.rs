//! Noisy crook-cylinder render subcommand (`render noisy-crook-cylinder …`).

pub mod plugin;

use bevy::prelude::*;
use clap::Args;

use super::RenderHelper;
use sdf_common::{CrookCylinder, NoiseParams, UnitCylinderNoiseParams};

/// Inner clap args for noisy crook (crook + noise flattened).
#[derive(Debug, Clone, Args, Component)]
#[command(rename_all = "kebab-case")]
pub struct NoisyCrookCylinderArgs {
	#[command(flatten)]
	pub crook: CrookCylinder,
	/// Use [`UnitCylinderNoiseParams`] instead of the flattened noise flags (Perlin, amp 0.05, freq 5, 1 octave).
	#[arg(long, default_value_t = false)]
	pub suggested: bool,
	#[command(flatten)]
	pub noise: NoiseParams,
}

impl NoisyCrookCylinderArgs {
	/// Noise actually applied after CLI parse (`--suggested` overrides flattened `noise`).
	pub fn resolved_noise(&self) -> NoiseParams {
		if self.suggested {
			UnitCylinderNoiseParams.into()
		} else {
			self.noise
		}
	}
}

pub type NoisyCrookCylinderHelper = RenderHelper<NoisyCrookCylinderArgs>;
