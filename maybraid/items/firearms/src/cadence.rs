//! Trigger cadence sitting beside [`Weapon`] interval.

use bevy::prelude::*;

/// How a held trigger becomes shots. Interval still lives on [`crate::Weapon`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Cadence {
	#[default]
	Auto,
	Semi,
	Burst,
	Gated,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TriggerIntent {
	pub held: bool,
	pub rising: bool,
}

/// Per-gun burst / semi state. Missing means auto.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct FireControl {
	pub cadence: Cadence,
	pub burst_rounds: u8,
	pub burst_left: u8,
	pub trigger_was: bool,
}

impl Default for FireControl {
	fn default() -> Self {
		Self::auto()
	}
}

impl FireControl {
	pub fn auto() -> Self {
		Self { cadence: Cadence::Auto, burst_rounds: 0, burst_left: 0, trigger_was: false }
	}

	pub fn semi() -> Self {
		Self { cadence: Cadence::Semi, burst_rounds: 0, burst_left: 0, trigger_was: false }
	}

	pub fn burst(rounds: u8) -> Self {
		let rounds = rounds.max(1);
		Self {
			cadence: Cadence::Burst,
			burst_rounds: rounds,
			burst_left: rounds,
			trigger_was: false,
		}
	}

	pub fn gated() -> Self {
		Self { cadence: Cadence::Gated, burst_rounds: 0, burst_left: 0, trigger_was: false }
	}

	pub fn poll(&mut self, held: bool) -> TriggerIntent {
		let rising = held && !self.trigger_was;
		if !held {
			self.burst_left = self.burst_rounds;
		}
		self.trigger_was = held;
		TriggerIntent { held, rising }
	}

	pub fn allows(&self, intent: TriggerIntent) -> bool {
		match self.cadence {
			Cadence::Auto | Cadence::Gated => intent.held,
			Cadence::Semi => intent.rising,
			Cadence::Burst => intent.held && self.burst_left > 0,
		}
	}

	pub fn note_shot(&mut self) {
		if self.cadence == Cadence::Burst {
			self.burst_left = self.burst_left.saturating_sub(1);
		}
	}
}

/// Kick applied to the shooter's look after a shot.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct WeaponRecoil(pub f32);

#[derive(Message, Clone, Copy, Debug)]
pub struct WeaponFired {
	pub shooter: Entity,
	pub recoil: f32,
}

pub fn trigger_allows_fire(control: Option<&mut FireControl>, manual: bool, held: bool) -> bool {
	if !manual {
		return true;
	}
	let Some(control) = control else {
		return held;
	};
	let intent = control.poll(held);
	control.allows(intent)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn auto_fires_while_held() {
		let mut control = FireControl::auto();
		let intent = control.poll(true);
		assert!(control.allows(intent));
		let intent = control.poll(false);
		assert!(!control.allows(intent));
	}

	#[test]
	fn semi_fires_on_rising_edge_only() {
		let mut control = FireControl::semi();
		let intent = control.poll(true);
		assert!(control.allows(intent));
		let intent = control.poll(true);
		assert!(!control.allows(intent));
		let intent = control.poll(false);
		assert!(!control.allows(intent));
		let intent = control.poll(true);
		assert!(control.allows(intent));
	}

	#[test]
	fn burst_stops_until_release() {
		let mut control = FireControl::burst(2);
		let intent = control.poll(true);
		assert!(control.allows(intent));
		control.note_shot();
		let intent = control.poll(true);
		assert!(control.allows(intent));
		control.note_shot();
		let intent = control.poll(true);
		assert!(!control.allows(intent));
		let intent = control.poll(false);
		assert!(!control.allows(intent));
		let intent = control.poll(true);
		assert!(control.allows(intent));
	}
}
