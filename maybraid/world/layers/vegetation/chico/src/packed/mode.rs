//! Launch-time switch for the packed orchard renderer.

use std::sync::OnceLock;

use bevy::prelude::*;

use crate::kind::LayeringKind;

const ENV_PACKED_GROVES: &str = "MAYBRAID_PACKED_GROVES";
const ENV_EMPHASIS: &str = "MAYBRAID_PACKED_GROVE_EMPHASIS";
const ENV_BUDGET_MB: &str = "MAYBRAID_PACKED_GROVE_BUDGET_MB";
const ENV_METRICS: &str = "MAYBRAID_PACKED_GROVE_METRICS";

const DEFAULT_BUDGET_MB: u32 = 256;

/// Which grove family the custom path migrates. Unset / unknown stays on Mesh3d.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Resource)]
pub enum PackMode {
	#[default]
	Off,
	Orchard,
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
			"1" | "on" | "true" | "orchard" => Self::Orchard,
			_ => Self::Off,
		}
	}

	pub fn packs_orchard(self) -> bool {
		matches!(self, Self::Orchard)
	}

	pub fn is_off(self) -> bool {
		matches!(self, Self::Off)
	}

	/// Optional Discovery layering pin (`ag-town`).
	pub fn emphasis_from_env() -> Option<LayeringKind> {
		let value = std::env::var(ENV_EMPHASIS).ok()?;
		LayeringKind::from_kebab(value.trim())
	}

	pub fn budget_bytes_from_env() -> u64 {
		let mb = std::env::var(ENV_BUDGET_MB)
			.ok()
			.and_then(|value| value.trim().parse::<u32>().ok())
			.unwrap_or(DEFAULT_BUDGET_MB)
			.max(1);
		u64::from(mb) * 1024 * 1024
	}

	pub fn metrics_from_env() -> bool {
		matches!(
			std::env::var(ENV_METRICS).as_deref(),
			Ok("1" | "on" | "true" | "TRUE")
		)
	}
}
