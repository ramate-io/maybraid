//! Catalog [`FirearmStats`] → live [`Weapon`] / cadence / payload.

use crozon_character_items::{FireMode, FirearmStats, ProjectileKind};
use damage::{HitPayload, DEFAULT_HIT};
use firearms::{
	BoltSpec, BulletSpec, FireControl, LaserSpec, ProjectileLoad, Weapon, WeaponRecoil,
};

/// Look-up pitch (radians) per catalog recoil unit. Positive pitch is look up.
pub const RECOIL_PITCH_PER_UNIT: f32 = 0.02;

/// Components stamped on a held [`firearms::FirearmRoot`] at spawn.
#[derive(Clone, Copy, Debug)]
pub struct LiveWeapon {
	pub weapon: Weapon,
	pub payload: HitPayload,
	pub fire: FireControl,
	pub recoil: WeaponRecoil,
}

impl Default for LiveWeapon {
	fn default() -> Self {
		Self {
			weapon: Weapon::bolt(),
			payload: HitPayload { amount: DEFAULT_HIT },
			fire: FireControl::auto(),
			recoil: WeaponRecoil(0.0),
		}
	}
}

/// Bake integer catalog stats plus the wearer's outgoing DPC bonus.
pub fn live_weapon_from_stats(stats: FirearmStats, outgoing_damage_bonus: i16) -> LiveWeapon {
	let (fire, interval) = cadence_from_stats(stats);
	let load = load_from_stats(stats);
	let amount = (f32::from(stats.damage) + f32::from(outgoing_damage_bonus)).max(0.0);
	LiveWeapon {
		weapon: Weapon::new(load, interval),
		payload: HitPayload { amount },
		fire,
		recoil: WeaponRecoil(f32::from(stats.recoil) * RECOIL_PITCH_PER_UNIT),
	}
}

fn cadence_from_stats(stats: FirearmStats) -> (FireControl, f32) {
	match stats.fire {
		Some(FireMode::FullAuto { rpm }) => (FireControl::auto(), interval_from_rpm(rpm)),
		Some(FireMode::Burst { rounds, rpm }) => {
			(FireControl::burst(rounds), interval_from_rpm(rpm))
		}
		Some(FireMode::SemiAuto) => (FireControl::semi(), 0.0),
		Some(FireMode::Gated { recharge_tenths }) => {
			(FireControl::gated(), f32::from(recharge_tenths) / 10.0)
		}
		None => match stats.projectile {
			ProjectileKind::Laser => (FireControl::auto(), 0.0),
			_ => (FireControl::auto(), Weapon::bolt().interval),
		},
	}
}

fn interval_from_rpm(rpm: u16) -> f32 {
	60.0 / f32::from(rpm.max(1))
}

fn load_from_stats(stats: FirearmStats) -> ProjectileLoad {
	let speed = f32::from(stats.speed.max(1));
	let range = f32::from(stats.range);
	let travel = (range / speed) + 0.25;
	match stats.projectile {
		ProjectileKind::Bolt => {
			let bolt = BoltSpec::default();
			ProjectileLoad::Bolt(BoltSpec {
				speed,
				max_range: range,
				penetration: f32::from(stats.penetration) / 1000.0,
				max_age: bolt.max_age.max(travel),
				..bolt
			})
		}
		ProjectileKind::Bullet => {
			let bullet = BulletSpec::default();
			ProjectileLoad::Bullet(BulletSpec {
				speed,
				max_range: range,
				penetration: f32::from(stats.penetration) / 1000.0,
				max_age: bullet.max_age.max(travel),
				..bullet
			})
		}
		ProjectileKind::Laser => ProjectileLoad::Laser(LaserSpec {
			max_length: range,
			max_time: travel.clamp(0.2, 2.0),
			..LaserSpec::default()
		}),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use firearms::Cadence;

	fn bolt_auto() -> FirearmStats {
		FirearmStats {
			projectile: ProjectileKind::Bolt,
			speed: 180,
			penetration: 600,
			range: 40,
			fire: Some(FireMode::FullAuto { rpm: 600 }),
			recoil: 4,
			damage: 25,
			weight: 10,
		}
	}

	#[test]
	fn auto_rpm_becomes_interval() {
		let live = live_weapon_from_stats(bolt_auto(), 0);
		assert!((live.weapon.interval - 0.1).abs() < 1e-5);
		assert_eq!(live.fire.cadence, Cadence::Auto);
		assert_eq!(live.payload.amount, 25.0);
	}

	#[test]
	fn clothing_damage_adds_to_dpc() {
		let live = live_weapon_from_stats(bolt_auto(), 10);
		assert_eq!(live.payload.amount, 35.0);
	}

	#[test]
	fn penetration_millunits_become_through() {
		let ProjectileLoad::Bolt(spec) = live_weapon_from_stats(bolt_auto(), 0).weapon.load else {
			panic!("expected bolt");
		};
		assert!((spec.penetration - 0.6).abs() < 1e-5);
		assert_eq!(spec.speed, 180.0);
		assert_eq!(spec.max_range, 40.0);
	}

	#[test]
	fn gated_tenths_become_seconds() {
		let mut stats = bolt_auto();
		stats.fire = Some(FireMode::Gated { recharge_tenths: 12 });
		let live = live_weapon_from_stats(stats, 0);
		assert!((live.weapon.interval - 1.2).abs() < 1e-5);
		assert_eq!(live.fire.cadence, Cadence::Gated);
	}

	#[test]
	fn burst_keeps_round_count() {
		let mut stats = bolt_auto();
		stats.fire = Some(FireMode::Burst { rounds: 3, rpm: 900 });
		let live = live_weapon_from_stats(stats, 0);
		assert_eq!(live.fire.cadence, Cadence::Burst);
		assert_eq!(live.fire.burst_rounds, 3);
		assert!((live.weapon.interval - 60.0 / 900.0).abs() < 1e-5);
	}

	#[test]
	fn laser_uses_range_as_length() {
		let stats = FirearmStats {
			projectile: ProjectileKind::Laser,
			speed: 90,
			penetration: 0,
			range: 80,
			fire: None,
			recoil: 0,
			damage: 18,
			weight: 8,
		};
		let live = live_weapon_from_stats(stats, 0);
		let ProjectileLoad::Laser(spec) = live.weapon.load else {
			panic!("expected laser");
		};
		assert_eq!(spec.max_length, 80.0);
		assert_eq!(live.payload.amount, 18.0);
	}

	#[test]
	fn recoil_becomes_look_pitch() {
		let live = live_weapon_from_stats(bolt_auto(), 0);
		assert!((live.recoil.0 - 0.08).abs() < 1e-5);
	}
}
