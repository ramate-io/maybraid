//! Health-triggered sleep/recovery as explicit live phase.
//!
//! Open combat rules stay labeled on [`EncounterSleepPolicy`]. Defaults for
//! Bikanwarda match [#1120](https://github.com/ramate-io/maybraid/issues/1120)
//! numbers; one-time vs repeatable, gradual vs end restore, and early-wake
//! remaining recovery are configuration, not silent new rules.

use attacks::AttackSuspended;
use bevy::prelude::*;
use damage::Health;

use crate::lifetime::EncounterTimeEnabled;
use crate::progress::{EncounterLifetime, EncounterPhase, EncounterRecord, EncounterSleep};
use crate::store::EncounterProgressStore;

/// Bikanwarda enter threshold. Crossing to this fraction or lower enters sleep.
pub const BIKANWARDA_SLEEP_ENTER_FRACTION: f32 = 0.20;
/// Bikanwarda sleep duration in game seconds.
pub const BIKANWARDA_SLEEP_DURATION_SECS: f32 = 15.0;
/// Health fraction restored when a full sleep completes.
pub const BIKANWARDA_SLEEP_RESTORE_FRACTION: f32 = 0.50;
/// Early-wake threshold while sleeping.
pub const BIKANWARDA_SLEEP_INTERRUPT_FRACTION: f32 = 0.10;

/// How many times this occurrence may enter sleep.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncounterSleepRepeat {
	/// PLACEHOLDER default for Bikanwarda: one sleep per occurrence.
	Once,
	Repeatable,
}

/// When restored HP is applied.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncounterSleepRestore {
	/// PLACEHOLDER default: apply the restore amount when the 15 s session ends.
	EndOfSleep,
	/// Spread restoration across the sleep session.
	Gradual,
}

/// What happens to remaining restore if she wakes at 10%.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncounterEarlyWake {
	/// PLACEHOLDER default: waking cancels remaining recovery.
	CancelRemaining,
	/// Apply whatever restore is still owed, then wake.
	KeepRemaining,
}

/// Explicit sleep configuration. Disabled by default so non-boss fixtures
/// do not pick up Bikanwarda combat rules.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct EncounterSleepPolicy {
	pub enabled: bool,
	pub enter_fraction: f32,
	pub duration_secs: f32,
	pub restore_fraction: f32,
	pub interrupt_fraction: f32,
	pub repeat: EncounterSleepRepeat,
	pub restoration: EncounterSleepRestore,
	pub early_wake: EncounterEarlyWake,
}

impl Default for EncounterSleepPolicy {
	fn default() -> Self {
		Self {
			enabled: false,
			enter_fraction: BIKANWARDA_SLEEP_ENTER_FRACTION,
			duration_secs: BIKANWARDA_SLEEP_DURATION_SECS,
			restore_fraction: BIKANWARDA_SLEEP_RESTORE_FRACTION,
			interrupt_fraction: BIKANWARDA_SLEEP_INTERRUPT_FRACTION,
			repeat: EncounterSleepRepeat::Once,
			restoration: EncounterSleepRestore::EndOfSleep,
			early_wake: EncounterEarlyWake::CancelRemaining,
		}
	}
}

impl EncounterSleepPolicy {
	/// Authored Bikanwarda numbers. Repeat / restore / early-wake remain
	/// labeled PLACEHOLDER choices from [#1120](https://github.com/ramate-io/maybraid/issues/1120).
	pub fn bikanwarda() -> Self {
		Self { enabled: true, ..Self::default() }
	}

	fn enter_fraction(self) -> f32 {
		sanitize_fraction(self.enter_fraction, BIKANWARDA_SLEEP_ENTER_FRACTION)
	}

	fn interrupt_fraction(self) -> f32 {
		sanitize_fraction(self.interrupt_fraction, BIKANWARDA_SLEEP_INTERRUPT_FRACTION)
	}

	fn restore_fraction(self) -> f32 {
		sanitize_fraction(self.restore_fraction, BIKANWARDA_SLEEP_RESTORE_FRACTION)
	}

	fn duration_secs(self) -> f32 {
		if self.duration_secs.is_finite() && self.duration_secs > 0.0 {
			self.duration_secs
		} else {
			BIKANWARDA_SLEEP_DURATION_SECS
		}
	}
}

fn sanitize_fraction(value: f32, fallback: f32) -> f32 {
	if value.is_finite() {
		value.clamp(0.0, 1.0)
	} else {
		fallback
	}
}

pub(crate) fn tick_sleep(
	time: Res<Time>,
	enabled: Res<EncounterTimeEnabled>,
	policy: Res<EncounterSleepPolicy>,
	mut store: ResMut<EncounterProgressStore>,
	mut healths: Query<&mut Health>,
	suspended: Query<(), With<AttackSuspended>>,
	mut commands: Commands,
) {
	if !policy.enabled {
		return;
	}
	let ids: Vec<_> = store.iter().map(|record| record.id).collect();
	for id in ids {
		let Some(instance) = store.get(id).and_then(|record| record.instance) else {
			continue;
		};
		if store.get(id).is_some_and(EncounterRecord::is_finished) {
			if suspended.get(instance).is_ok() {
				commands.entity(instance).try_remove::<AttackSuspended>();
			}
			continue;
		}
		let Ok(mut health) = healths.get_mut(instance) else {
			continue;
		};
		if health.is_dead() {
			if suspended.get(instance).is_ok() {
				commands.entity(instance).try_remove::<AttackSuspended>();
			}
			continue;
		}
		let record = store.record_mut(id);
		if record.lifetime != EncounterLifetime::InProgress {
			continue;
		}
		drive_sleep(record, &mut health, &policy, enabled.0, time.delta_secs());
		sync_suspended(record, instance, &suspended, &mut commands);
	}
}

fn drive_sleep(
	record: &mut EncounterRecord,
	health: &mut Health,
	policy: &EncounterSleepPolicy,
	time_enabled: bool,
	dt: f32,
) {
	let fraction = health.fraction();
	let previous = record.last_health_fraction;
	record.last_health_fraction = Some(fraction);

	if matches!(record.phase, EncounterPhase::Waking) {
		record.phase = EncounterPhase::Combat;
		return;
	}

	if let EncounterPhase::Sleep(sleep) = record.phase {
		if fraction <= policy.interrupt_fraction() {
			wake(record, health, policy, true);
			return;
		}
		if !time_enabled {
			return;
		}
		let mut sleep = sleep;
		sleep.elapsed_secs = (sleep.elapsed_secs + dt.max(0.0)).max(0.0);
		if policy.restoration == EncounterSleepRestore::Gradual {
			let cap = health.max * policy.restore_fraction();
			let span = (cap - sleep.health_at_enter).max(0.0);
			let step = if policy.duration_secs() > 0.0 {
				span * (dt.max(0.0) / policy.duration_secs())
			} else {
				0.0
			};
			health.heal_toward(step, cap);
		}
		if sleep.elapsed_secs >= policy.duration_secs() {
			if policy.restoration == EncounterSleepRestore::EndOfSleep {
				let cap = health.max * policy.restore_fraction();
				health.heal_toward(health.max, cap);
			}
			record.phase = EncounterPhase::Waking;
			return;
		}
		record.phase = EncounterPhase::Sleep(sleep);
		return;
	}

	if previous.is_none() || !time_enabled {
		return;
	}
	let Some(previous) = previous else {
		return;
	};
	let can_repeat = match policy.repeat {
		EncounterSleepRepeat::Once => !record.sleep_used,
		EncounterSleepRepeat::Repeatable => true,
	};
	if can_repeat && previous > policy.enter_fraction() && fraction <= policy.enter_fraction() {
		record.sleep_used = true;
		record.phase = EncounterPhase::Sleep(EncounterSleep::new(health.current));
		if fraction <= policy.interrupt_fraction() {
			wake(record, health, policy, true);
		}
	}
}

fn wake(
	record: &mut EncounterRecord,
	health: &mut Health,
	policy: &EncounterSleepPolicy,
	early: bool,
) {
	if early && policy.early_wake == EncounterEarlyWake::KeepRemaining {
		let cap = health.max * policy.restore_fraction();
		health.heal_toward(health.max, cap);
	}
	record.phase = EncounterPhase::Waking;
}

fn sync_suspended(
	record: &EncounterRecord,
	instance: Entity,
	suspended: &Query<(), With<AttackSuspended>>,
	commands: &mut Commands,
) {
	let sleeping = matches!(record.phase, EncounterPhase::Sleep(_));
	let has = suspended.get(instance).is_ok();
	if sleeping && !has {
		commands.entity(instance).try_insert(AttackSuspended);
	} else if !sleeping && has {
		commands.entity(instance).try_remove::<AttackSuspended>();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn bikanwarda_policy_uses_authored_thresholds() {
		let policy = EncounterSleepPolicy::bikanwarda();
		assert!(policy.enabled);
		assert_eq!(policy.enter_fraction(), 0.20);
		assert_eq!(policy.duration_secs(), 15.0);
		assert_eq!(policy.restore_fraction(), 0.50);
		assert_eq!(policy.interrupt_fraction(), 0.10);
		assert_eq!(policy.repeat, EncounterSleepRepeat::Once);
		assert_eq!(policy.restoration, EncounterSleepRestore::EndOfSleep);
		assert_eq!(policy.early_wake, EncounterEarlyWake::CancelRemaining);
	}
}
