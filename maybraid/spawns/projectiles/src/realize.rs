//! Create flight / collision / damage entities. Presentation attaches visuals.
//!
//! One shared interpreter pass consumes the batch. Construction is
//! synchronous; there is no pending-request tracker.

use bevy::prelude::*;
use projectiles::spawn_flight_sim;

use crate::request::SpawnProjectile;

impl SpawnProjectile {
	/// Build the simulation body. Does not emit a spawn-success message.
	pub fn realize(&self, commands: &mut Commands) -> Entity {
		let (length, radius, speed, max_range, penetration, max_age, color, gravity) =
			self.spec.ballistic();
		let projectile = spawn_flight_sim(
			commands,
			self.origin,
			self.direction,
			length,
			radius,
			speed,
			max_range,
			penetration,
			max_age,
			gravity,
			self.age,
		);
		commands.entity(projectile).insert((
			self.projectile_source(),
			crate::request::ProjectileVisual::new(length, radius, color),
		));
		if let Some(payload) = self.payload {
			commands.entity(projectile).insert(payload);
		}
		projectile
	}
}

pub fn realize_projectiles(mut commands: Commands, mut requests: MessageReader<SpawnProjectile>) {
	for request in requests.read() {
		request.realize(&mut commands);
	}
}
