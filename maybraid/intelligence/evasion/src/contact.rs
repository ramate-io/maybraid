use bevy::prelude::*;

/// Last known state of one assailant. This is a knowledge snapshot, not a live transform.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AssailantContact {
	pub subject: Entity,
	pub position: Vec3,
	pub movement_vector: Vec3,
	pub last_known_at: f32,
}

impl AssailantContact {
	pub fn is_fresh(self, now: f32, memory_secs: f32) -> bool {
		now - self.last_known_at <= memory_secs.max(0.0)
	}

	pub fn xz_distance(self, from: Vec3) -> f32 {
		Vec2::new(self.position.x, self.position.z).distance(Vec2::new(from.x, from.z))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn freshness_uses_last_known() -> anyhow::Result<()> {
		let contact = AssailantContact {
			subject: Entity::from_bits(1),
			position: Vec3::X,
			movement_vector: Vec3::ZERO,
			last_known_at: 1.0,
		};
		assert!(contact.is_fresh(2.0, 1.0));
		assert!(!contact.is_fresh(2.01, 1.0));
		Ok(())
	}
}
