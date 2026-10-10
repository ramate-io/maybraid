//! Trigger cadence sitting beside [`Weapon`] interval.

use bevy::prelude::*;

/// Max shots one fire tick may emit after a hitch. Drops leftover debt past this.
pub const CATCH_UP_SHOTS: u8 = 3;

/// How many shots this tick may catch up for `control`. Semi is one rising edge.
pub fn catch_up_limit(control: Option<&FireControl>) -> u8 {
	match control.map(|control| control.cadence) {
		Some(Cadence::Semi) => 1,
		Some(Cadence::Burst) => {
			CATCH_UP_SHOTS.min(control.map(|control| control.burst_left).unwrap_or(0))
		}
		_ => CATCH_UP_SHOTS,
	}
}

/// Subtract `dt` from `cooldown` and return how many shots this tick should emit.
///
/// Leftover time stays on `cooldown` (`+= interval` per shot). Hitting `max_shots`
/// while still overdue snaps `cooldown` to `interval` so a pause cannot dump a magazine.
pub fn advance_shot_clock(cooldown: &mut f32, interval: f32, dt: f32, max_shots: u8) -> u8 {
	*cooldown -= dt;
	if *cooldown > 0.0 || max_shots == 0 {
		return 0;
	}
	let interval = interval.max(0.0);
	if interval <= 0.0 {
		*cooldown = 0.0;
		return 1;
	}
	let mut shots = 0u8;
	while *cooldown <= 0.0 && shots < max_shots {
		shots += 1;
		*cooldown += interval;
	}
	if shots >= max_shots && *cooldown <= 0.0 {
		*cooldown = interval;
	}
	shots
}

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

/// Look/camera kick strength in radians after a shot. Zero skips the pattern.
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

	#[test]
	fn six_hundred_rpm_at_sixteen_ms_is_ten_shots_per_second() {
		let interval = 60.0 / 600.0;
		let mut cooldown = 0.0;
		let mut shots = 0u32;
		for _ in 0..62 {
			shots += u32::from(advance_shot_clock(&mut cooldown, interval, 0.016, CATCH_UP_SHOTS));
		}
		assert!((9..=11).contains(&shots), "{shots}");
	}

	#[test]
	fn fifteen_hundred_rpm_at_thirty_three_ms_stays_near_catalog() {
		let interval = 60.0 / 1500.0;
		let mut cooldown = 0.0;
		let mut shots = 0u32;
		let dt = 0.033;
		let ticks = (1.0_f32 / dt).round() as u32;
		for _ in 0..ticks {
			shots += u32::from(advance_shot_clock(&mut cooldown, interval, dt, CATCH_UP_SHOTS));
		}
		// Tick quantization can rise toward 1/dt (30 Hz); leftover-discard used to fall to ~15 Hz.
		assert!(shots >= 24, "{shots}");
		assert!(shots <= ticks, "{shots} vs {ticks}");
	}

	#[test]
	fn hitch_does_not_exceed_catch_up_cap() {
		let interval = 60.0 / 1500.0;
		let mut cooldown = 0.0;
		let shots = advance_shot_clock(&mut cooldown, interval, 1.0, CATCH_UP_SHOTS);
		assert_eq!(shots, CATCH_UP_SHOTS);
		assert!(cooldown > 0.0);
	}

	#[test]
	fn semi_is_one_shot_per_tick() {
		assert_eq!(catch_up_limit(Some(&FireControl::semi())), 1);
		let mut cooldown = 0.0;
		let shots = advance_shot_clock(&mut cooldown, 0.05, 0.25, 1);
		assert_eq!(shots, 1);
	}

	#[test]
	fn missing_barrel_does_not_consume_a_semi_edge() {
		let mut control = FireControl::semi();
		let held = true;
		let barrel_present = false;
		if barrel_present {
			let intent = control.poll(held);
			assert!(control.allows(intent));
		}
		assert!(!control.trigger_was);
		let intent = control.poll(true);
		assert!(control.allows(intent));
	}

	#[test]
	fn held_auto_counts_match_when_the_barrel_is_always_present() {
		fn held_auto_shots(interval: f32, dt: f32, secs: f32) -> u32 {
			let mut cooldown = 0.0;
			let mut shots = 0u32;
			let mut t = 0.0;
			while t < secs - 1e-6 {
				t += dt;
				shots += u32::from(advance_shot_clock(&mut cooldown, interval, dt, CATCH_UP_SHOTS));
			}
			shots
		}
		let hip = held_auto_shots(0.1, 0.016, 2.0);
		let ads = held_auto_shots(0.1, 0.016, 2.0);
		assert_eq!(hip, ads);
		assert!((19..=21).contains(&hip), "{hip}");
	}

	fn simulate_held(mut control: FireControl, interval: f32, dt: f32, secs: f32) -> u32 {
		let mut cooldown = 0.0;
		let mut shots = 0u32;
		let mut t = 0.0;
		while t < secs - 1e-6 {
			t += dt;
			let allowed = trigger_allows_fire(Some(&mut control), true, true);
			if !allowed {
				cooldown -= dt;
				continue;
			}
			let n = advance_shot_clock(&mut cooldown, interval, dt, catch_up_limit(Some(&control)));
			for _ in 0..n {
				if control.cadence == Cadence::Burst && control.burst_left == 0 {
					break;
				}
				shots += 1;
				control.note_shot();
			}
		}
		shots
	}

	#[test]
	fn locked_dt_hip_and_ads_spawn_counts_match() {
		let dt = 0.016;
		let secs = 2.0;
		let cases: [(FireControl, f32); 5] = [
			(FireControl::auto(), 60.0 / 600.0),
			(FireControl::auto(), 60.0 / 1500.0),
			(FireControl::burst(3), 60.0 / 900.0),
			(FireControl::semi(), 1.0 / 20.0),
			(FireControl::gated(), 1.2),
		];
		for (control, interval) in cases {
			let hip = simulate_held(control, interval, dt, secs);
			let ads = simulate_held(control, interval, dt, secs);
			assert_eq!(hip, ads, "cadence {:?}", control.cadence);
		}
	}

	#[test]
	fn locked_dt_held_cadence_counts() {
		let dt = 0.016;
		assert!((19..=21).contains(&simulate_held(FireControl::auto(), 0.1, dt, 2.0)));
		let fast = simulate_held(FireControl::auto(), 60.0 / 1500.0, dt, 2.0);
		assert!((48..=52).contains(&fast), "{fast}");
		assert_eq!(simulate_held(FireControl::burst(3), 60.0 / 900.0, dt, 2.0), 3);
		assert_eq!(simulate_held(FireControl::semi(), 1.0 / 20.0, dt, 2.0), 1);
		assert_eq!(simulate_held(FireControl::gated(), 1.2, dt, 2.0), 2);
	}

	#[test]
	fn skipped_barrel_ticks_slow_held_auto() {
		let interval = 0.1;
		let dt = 0.016;
		let mut full = 0u32;
		let mut skipped = 0u32;
		let mut cooldown_full = 0.0;
		let mut cooldown_skip = 0.0;
		for i in 0..125 {
			full += u32::from(advance_shot_clock(&mut cooldown_full, interval, dt, CATCH_UP_SHOTS));
			if i % 4 == 0 {
				continue;
			}
			skipped +=
				u32::from(advance_shot_clock(&mut cooldown_skip, interval, dt, CATCH_UP_SHOTS));
		}
		assert!(skipped < full, "skip {skipped} vs full {full}");
	}
}
