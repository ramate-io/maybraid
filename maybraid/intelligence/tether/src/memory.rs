//! Anchor marker and last observation. Memory is not uninstalled with the brain.

use bevy::prelude::*;

use crate::objective::TetherObjective;

/// Last tether observation. Higher-order systems read this while the brain is off.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct TetherMemory {
	pub subject: Entity,
	pub satisfied: bool,
	pub remaining: f32,
	pub last_checked_at: f32,
}

impl TetherMemory {
	pub fn new(subject: Entity) -> Self {
		Self { subject, satisfied: false, remaining: f32::MAX, last_checked_at: 0.0 }
	}

	pub fn from_objective(objective: TetherObjective) -> Self {
		Self::new(objective.subject())
	}
}
