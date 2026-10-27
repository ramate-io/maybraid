//! Thrown-body mesh, material, and drop fulfillment.

use bevy::prelude::*;
use character_items::{GrenadeSpec, GrenadeStats};

use crate::material::{grenade_material, GrenadeMaterial, GrenadeMaterialPlugin};
use crate::runtime::{tick_grenade_fuses, GrenadeEffect, ThrownGrenade, ThrownGrenadeSpawn};
use crate::sound;

pub fn grenade_mesh() -> Mesh {
	Sphere::new(0.5).mesh().ico(3).expect("grenade sphere")
}

impl ThrownGrenadeSpawn {
	/// Simulation body plus mesh. Production throw / drop go through
	/// `maybraid-spawns-grenades`; this keeps unit tests self-contained.
	pub fn realize(
		self,
		commands: &mut Commands,
		meshes: &mut Assets<Mesh>,
		materials: &mut Assets<GrenadeMaterial>,
	) -> Entity {
		let entity = self.realize_body(commands);
		attach_grenade_visual(commands, meshes, materials, entity);
		commands
			.entity(entity)
			.insert(sound::GrenadeBounceClock { next_at: 0.0, touching: Vec::new() });
		entity
	}
}

fn attach_grenade_visual(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<GrenadeMaterial>,
	entity: Entity,
) {
	commands.entity(entity).insert((
		Mesh3d(meshes.add(grenade_mesh())),
		MeshMaterial3d(materials.add(grenade_material())),
		sound::GrenadeBounceClock { next_at: 0.0, touching: Vec::new() },
	));
}

pub fn spawn_thrown_grenade(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<GrenadeMaterial>,
	origin: Vec3,
	velocity: Vec3,
	spec: GrenadeSpec,
	stats: GrenadeStats,
	effect: GrenadeEffect,
	source: Entity,
	fuse_remaining: f32,
) -> Entity {
	ThrownGrenadeSpawn::from_throw(origin, velocity, spec, stats, effect, source, fuse_remaining)
		.realize(commands, meshes, materials)
}

/// Attach mesh / bounce audio after the simulation interpreter creates the body.
pub fn spawn_thrown_grenades(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<GrenadeMaterial>>,
	added: Query<Entity, (Added<ThrownGrenade>, Without<Mesh3d>)>,
) {
	for entity in &added {
		attach_grenade_visual(&mut commands, &mut meshes, &mut materials, entity);
	}
}

pub fn spawn_dropped_live_grenades(
	commands: Commands,
	meshes: ResMut<Assets<Mesh>>,
	materials: ResMut<Assets<GrenadeMaterial>>,
	added: Query<Entity, (Added<ThrownGrenade>, Without<Mesh3d>)>,
) {
	spawn_thrown_grenades(commands, meshes, materials, added);
}

#[derive(Resource, Default)]
struct GrenadePresentation;

/// Mesh / material / audio for thrown bodies. Fulfills throw and drop requests.
pub fn register_grenade_presentation(app: &mut App) {
	if app.world().contains_resource::<GrenadePresentation>() {
		return;
	}
	if !app.is_plugin_added::<GrenadeMaterialPlugin>() {
		app.add_plugins(GrenadeMaterialPlugin);
	}
	sound::configure_grenade_audio(app);
	app.init_resource::<GrenadePresentation>().add_systems(
		Update,
		(spawn_thrown_grenades, spawn_dropped_live_grenades)
			.chain()
			.after(tick_grenade_fuses),
	);
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use character_items::GrenadeSpec;

	use super::*;
	use crate::runtime::GrenadeFuse;

	#[test]
	fn thrown_body_keeps_the_requested_fuse() {
		let mut world = World::new();
		world.init_resource::<Assets<Mesh>>();
		world.init_resource::<Assets<GrenadeMaterial>>();
		let stats = GrenadeStats::standard();
		let id = world
			.run_system_once(
				move |mut commands: Commands,
				      mut meshes: ResMut<Assets<Mesh>>,
				      mut materials: ResMut<Assets<GrenadeMaterial>>| {
					ThrownGrenadeSpawn::from_throw(
						Vec3::ZERO,
						Vec3::Z,
						GrenadeSpec::standard(),
						stats,
						GrenadeEffect::from_stats(&stats),
						Entity::PLACEHOLDER,
						1.25,
					)
					.realize(&mut commands, &mut meshes, &mut materials)
				},
			)
			.expect("spawn");
		let fuse = world.get::<GrenadeFuse>(id).expect("fuse");
		assert!((fuse.remaining - 1.25).abs() < 1e-4);
	}
}
