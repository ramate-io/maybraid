//! Simulation-first projectile spawn. Visual assets attach afterward.

pub mod realize;
pub mod request;

pub use realize::realize_projectiles;
pub use request::{ProjectileSpawned, ProjectileSpec, SpawnProjectile};

use bevy::prelude::*;

#[derive(Resource, Default)]
struct ProjectileSpawnRuntime;

pub fn register_projectile_spawn_runtime(app: &mut App) {
	if app.world().contains_resource::<ProjectileSpawnRuntime>() {
		return;
	}
	app.init_resource::<ProjectileSpawnRuntime>()
		.add_message::<SpawnProjectile>()
		.add_message::<ProjectileSpawned>();
}

pub struct ProjectileSpawnPlugin;

impl Plugin for ProjectileSpawnPlugin {
	fn build(&self, app: &mut App) {
		register_projectile_spawn_runtime(app);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use maybraid_spawns_core::{SpawnIdentity, SpawnOwner, SpawnRequestId, SpawnSessionId};
	use projectiles::{BoltSpec, Flight};

	fn identity() -> SpawnIdentity {
		SpawnIdentity::new(SpawnRequestId::new(1), SpawnOwner::new(SpawnSessionId::new(1)))
	}

	#[test]
	fn realize_creates_flight_without_a_mesh() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Assets<bevy::prelude::Mesh>>();
		let spawned = world
			.run_system_once(|mut commands: Commands| {
				SpawnProjectile::new(
					identity(),
					Entity::from_bits(2),
					Vec3::ZERO,
					Vec3::Z,
					ProjectileSpec::Bolt(BoltSpec::default()),
					0.0,
					None,
				)
				.realize(&mut commands)
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get::<Flight>(spawned.projectile).is_some());
		assert!(world.get::<Mesh3d>(spawned.projectile).is_none());
		Ok(())
	}

	#[test]
	fn register_projectile_spawn_runtime_is_idempotent() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_projectile_spawn_runtime(&mut app);
		register_projectile_spawn_runtime(&mut app);
		assert!(app.world().contains_resource::<Messages<SpawnProjectile>>());
	}
}
