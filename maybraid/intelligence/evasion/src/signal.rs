/// Exclusive movement actuator selected from ranked assailant knowledge.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EvasionActuator {
	#[default]
	Idle,
	Flee,
	Hide,
}

/// Ranked threat plus the actuator hide and flee should consume.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EvasionSignal {
	pub actuator: EvasionActuator,
	pub threat: Option<bevy::prelude::Entity>,
}

impl EvasionSignal {
	pub fn idle() -> Self {
		Self { actuator: EvasionActuator::Idle, threat: None }
	}

	pub fn is_idle(self) -> bool {
		self.actuator == EvasionActuator::Idle
	}

	pub fn is_flee(self) -> bool {
		self.actuator == EvasionActuator::Flee
	}

	pub fn is_hide(self) -> bool {
		self.actuator == EvasionActuator::Hide
	}
}
