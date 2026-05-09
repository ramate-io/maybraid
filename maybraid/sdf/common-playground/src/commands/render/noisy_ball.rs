//! Noisy ball render subcommand (`render noisy-ball …`).

pub mod plugin;

use bevy::prelude::*;
use clap::Args;

use super::RenderHelper;
use sdf_common::{Ball, NoiseParams, UnitBallNoiseParams};

/// Inner clap args for noisy ball (sphere + noise flattened).
#[derive(Debug, Clone, Args, Component)]
#[command(rename_all = "kebab-case")]
pub struct NoisyBallArgs {
	#[command(flatten)]
	pub ball: Ball,
	/// Use [`UnitBallNoiseParams`] instead of the flattened noise flags (Perlin, amp 0.05, freq 5, 1 octave).
	#[arg(long, default_value_t = false)]
	pub suggested: bool,
	#[command(flatten)]
	pub noise: NoiseParams,
}

impl NoisyBallArgs {
	/// Noise actually applied after CLI parse (`--suggested` overrides flattened `noise`).
	pub fn resolved_noise(&self) -> NoiseParams {
		if self.suggested {
			UnitBallNoiseParams.into()
		} else {
			self.noise
		}
	}
}

pub type NoisyBallHelper = RenderHelper<NoisyBallArgs>;
