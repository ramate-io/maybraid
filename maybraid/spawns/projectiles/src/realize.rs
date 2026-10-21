//! Create flight / collision / damage entities. Presentation attaches visuals.

use bevy::prelude::*;
use projectiles::spawn_flight_sim;

use crate::request::{ProjectileSpawned, SpawnProjectile};

impl SpawnProjectile {
	pub fn realize(&self, commands: &mut Commands) -> ProjectileSpawned {
		let (length, radius, speed, max_range, penetration, max_age, _color, gravity) =
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
		commands.entity(projectile).insert(self.projectile_source());
		if let Some(payload) = self.payload {
			commands.entity(projectile).insert(payload);
		}
		ProjectileSpawned::new(self.identity, projectile, self.source)
	}
}

pub fn realize_projectiles(
	mut commands: Commands,
	mut requests: MessageReader<SpawnProjectile>,
	mut spawned: MessageWriter<ProjectileSpawned>,
) {
	for request in requests.read() {
		spawned.write(request.realize(&mut commands));
	}
}
