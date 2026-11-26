//! Authored identities and contextual load failures for tooling.
use bevy::prelude::*;

/// One realized layer host. Indices refer to the flattened definition, starting at zero.
#[derive(Component, Clone, Debug)]
pub struct VfxLayerInfo {
	pub index: usize,
	pub label: String,
}

/// Emitted before a failed instance is cancelled. Includes dependency loader errors.
#[derive(Message, Clone, Debug)]
pub struct VfxLoadFailure {
	pub root: Entity,
	pub effect: String,
	pub layer: String,
	pub path: String,
	pub error: String,
}

impl std::fmt::Display for VfxLoadFailure {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(f, "{} / {} / {}: {}", self.effect, self.layer, self.path, self.error)
	}
}
