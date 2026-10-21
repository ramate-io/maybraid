//! Cached mesh / material attachment. Simulation realize stays mesh-free.

use bevy::prelude::*;
use projectiles::{Flight, ProjectileVisualCache};

use crate::request::ProjectileVisual;

/// Attach cached visuals to flights that carry a compact visual spec.
pub fn attach_projectile_visuals(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	mut visuals: ResMut<ProjectileVisualCache>,
	added: Query<(Entity, &ProjectileVisual), (With<Flight>, Without<Mesh3d>)>,
) {
	for (entity, spec) in &added {
		let (mesh, material) = visuals.mesh_and_material(
			spec.length,
			spec.radius,
			spec.color,
			&mut meshes,
			&mut materials,
		);
		commands.entity(entity).insert((Mesh3d(mesh), MeshMaterial3d(material)));
	}
}

#[derive(Resource, Default)]
struct ProjectilePresentation;

/// Visual attachment after simulation realize. Idempotent. Headless sim
/// registration does not call this.
pub fn register_projectile_presentation(app: &mut App) {
	if app.world().contains_resource::<ProjectilePresentation>() {
		return;
	}
	crate::register_projectile_spawn_runtime(app);
	app.init_resource::<ProjectileVisualCache>()
		.init_resource::<ProjectilePresentation>()
		.add_systems(PostUpdate, attach_projectile_visuals.after(crate::realize_projectiles));
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use projectiles::BoltSpec;

	use crate::request::{ProjectileSpec, SpawnProjectile};

	#[test]
	fn presentation_attaches_cached_mesh_from_the_visual_spec() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Assets<Mesh>>();
		world.init_resource::<Assets<StandardMaterial>>();
		world.init_resource::<ProjectileVisualCache>();
		let projectile = world
			.run_system_once(|mut commands: Commands| {
				SpawnProjectile::launch(
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
		assert!(world.get::<Mesh3d>(projectile).is_none());
		world
			.run_system_once(attach_projectile_visuals)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get::<Mesh3d>(projectile).is_some());
		assert!(world.get::<MeshMaterial3d<StandardMaterial>>(projectile).is_some());
		assert!(world.get::<ProjectileVisual>(projectile).is_some());
		Ok(())
	}

	#[test]
	fn register_projectile_presentation_is_idempotent() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		register_projectile_presentation(&mut app);
		register_projectile_presentation(&mut app);
		assert!(app.world().contains_resource::<ProjectileVisualCache>());
	}
}
