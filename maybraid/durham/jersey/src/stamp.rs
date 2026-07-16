//! Shared stamp output bundle.

use crate::modulation::JerseyModulation;
use bevy_math::Vec2;

/// Non-geometric facts for later gameplay / hydrology hand-off ([RFC-105 §3.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain)).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StampSemantics {
	/// Tags such as `bank`, `arroyo`, `spillway_ready`.
	pub tags: Vec<&'static str>,
}

impl StampSemantics {
	pub fn with_tag(mut self, tag: &'static str) -> Self {
		self.tags.push(tag);
		self
	}
}

/// Ordered modulations plus optional spine geometry from one stamp construction.
#[derive(Debug, Clone)]
pub struct StampSet {
	pub modulations: Vec<JerseyModulation>,
	pub spine: Vec<Vec2>,
	pub semantics: StampSemantics,
}

impl StampSet {
	pub fn empty() -> Self {
		Self {
			modulations: Vec::new(),
			spine: Vec::new(),
			semantics: StampSemantics::default(),
		}
	}

	pub fn apply_elevation(&self, mut elevation: f32, x: f32, z: f32) -> f32 {
		for m in &self.modulations {
			elevation = m.modify_elevation(elevation, x, z);
		}
		elevation
	}
}
