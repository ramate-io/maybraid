//! HP pool and a generic [`Hit`] seam. Weapons stamp [`HitPayload`]; this crate
//! does not know about firearms.

mod apply;
mod from_projectiles;

use bevy::prelude::*;
use projectiles::tick_flights;

pub use apply::apply_hits;
pub use from_projectiles::contacts_to_hits;

/// Default DPC used when a bolt is spawned without catalog stats.
pub const DEFAULT_HIT: f32 = 25.0;

/// Baseline max HP before clothing modifiers.
pub const DEFAULT_MAX_HEALTH: f32 = 100.0;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DamageSystems {
	Collect,
	Apply,
}

/// Current / max hit points. Missing means the entity cannot be hurt this way.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Health {
	pub current: f32,
	pub max: f32,
}

impl Default for Health {
	fn default() -> Self {
		Self::from_max(DEFAULT_MAX_HEALTH)
	}
}

impl Health {
	pub fn from_max(max: f32) -> Self {
		let max = max.max(1.0);
		Self { current: max, max }
	}

	pub fn apply_damage(&mut self, damage: f32) {
		self.current = (self.current - damage.max(0.0)).max(0.0);
	}

	pub fn is_dead(self) -> bool {
		self.current <= 0.0
	}

	pub fn fraction(self) -> f32 {
		if self.max <= 0.0 {
			0.0
		} else {
			(self.current / self.max).clamp(0.0, 1.0)
		}
	}
}

/// Amount copied onto a projectile (or read off a laser) at fire time.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct HitPayload {
	pub amount: f32,
}

impl Default for HitPayload {
	fn default() -> Self {
		Self { amount: DEFAULT_HIT }
	}
}

/// Requested injury. Any producer may write this.
#[derive(Message, Clone, Copy, Debug)]
pub struct Hit {
	pub target: Entity,
	pub source: Option<Entity>,
	pub amount: f32,
	pub point: Vec3,
}

/// HP actually subtracted.
#[derive(Message, Clone, Copy, Debug)]
pub struct DamageApplied {
	pub target: Entity,
	pub source: Option<Entity>,
	pub amount: f32,
	pub remaining: f32,
	pub point: Vec3,
}

/// `Health` crossed zero this apply.
#[derive(Message, Clone, Copy, Debug)]
pub struct Died {
	pub entity: Entity,
	pub source: Option<Entity>,
}

pub struct DamagePlugin;

impl Plugin for DamagePlugin {
	fn build(&self, app: &mut App) {
		app.add_message::<Hit>()
			.add_message::<DamageApplied>()
			.add_message::<Died>()
			.configure_sets(
				PostUpdate,
				(
					DamageSystems::Collect.after(tick_flights),
					DamageSystems::Apply.after(DamageSystems::Collect),
				),
			)
			.add_systems(
				PostUpdate,
				(
					contacts_to_hits.in_set(DamageSystems::Collect),
					apply_hits.in_set(DamageSystems::Apply),
				),
			);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn damage_clamps_at_zero() {
		let mut health = Health::default();
		health.apply_damage(25.0);
		assert_eq!(health.current, 75.0);
		health.apply_damage(100.0);
		assert_eq!(health.current, 0.0);
		assert!(health.is_dead());
	}

	#[test]
	fn fraction_tracks_remaining() {
		let mut health = Health::default();
		assert_eq!(health.fraction(), 1.0);
		health.apply_damage(50.0);
		assert!((health.fraction() - 0.5).abs() < 1e-5);
		health.apply_damage(50.0);
		assert_eq!(health.fraction(), 0.0);
	}

	#[test]
	fn from_max_floors_at_one() {
		let health = Health::from_max(0.0);
		assert_eq!(health.max, 1.0);
		assert_eq!(health.current, 1.0);
	}
}
