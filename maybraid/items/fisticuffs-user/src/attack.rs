//! Input-facing attack state. v1 is two jabs; a combo graph can replace the
//! lookup from [`AttackSpec`] without changing consumers.

use character_animations::animations::HOLD_END;
use character_rigs::Side;
use characters::JAB_DURATION_SECS;
use maybraid_character_controller::CharacterIntent;

use crate::aim::JabAim;

/// Gameplay startup ends during the authored snap (`EXTEND_END = 0.42`).
///
/// Active begins at ~0.13 s on a [`JAB_DURATION_SECS`] jab so the hit window
/// sits on the incoming fist rather than waiting for the visual hold.
pub const JAB_STARTUP_END: f32 = 0.32;

/// Clip-progress windows and wall-clock length for one authored attack.
///
/// The `Jab` clip envelopes are unchanged (`BACKSWING_END = 0.18`,
/// `EXTEND_END = 0.42`, `HOLD_END = 0.52`). Playback is [`JAB_DURATION_SECS`]
/// (`0.42 s`) so a jab is combo-snappy:
///
/// | Phase | Clip progress | Seconds |
/// |---|---|---|
/// | Startup | `0.00 .. 0.32` ([`JAB_STARTUP_END`]) | 0.134 |
/// | Active | `0.32 .. 0.52` (`HOLD_END`) | 0.084 |
/// | Recovery | `0.52 .. 1.00` | 0.202 |
///
/// Aim is latched when the jab **starts** ([`AttackState::just_started`]), not
/// at Active. See [`crate::aim`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AttackTiming {
	pub startup_end: f32,
	pub active_end: f32,
	pub duration_secs: f32,
}

impl AttackTiming {
	pub const JAB: Self = Self {
		startup_end: JAB_STARTUP_END,
		active_end: HOLD_END,
		duration_secs: JAB_DURATION_SECS,
	};

	pub fn progress(self, elapsed: f32) -> f32 {
		if self.duration_secs <= 0.0 {
			return 1.0;
		}
		(elapsed / self.duration_secs).clamp(0.0, 1.0)
	}

	pub fn phase(self, elapsed: f32) -> Option<AttackPhase> {
		let progress = self.progress(elapsed);
		if elapsed >= self.duration_secs {
			None
		} else if progress < self.startup_end {
			Some(AttackPhase::Startup)
		} else if progress < self.active_end {
			Some(AttackPhase::Active)
		} else {
			Some(AttackPhase::Recovery)
		}
	}
}

/// Discriminant for the future combo graph. Only jab is implemented.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttackKind {
	Jab,
}

impl AttackKind {
	pub fn timing(self) -> AttackTiming {
		match self {
			Self::Jab => AttackTiming::JAB,
		}
	}
}

/// One node the combo graph will later select.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AttackSpec {
	pub kind: AttackKind,
	pub side: Side,
}

impl AttackSpec {
	pub fn jab(side: Side) -> Self {
		Self { kind: AttackKind::Jab, side }
	}

	pub fn from_intent(intent: CharacterIntent) -> Option<Self> {
		match intent {
			CharacterIntent::AttackLeft => Some(Self::jab(Side::Left)),
			CharacterIntent::AttackRight => Some(Self::jab(Side::Right)),
			_ => None,
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AttackPhase {
	Startup,
	Active,
	Recovery,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ActiveAttack {
	spec: AttackSpec,
	elapsed: f32,
	aim: JabAim,
}

impl ActiveAttack {
	fn phase(self) -> Option<AttackPhase> {
		self.spec.kind.timing().phase(self.elapsed)
	}

	fn progress(self) -> f32 {
		self.spec.kind.timing().progress(self.elapsed)
	}
}

/// One-slot attack machine. Queue during startup/active; cancel into the next
/// attack during recovery. A combo graph can consume [`Self::queued`] earlier.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AttackState {
	current: Option<ActiveAttack>,
	queued: Option<AttackSpec>,
	just_started: bool,
	just_entered_active: bool,
}

impl AttackState {
	pub fn request(&mut self, spec: AttackSpec) {
		match self.phase() {
			None | Some(AttackPhase::Recovery) => self.start(spec),
			Some(AttackPhase::Startup | AttackPhase::Active) => self.queued = Some(spec),
		}
	}

	pub fn tick(&mut self, dt: f32) {
		self.just_started = false;
		self.just_entered_active = false;
		let Some(current) = self.current.as_mut() else {
			if let Some(queued) = self.queued.take() {
				self.start(queued);
			}
			return;
		};
		let previous = current.phase();
		current.elapsed += dt.max(0.0);
		let next = current.phase();
		if previous != Some(AttackPhase::Active) && next == Some(AttackPhase::Active) {
			self.just_entered_active = true;
		}
		if next.is_none() {
			self.current = None;
			if let Some(queued) = self.queued.take() {
				self.start(queued);
			}
		}
	}

	pub fn cancel(&mut self) {
		*self = Self::default();
	}

	pub fn is_busy(&self) -> bool {
		self.current.is_some()
	}

	pub fn phase(&self) -> Option<AttackPhase> {
		self.current.and_then(ActiveAttack::phase)
	}

	pub fn side(&self) -> Option<Side> {
		self.current.map(|attack| attack.spec.side)
	}

	pub fn spec(&self) -> Option<AttackSpec> {
		self.current.map(|attack| attack.spec)
	}

	pub fn clip_progress(&self) -> Option<f32> {
		self.current.map(ActiveAttack::progress)
	}

	pub fn queued(&self) -> Option<AttackSpec> {
		self.queued
	}

	pub fn just_started(&self) -> bool {
		self.just_started
	}

	pub fn just_entered_active(&self) -> bool {
		self.just_entered_active
	}

	/// Quantized aim latched for the current jab, if any.
	pub fn aim(&self) -> Option<JabAim> {
		self.current.map(|attack| attack.aim)
	}

	/// Store look-derived aim only while [`Self::just_started`] is set.
	///
	/// Mid-punch look changes are ignored so the fist does not wobble.
	pub fn latch_aim(&mut self, aim: JabAim) {
		if !self.just_started {
			return;
		}
		if let Some(current) = self.current.as_mut() {
			current.aim = aim;
		}
	}

	fn start(&mut self, spec: AttackSpec) {
		self.current = Some(ActiveAttack { spec, elapsed: 0.0, aim: JabAim::default() });
		self.queued = None;
		self.just_started = true;
		self.just_entered_active = false;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::aim::{PITCH_MAX, PITCH_MIN, YAW_LIMIT};

	fn jab_dt(progress: f32) -> f32 {
		progress * AttackTiming::JAB.duration_secs
	}

	#[test]
	fn left_and_right_intents_select_side() {
		assert_eq!(
			AttackSpec::from_intent(CharacterIntent::AttackLeft).map(|spec| spec.side),
			Some(Side::Left)
		);
		assert_eq!(
			AttackSpec::from_intent(CharacterIntent::AttackRight).map(|spec| spec.side),
			Some(Side::Right)
		);
		assert!(AttackSpec::from_intent(CharacterIntent::Jump).is_none());
	}

	#[test]
	fn idle_press_starts_immediately() {
		let mut state = AttackState::default();
		state.request(AttackSpec::jab(Side::Left));
		assert!(state.just_started());
		assert_eq!(state.side(), Some(Side::Left));
		assert_eq!(state.phase(), Some(AttackPhase::Startup));
	}

	#[test]
	fn jab_is_snappy_for_combos() {
		assert!((AttackTiming::JAB.duration_secs - 0.42).abs() < 1e-5);
		let startup_secs = AttackTiming::JAB.startup_end * AttackTiming::JAB.duration_secs;
		assert!(startup_secs > 0.10 && startup_secs < 0.15, "startup {startup_secs}");
		assert!(AttackTiming::JAB.duration_secs > 0.35 && AttackTiming::JAB.duration_secs < 0.50);
	}

	#[test]
	fn timing_windows_match_jab_clip() {
		let mut state = AttackState::default();
		state.request(AttackSpec::jab(Side::Right));
		state.tick(0.0);
		assert_eq!(state.phase(), Some(AttackPhase::Startup));
		assert!((state.clip_progress().unwrap_or(1.0) - 0.0).abs() < 1e-5);

		state.tick(jab_dt(JAB_STARTUP_END) - 1e-4);
		assert_eq!(state.phase(), Some(AttackPhase::Startup));

		state.tick(2e-4);
		assert_eq!(state.phase(), Some(AttackPhase::Active));
		assert!(state.just_entered_active());

		state.tick(jab_dt(HOLD_END - JAB_STARTUP_END));
		assert_eq!(state.phase(), Some(AttackPhase::Recovery));

		state.tick(jab_dt(1.0 - HOLD_END) + 1e-3);
		assert!(!state.is_busy());
	}

	#[test]
	fn overlapping_press_queues_until_recovery() {
		let mut state = AttackState::default();
		state.request(AttackSpec::jab(Side::Left));
		state.tick(jab_dt(0.1));
		state.request(AttackSpec::jab(Side::Right));
		assert_eq!(state.side(), Some(Side::Left));
		assert_eq!(state.queued(), Some(AttackSpec::jab(Side::Right)));
		assert!(!state.just_started());

		state.tick(jab_dt(0.2));
		state.request(AttackSpec::jab(Side::Left));
		assert_eq!(state.queued(), Some(AttackSpec::jab(Side::Left)));
		assert_eq!(state.side(), Some(Side::Left));
	}

	#[test]
	fn recovery_press_cancels_into_the_next_jab() {
		let mut state = AttackState::default();
		state.request(AttackSpec::jab(Side::Left));
		state.tick(jab_dt(HOLD_END + 0.01));
		assert_eq!(state.phase(), Some(AttackPhase::Recovery));
		state.request(AttackSpec::jab(Side::Right));
		assert!(state.just_started());
		assert_eq!(state.side(), Some(Side::Right));
		assert_eq!(state.phase(), Some(AttackPhase::Startup));
		assert!(state.queued().is_none());
	}

	#[test]
	fn queued_jab_starts_when_the_current_one_ends() {
		let mut state = AttackState::default();
		state.request(AttackSpec::jab(Side::Left));
		state.tick(jab_dt(0.2));
		state.request(AttackSpec::jab(Side::Right));
		state.tick(jab_dt(0.9));
		assert!(state.just_started());
		assert_eq!(state.side(), Some(Side::Right));
		assert_eq!(state.clip_progress().map(|progress| progress < 0.05), Some(true));
	}

	#[test]
	fn aim_latches_when_the_jab_starts() {
		let mut state = AttackState::default();
		state.request(AttackSpec::jab(Side::Left));
		assert!(state.just_started());
		let up = JabAim::from_camera_angles(PITCH_MAX, 0.0);
		state.latch_aim(up);
		assert_eq!(state.aim(), Some(up));

		state.tick(jab_dt(0.1));
		assert!(!state.just_started());
		let down = JabAim::from_camera_angles(PITCH_MIN, 0.0);
		state.latch_aim(down);
		assert_eq!(state.aim(), Some(up), "look changes mid-punch must not retarget");
	}

	#[test]
	fn cancel_clears_a_pending_active_window() {
		let mut state = AttackState::default();
		state.request(AttackSpec::jab(Side::Left));
		state.tick(jab_dt(JAB_STARTUP_END));
		assert!(state.just_entered_active() || state.phase() == Some(AttackPhase::Active));
		state.request(AttackSpec::jab(Side::Right));
		assert!(state.queued().is_some());
		state.cancel();
		assert!(!state.is_busy());
		assert!(!state.just_entered_active());
		assert!(state.queued().is_none());
		state.tick(jab_dt(0.05));
		assert!(!state.just_entered_active());
		assert!(!state.is_busy());
	}

	#[test]
	fn queued_jab_relatches_when_it_starts() {
		let mut state = AttackState::default();
		state.request(AttackSpec::jab(Side::Left));
		state.latch_aim(JabAim::from_camera_angles(PITCH_MAX, 0.0));
		state.tick(jab_dt(0.1));
		state.request(AttackSpec::jab(Side::Right));
		state.tick(jab_dt(0.95));
		assert!(state.just_started());
		let yawed = JabAim::from_camera_angles(0.0, YAW_LIMIT);
		state.latch_aim(yawed);
		assert_eq!(state.aim(), Some(yawed));
	}
}
