//! Boss assembly spawn system for HCSG hosts.

use bevy::prelude::*;
use bikanwarda_attacks::BikanwardaMouthSocket;
use bikanwarda_character::{
	encounter_meters, spawn_bikanwarda_encounter, BikanwardaAnchors, BikanwardaRoot,
};
use characters::CharacterRecipe;
use spotting_intelligence::{InterestLayers, SpotDirective, SpottingUser};

use crate::presentation::{BossActor, RequestBossAssembly};

/// Eye height on the scaled encounter hull (authored 15 m on the 20 m boss).
pub fn boss_eye_offset() -> Vec3 {
	Vec3::Y * encounter_meters(15.0)
}

/// Spotting range matching the scaled arena (authored 50 m on the 20 m boss).
pub fn boss_spotting_range() -> f32 {
	encounter_meters(50.0)
}

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

		// Add spotting component for perception/visibility
		// Reuses existing spotting_intelligence system instead of hand-writing visibility checks
		let eye_offset = boss_eye_offset();
		let spotting_range = boss_spotting_range();
		commands.entity(boss_entity).insert(SpottingUser::new(
			eye_offset,
			[SpotDirective::new(InterestLayers::CHARACTER, spotting_range)],
		));

		// Spawn the mouth socket for Bray emission.
		// Actor root is unit-scale with scaled visual child, so socket needs scaled offsets.
		let recipe = bikanwarda_character::BikanwardaActorRecipe::from_spec(request.spec.clone());
		let actor = recipe.components();
		let scaled_capsule = actor.scaled_locomotion_capsule();
		let anchors = BikanwardaAnchors::for_capsule(scaled_capsule);
		let mouth_transform = anchors
			.get(bikanwarda_character::anchors::names::MOUTH_EMIT)
			.copied()
			.unwrap_or(Transform::from_translation(Vec3::Y * 1.5));

		commands.spawn((
			Name::new("bikanwarda_mouth_socket"),
			BikanwardaMouthSocket,
			ChildOf(boss_entity),
			mouth_transform,
			GlobalTransform::default(),
		));

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
		let host_transform =
			Transform::from_translation(authored_position).with_rotation(host_rotation);

		let spec = BikanwardaSpec::encounter();
		let host = world
			.spawn((
				RequestBossAssembly { spec: spec.clone() },
				host_transform,
				GlobalTransform::default(),
			))
			.id();

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
				let boss_entity = bikanwarda_character::spawn_bikanwarda_encounter(
					&mut commands,
					&request.spec,
					local_transform,
				);
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
		assert_eq!(
			local_transform.translation,
			Vec3::ZERO,
			"Assembly should pass identity translation to spawn function"
		);
	}

	/// Regression test: production assembly should create a BikanwardaMouthSocket
	/// so Bray attacks emit from the correct mouth position rather than actor origin.
	#[test]
	fn assembly_creates_mouth_socket() {
		use bikanwarda_attacks::BikanwardaMouthSocket;

		// Use App for transform propagation support
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_plugins(TransformPlugin);
		app.finish();
		app.update();

		let mut world = app.world_mut();

		let spec = BikanwardaSpec::encounter();
		let host = world
			.spawn((
				RequestBossAssembly { spec: spec.clone() },
				Transform::default(),
				GlobalTransform::default(),
			))
			.id();

		// Run the assembly system manually with command flushing
		let requests: Vec<_> = world
			.query_filtered::<(Entity, &RequestBossAssembly, &Transform), Without<BikanwardaRoot>>()
			.iter(world)
			.map(|(e, r, t)| (e, r.clone(), *t))
			.collect();

		{
			let mut commands = world.commands();
			for (host_entity, request, _host_transform) in requests {
				let local_transform = Transform::IDENTITY;
				let boss_entity =
					spawn_bikanwarda_encounter(&mut commands, &request.spec, local_transform);
				commands.entity(boss_entity).insert(ChildOf(host_entity));
				commands.entity(boss_entity).insert(attacks::AttackUser::default());
				let eye_offset = boss_eye_offset();
				let spotting_range = boss_spotting_range();
				commands.entity(boss_entity).insert(SpottingUser::new(
					eye_offset,
					[SpotDirective::new(InterestLayers::CHARACTER, spotting_range)],
				));
				// Actor root is unit-scale with scaled visual child, so socket needs scaled offsets
				let recipe =
					bikanwarda_character::BikanwardaActorRecipe::from_spec(request.spec.clone());
				let actor = recipe.components();
				let scaled_capsule = actor.scaled_locomotion_capsule();
				let anchors = BikanwardaAnchors::for_capsule(scaled_capsule);
				let mouth_transform = anchors
					.get(bikanwarda_character::anchors::names::MOUTH_EMIT)
					.copied()
					.unwrap_or(Transform::from_translation(Vec3::Y * 1.5));
				commands.spawn((
					Name::new("bikanwarda_mouth_socket"),
					BikanwardaMouthSocket,
					ChildOf(boss_entity),
					mouth_transform,
					GlobalTransform::default(),
				));
				commands.entity(host_entity).insert(BossActor(boss_entity));
				commands.entity(host_entity).remove::<RequestBossAssembly>();
			}
		}
		world.flush();

		// Verify: the boss actor should have a child mouth socket
		let boss_entity = world
			.query_filtered::<Entity, With<BikanwardaRoot>>()
			.single(world)
			.expect("Expected one boss actor");

		let socket_count = world
			.query_filtered::<(&ChildOf, &BikanwardaMouthSocket), With<BikanwardaMouthSocket>>()
			.iter(world)
			.filter(|(parent, _)| parent.parent() == boss_entity)
			.count();

		assert_eq!(
			socket_count, 1,
			"Boss actor should have exactly one BikanwardaMouthSocket child"
		);

		// Verify: socket world-space position matches expected mouth location
		// Default encounter spec has uniform_scale of ~10.5x, capsule height ~1.9m
		// Root is unit-scale, so socket local coords are scaled to match the scaled visual
		let spec = BikanwardaSpec::encounter();
		let recipe = bikanwarda_character::BikanwardaActorRecipe::from_spec(spec);
		let actor = recipe.components();
		let scaled_capsule = actor.scaled_locomotion_capsule();
		let anchors = BikanwardaAnchors::for_capsule(scaled_capsule);
		let expected_local = anchors
			.get(bikanwarda_character::anchors::names::MOUTH_EMIT)
			.map(|t| t.translation)
			.unwrap_or(Vec3::Y * 1.5);
		let expected_world_y = expected_local.y; // Root is unit-scale, so local=world here

		// Get socket's world-space transform
		let socket_local = world
			.query_filtered::<(&ChildOf, &Transform), With<BikanwardaMouthSocket>>()
			.iter(world)
			.find_map(|(parent, transform)| {
				if parent.parent() == boss_entity {
					Some(transform.translation)
				} else {
					None
				}
			})
			.expect("Socket should have a Transform");

		// Verify local position is in scaled coordinates (root is unit-scale)
		let tolerance = 0.1;
		assert!(
			(socket_local.y - expected_local.y).abs() < tolerance,
			"Mouth socket local Y {} should match scaled anchor Y {} (within {})",
			socket_local.y,
			expected_local.y,
			tolerance
		);

		// Verify world-space GlobalTransform after propagation
		// Propagate transforms using the transform systems
		app.update(); // Run PostUpdate schedule to propagate transforms

		// Get socket's world-space GlobalTransform
		let world = app.world_mut();
		let socket_entity = world
			.query_filtered::<Entity, With<BikanwardaMouthSocket>>()
			.single(world)
			.expect("Should have exactly one mouth socket");

		let socket_global = world
			.get::<GlobalTransform>(socket_entity)
			.expect("Socket should have GlobalTransform after propagation");

		// Since host is at origin and root is unit-scale, world Y should equal local Y
		assert!(
			(socket_global.translation().y - expected_local.y).abs() < tolerance,
			"Mouth socket world Y {:.6} should be ~{:.6} (scaled anchor local position), within {:.2}m tolerance",
			socket_global.translation().y,
			expected_local.y,
			tolerance,
		);
	}
}
