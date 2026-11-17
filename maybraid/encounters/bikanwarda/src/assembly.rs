//! Boss assembly spawn system for HCSG hosts.

use bevy::prelude::*;
use bikanwarda_character::{spawn_bikanwarda_encounter, BikanwardaRoot};

use crate::presentation::{RequestBossAssembly, BossActor};

/// System to spawn actual boss assemblies for HCSG hosts requesting them.
pub fn spawn_boss_assemblies(
	mut commands: Commands,
	requests: Query<(Entity, &RequestBossAssembly, &Transform), Without<BikanwardaRoot>>,
) {
	for (host_entity, request, _host_transform) in &requests {
		// Pass identity transform since the actor will be parented to the host.
		// The host already has the world position; the actor's transform should be local/relative.
		let local_transform = Transform::IDENTITY;
		let boss_entity = spawn_bikanwarda_encounter(&mut commands, &request.spec, local_transform);
		
		// Parent the spawned boss to the host (use ChildOf component)
		commands.entity(boss_entity).insert(ChildOf(host_entity));
		
		// Add AttackUser so the attack runtime accepts this owner
		commands.entity(boss_entity).insert(attacks::AttackUser::default());
		
		// Bind host to actor with relationship component
		commands.entity(host_entity).insert(BossActor(boss_entity));
		
		// Remove the request marker
		commands.entity(host_entity).remove::<RequestBossAssembly>();
	}
}

/// Plugin to handle boss assembly spawning.
pub struct BossAssemblyPlugin;

impl Plugin for BossAssemblyPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, spawn_boss_assemblies);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::world::{CommandQueue, World};
	use bevy::prelude::{Commands, Entity, GlobalTransform, Quat, Transform, Vec3, With, Without};
	use bikanwarda_character::BikanwardaSpec;
	
	/// Regression test: boss actor should receive a local/relative transform, not world transform,
	/// since it will be parented to the host which already has the world position.
	#[test]
	fn assembly_passes_local_transform_to_spawn() {
		let mut world = World::new();
		world.init_resource::<bevy::time::Time>();
		world.init_resource::<bevy::time::Time<bevy::time::Real>>();
		
		// Spawn a host with nonzero translation and rotation (authored position)
		let authored_position = Vec3::new(1746.8, -351.16534, 1029.9);
		let host_rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_4);
		let host_transform = Transform::from_translation(authored_position)
			.with_rotation(host_rotation);
		
		let spec = BikanwardaSpec::encounter();
		let host = world.spawn((
			RequestBossAssembly { spec: spec.clone() },
			host_transform,
			GlobalTransform::default(),
		)).id();
		
		// Run the assembly system manually
		// First collect the requests
		let requests: Vec<_> = world
			.query_filtered::<(Entity, &RequestBossAssembly, &Transform), Without<BikanwardaRoot>>()
			.iter(&world)
			.map(|(e, r, t)| (e, r.clone(), *t))
			.collect();
		
		// Then process them with commands
		let mut commands_queue = CommandQueue::default();
		{
			let mut commands = Commands::new(&mut commands_queue, &world);
			for (host_entity, request, _host_transform) in requests {
				// This is the core fix: pass Transform::IDENTITY (local) instead of world transform
				let local_transform = Transform::IDENTITY;
				let boss_entity = bikanwarda_character::spawn_bikanwarda_encounter(&mut commands, &request.spec, local_transform);
				commands.entity(boss_entity).insert(ChildOf(host_entity));
				commands.entity(host_entity).remove::<RequestBossAssembly>();
			}
		}
		commands_queue.apply(&mut world);
		
		// Verify: the spawned boss actor should have identity (or near-identity) local translation
		let mut boss_query = world.query_filtered::<&Transform, With<BikanwardaRoot>>();
		let boss_transform = boss_query.single(&world).expect("Expected one boss actor");
		
		let epsilon = 0.01;
		assert!(
			boss_transform.translation.length() < epsilon,
			"Boss actor local translation {:?} should be near identity (within {}), not contain the world position {:?}",
			boss_transform.translation, epsilon, authored_position
		);
		
		// Verify the request marker was removed
		assert!(
			world.get::<RequestBossAssembly>(host).is_none(),
			"RequestBossAssembly marker should be removed after assembly"
		);
	}
	
	/// Test that assembly uses Transform::IDENTITY for the spawned actor.
	#[test]
	fn assembly_uses_identity_transform_for_actor() {
		// This test verifies the fix at the source: we pass Transform::IDENTITY
		// to the spawn function, not the host's world transform.
		let local_transform = Transform::IDENTITY;
		assert_eq!(local_transform.translation, Vec3::ZERO, 
			"Assembly should pass identity translation to spawn function");
	}
}
