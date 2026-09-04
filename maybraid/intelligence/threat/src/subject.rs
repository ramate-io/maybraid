use bevy::prelude::*;

use crate::ThreatId;

/// Stable identity and semantic salience for a locally discoverable threat candidate.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct ThreatSubject {
	pub id: ThreatId,
	pub salience: f32,
}

impl ThreatSubject {
	pub fn new(id: ThreatId) -> Self {
		Self { id, salience: 1.0 }
	}

	pub fn with_salience(mut self, salience: f32) -> Self {
		if salience.is_finite() {
			self.salience = salience.max(0.0);
		}
		self
	}
}
