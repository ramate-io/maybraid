//! Victim class retained after affiliation components are stripped.

use bevy::prelude::Component;

use crate::affiliation::BikanwardaVictimClass;

/// Snapshotted on the first qualifying combat hit so Award can classify retired victims.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BikanwardaVictimSnapshot {
	pub class: BikanwardaVictimClass,
}
