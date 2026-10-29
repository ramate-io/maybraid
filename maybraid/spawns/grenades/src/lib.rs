//! Simulation-first thrown-grenade spawn. Visual assets attach afterward.

pub mod realize;
pub mod request;

pub use realize::realize_thrown_grenades;
pub use request::{DropLiveGrenade, GrenadeSpawned, SpawnThrownGrenade, ThrownGrenadeSpawn};

use bevy::prelude::*;

#[derive(Resource, Default)]
struct GrenadeSpawnRuntime;

pub fn register_grenade_spawn_runtime(app: &mut App) {
	if app.world().contains_resource::<GrenadeSpawnRuntime>() {
		return;
	}
	app.init_resource::<GrenadeSpawnRuntime>()
		.add_message::<SpawnThrownGrenade>()
		.add_message::<DropLiveGrenade>()
		.add_message::<GrenadeSpawned>()
		.add_systems(Update, realize_thrown_grenades);
}

pub struct GrenadeSpawnPlugin;

impl Plugin for GrenadeSpawnPlugin {
	fn build(&self, app: &mut App) {
		register_grenade_spawn_runtime(app);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use character_items::{GrenadeSpec, GrenadeStats};
	use grenades::{GrenadeEffect, GrenadeFuse, ThrownGrenade};

	#[test]
	fn realize_creates_a_body_without_a_mesh() -> anyhow::Result<()> {
		let mut world = World::new();
		let stats = GrenadeStats::standard();
		let spawned = world
			.run_system_once(move |mut commands: Commands| {
				SpawnThrownGrenade::new(
					Vec3::ZERO,
					Vec3::Z,
					GrenadeSpec::standard(),
					stats,
					GrenadeEffect::from_stats(&stats),
					Entity::from_bits(2),
					1.25,
				)
				.spawn()
				.realize_body(&mut commands)
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get::<ThrownGrenade>(spawned).is_some());
		assert_eq!(world.get::<GrenadeFuse>(spawned).map(|fuse| fuse.remaining), Some(1.25));
		assert!(world.get::<Mesh3d>(spawned).is_none());
		Ok(())
	}

	#[test]
	fn register_grenade_spawn_runtime_is_idempotent() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_grenade_spawn_runtime(&mut app);
		register_grenade_spawn_runtime(&mut app);
		assert!(app.world().contains_resource::<Messages<SpawnThrownGrenade>>());
	}
}
