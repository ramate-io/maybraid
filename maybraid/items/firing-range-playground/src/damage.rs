use bevy::prelude::*;
use projectiles::ProjectileContact;

pub(crate) const MAX_HEALTH: f32 = 100.0;
const PROJECTILE_DAMAGE: f32 = 25.0;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct Health {
	pub current: f32,
	pub max: f32,
}

impl Default for Health {
	fn default() -> Self {
		Self { current: MAX_HEALTH, max: MAX_HEALTH }
	}
}

impl Health {
	pub fn apply_damage(&mut self, damage: f32) {
		self.current = (self.current - damage.max(0.0)).max(0.0);
	}
}

pub(crate) fn apply_projectile_damage(
	mut contacts: MessageReader<ProjectileContact>,
	mut health: Query<&mut Health>,
) {
	for contact in contacts.read() {
		if contact.source == Some(contact.target) {
			continue;
		}
		let Ok(mut target) = health.get_mut(contact.target) else {
			continue;
		};
		target.apply_damage(PROJECTILE_DAMAGE);
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
	}
}
