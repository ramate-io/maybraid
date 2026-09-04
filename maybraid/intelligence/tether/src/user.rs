//! Installed tether brain. Presence means the duty is assigned; [`Self::enabled`] is the grant.

use bevy::prelude::*;
use movement_intelligence::MovementObjective;

use crate::memory::TetherMemory;
use crate::objective::TetherObjective;

const REMAINING_SLOP: f32 = 0.2;

/// Anchor that may be tethered or stalked. Looked up live at check time.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Tether;

/// Policy and progress bookkeeping. Does not store [`TetherMemory`].
#[derive(Component, Clone, Debug)]
pub struct TetherIntelligenceUser {
	pub objective: TetherObjective,
	/// Extra remaining metres that still count as satisfied (hysteresis).
	pub added_radius: f32,
	/// Remaining work ≤ this writes movement; beyond it writes a routing destination.
	pub horizon: f32,
	/// Higher-order grant. When false the brain does not write routing or movement.
	pub enabled: bool,
	pub stuck_timeout: f32,
	last_remaining: Option<f32>,
	last_applied: Option<TetherObjective>,
	stuck_seconds: f32,
	was_writing: bool,
}

impl TetherIntelligenceUser {
	pub fn new(objective: TetherObjective) -> Self {
		Self { objective, ..Self::defaults() }
	}

	fn defaults() -> Self {
		Self {
			objective: TetherObjective::Tether(Entity::PLACEHOLDER, 8.0),
			added_radius: 2.0,
			horizon: 28.0,
			enabled: true,
			stuck_timeout: 1.6,
			last_remaining: None,
			last_applied: None,
			stuck_seconds: 0.0,
			was_writing: false,
		}
	}

	pub fn with_added_radius(mut self, added_radius: f32) -> Self {
		self.added_radius = added_radius.max(0.0);
		self
	}

	pub fn with_horizon(mut self, horizon: f32) -> Self {
		self.horizon = horizon.max(0.0);
		self
	}

	pub fn with_enabled(mut self, enabled: bool) -> Self {
		self.enabled = enabled;
		self
	}

	pub fn with_stuck_timeout(mut self, stuck_timeout: f32) -> Self {
		self.stuck_timeout = stuck_timeout.max(0.0);
		self
	}

	/// Snapshot the subject and decide whether to write local movement, a route, or hold.
	///
	/// A retracted grant returns [`TetherAction::None`] even if this brain was
	/// writing. Hold pins position and would stomp flee or firearm movement.
	pub fn evaluate(
		&mut self,
		memory: &mut TetherMemory,
		from: Vec3,
		subject: Vec3,
		dt: f32,
		now: f32,
	) -> TetherAction {
		if !self.enabled {
			self.last_remaining = None;
			self.last_applied = None;
			self.stuck_seconds = 0.0;
			self.was_writing = false;
			return TetherAction::None;
		}

		if self.last_applied != Some(self.objective) {
			self.last_applied = Some(self.objective);
			self.last_remaining = None;
			self.stuck_seconds = 0.0;
			memory.satisfied = false;
		}

		let remaining = self.objective.remaining(from, subject);
		memory.subject = self.objective.subject();
		memory.remaining = remaining;
		memory.last_checked_at = now;
		memory.satisfied = sticky_satisfied(memory.satisfied, remaining, self.added_radius);

		if memory.satisfied {
			self.last_remaining = Some(remaining);
			self.stuck_seconds = 0.0;
			if self.was_writing {
				self.was_writing = false;
				return TetherAction::Hold;
			}
			return TetherAction::None;
		}

		let first = self.last_remaining.is_none();
		let previous = self.last_remaining.unwrap_or(remaining);
		let progressed = remaining + REMAINING_SLOP < previous;
		let regress = remaining > previous + REMAINING_SLOP;
		self.last_remaining = Some(remaining);

		if progressed && !first {
			self.stuck_seconds = 0.0;
			return TetherAction::None;
		}

		if !first && !regress {
			self.stuck_seconds += dt.max(0.0);
			if self.stuck_seconds < self.stuck_timeout {
				return TetherAction::None;
			}
		}

		self.stuck_seconds = 0.0;
		self.was_writing = true;
		if remaining <= self.horizon {
			TetherAction::Local(self.objective.movement_objective(from, subject))
		} else {
			TetherAction::Route(self.objective.route_point(from, subject))
		}
	}
}

impl Default for TetherIntelligenceUser {
	fn default() -> Self {
		Self::defaults()
	}
}

/// What [`TetherIntelligenceUser::evaluate`] wants applied this check.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TetherAction {
	None,
	Local(MovementObjective),
	Route(Vec3),
	/// Pin pose after remaining work became satisfied while the grant is on.
	/// Disable returns [`Self::None`] instead so other brains can write.
	Hold,
}

/// Insert the brain and a fresh [`TetherMemory`]. Uninstall by removing only the user.
pub fn install_tether(commands: &mut Commands, entity: Entity, user: TetherIntelligenceUser) {
	let memory = TetherMemory::from_objective(user.objective);
	commands.entity(entity).insert((user, memory));
}

fn sticky_satisfied(was: bool, remaining: f32, added_radius: f32) -> bool {
	if remaining <= 0.0 { true } else { was && remaining <= added_radius }
}
