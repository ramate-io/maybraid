//! Create flight / collision / damage entities. Presentation attaches visuals.
//!
//! One shared interpreter pass consumes the batch. Construction is
//! synchronous; there is no pending-request tracker.

use bevy::prelude::*;
use projectiles::{spawn_flight_sim, ProjectileVisualCache};

use crate::presentation::attach_projectile_visual;
use crate::request::{ProjectileVisual, SpawnProjectile};

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

/// Realize the batch. When presentation assets are present, attach meshes in
/// the same pass so the equivalent-output path does not walk `Added` again.
pub fn realize_projectiles(
	mut commands: Commands,
	mut requests: MessageReader<SpawnProjectile>,
	cache: Option<ResMut<ProjectileVisualCache>>,
	meshes: Option<ResMut<Assets<Mesh>>>,
	materials: Option<ResMut<Assets<StandardMaterial>>>,
) {
	match (cache, meshes, materials) {
		(Some(mut cache), Some(mut meshes), Some(mut materials)) => {
			for request in requests.read() {
				let entity = request.realize(&mut commands);
				let (length, radius, _, _, _, _, color, _) = request.spec.ballistic();
				attach_projectile_visual(
					&mut commands,
					entity,
					&ProjectileVisual::new(length, radius, color),
					&mut cache,
					&mut meshes,
					&mut materials,
				);
			}
		}
		_ => {
			for request in requests.read() {
				request.realize(&mut commands);
			}
		}
	}
}
