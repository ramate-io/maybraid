//! Launch-time switch for the capability-based woody renderer.

use std::sync::OnceLock;

use bevy::prelude::*;

const ENV_PACKED_GROVES: &str = "MAYBRAID_PACKED_GROVES";
const ENV_BUDGET_MB: &str = "MAYBRAID_PACKED_GROVE_BUDGET_MB";

const DEFAULT_BUDGET_MB: u32 = 256;

/// Which grove family the custom path migrates. Unset / unknown stays on Mesh3d.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Resource)]
pub enum PackMode {
	#[default]
	Off,
	Orchard,
	Compatible,
}

impl PackMode {
	/// Cached process-wide mode. Launch-time only; tests construct [`Self`] directly.
	pub fn current() -> Self {
		static MODE: OnceLock<PackMode> = OnceLock::new();
		*MODE.get_or_init(Self::from_env)
	}

	pub fn from_env() -> Self {
		match std::env::var(ENV_PACKED_GROVES) {
			Ok(value) => Self::parse(&value),
			Err(_) => Self::Off,
		}
	}

	pub fn parse(value: &str) -> Self {
		match value.trim().to_ascii_lowercase().as_str() {
			"orchard" => Self::Orchard,
			"1" | "on" | "true" | "compatible" | "woody" => Self::Compatible,
			_ => Self::Off,
		}
	}

	pub fn accepts(self, tile: &crate::ForestGroveTile) -> bool {
		match self {
			Self::Off => false,
			Self::Orchard => tile.as_orchard().is_some(),
			Self::Compatible => true,
		}
	}

	pub fn is_off(self) -> bool {
		matches!(self, Self::Off)
	}

	pub fn budget_bytes_from_env() -> u64 {
		let mb = std::env::var(ENV_BUDGET_MB)
			.ok()
			.and_then(|value| value.trim().parse::<u32>().ok())
			.unwrap_or(DEFAULT_BUDGET_MB)
			.max(1);
		u64::from(mb) * 1024 * 1024
	}
}
