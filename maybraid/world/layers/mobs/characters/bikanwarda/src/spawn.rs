//! Encounter / Ward / Enraged spawn entry points (shared assembly).

use avian3d::prelude::RigidBody;
use bevy::prelude::*;
use characters::{components::CharacterComponents, spawn_fixed_character_assembly, CharacterRecipe};
use lod_avian::PhysicsInteractionLayer;
use crate::anchors::{BikanwardaAnchors, BikanwardaChainAnchor, CHAIN_ANCHOR_NAMES};
use crate::colliders::motor_collider;
use crate::recipe::{BikanwardaActorRecipe};
use crate::spec::BikanwardaSpec;

/// Domain marker on the boss assembly root.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct BikanwardaRoot;

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct BikanwardaEncounter;

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct BikanwardaWard;

#[derive(Component, Debug, Clone, Copy, Default)]
pub struct BikanwardaEnraged;

pub fn spawn_bikanwarda_encounter(
	commands: &mut Commands,
	spec: &BikanwardaSpec,
	transform: Transform,
) -> Entity {
	spawn_bikanwarda(commands, spec, transform, BikanwardaEncounter)
}

pub fn spawn_bikanwarda_ward(
	commands: &mut Commands,
	spec: &BikanwardaSpec,
	transform: Transform,
) -> Entity {
	spawn_bikanwarda(commands, spec, transform, BikanwardaWard)
}

pub fn spawn_bikanwarda_enraged(
	commands: &mut Commands,
	spec: &BikanwardaSpec,
	transform: Transform,
) -> Entity {
	spawn_bikanwarda(commands, spec, transform, BikanwardaEnraged)
}

fn spawn_bikanwarda<M: Component>(
	commands: &mut Commands,
	spec: &BikanwardaSpec,
	transform: Transform,
	marker: M,
) -> Entity {
	let recipe = BikanwardaActorRecipe::from_spec(spec.clone());
	let actor = recipe.components();
	let scale = spec.uniform_scale();
	
	// Root: unit-scale body with physics/collider (strip any scale from input transform)
	let root_transform = Transform::from_translation(transform.translation)
		.with_rotation(transform.rotation);
	let root = commands.spawn((
		Name::new("bikanwarda"),
		root_transform,
		GlobalTransform::default(),
		Visibility::default(),
		BikanwardaRoot,
		marker,
		spec.clone(),
	)).id();
	
	// Collider shape sized to match the visible scaled boss (body Transform stays unit-scale)
	let scaled_capsule = actor.scaled_locomotion_capsule();
	spawn_physics(commands, root, scaled_capsule);
	
	// Anchors use scaled offsets (children of unit-scale root, positions match scaled visual)
	let scaled_capsule = actor.scaled_locomotion_capsule();
	spawn_chain_anchors(commands, root, scaled_capsule);
	
	// Visual: scaled child with all meshes (inherits rotation from root, only needs scale)
	let visual_transform = Transform::from_scale(Vec3::splat(scale));
	let visual = spawn_fixed_character_assembly(commands, &actor, visual_transform);
	commands.entity(visual).insert((
		ChildOf(root),
		Name::new("bikanwarda-visual"),
	));
	
	root
}

fn spawn_physics(commands: &mut Commands, root: Entity, capsule: characters::LocomotionCapsule) {
	commands
		.entity(root)
		.insert((RigidBody::Static, motor_collider(capsule), PhysicsInteractionLayer::fixed_layers()));
}

fn spawn_chain_anchors(commands: &mut Commands, root: Entity, capsule: characters::LocomotionCapsule) {
	let anchors = BikanwardaAnchors::for_capsule(capsule);
	for name in CHAIN_ANCHOR_NAMES {
		let local = anchors.get(name).copied().unwrap_or(Transform::IDENTITY);
		commands.spawn((
			Name::new(name),
			ChildOf(root),
			local,
			BikanwardaChainAnchor { name },
		));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn encounter_recipe_matches_spec_form() {
		let spec = BikanwardaSpec::encounter();
		let recipe = BikanwardaActorRecipe::from_spec(spec.clone());
		assert_eq!(recipe.spec.form, spec.form);
	}

	/// Regression test for #1619 comment 6986265628: visual child should inherit
	/// root rotation, not double-apply it.
	///
	/// Before the fix, both root and visual child were assigned the input rotation,
	/// causing double-rotation for parented visuals.
	#[test]
	fn visual_child_inherits_rotation_from_root() {
		use bevy::app::{App, Startup};
		use bevy::prelude::*;

		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app.add_systems(Startup, |mut commands: Commands| {
			let spec = BikanwardaSpec::encounter();
			// Spawn with non-identity rotation (90 degrees around Y axis)
			let rotation = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
			let transform = Transform::from_rotation(rotation);
			spawn_bikanwarda_encounter(&mut commands, &spec, transform);
		});
		app.update();

		// Find root and visual entities
		let mut roots = app.world_mut().query_filtered::<(Entity, &Transform), With<BikanwardaRoot>>();
		let (root_entity, root_transform) = roots.single(app.world());
		let root_rotation = root_transform.rotation;

		let mut visuals = app
			.world_mut()
			.query_filtered::<(Entity, &Transform, &ChildOf), (With<Name>, Without<BikanwardaRoot>)>();
		let visual = visuals
			.iter(app.world())
			.find(|(_, _, parent)| parent.parent() == root_entity)
			.map(|(e, t, _)| (e, t.rotation));

		// Visual child should have identity rotation in local space (inherits from root)
		if let Some((visual_entity, visual_local_rotation)) = visual {
			let angle_diff = visual_local_rotation.angle_between(Quat::IDENTITY);
			assert!(
				angle_diff < 0.01,
				"Visual child local rotation should be identity (inherits from root), got angle {} rad",
				angle_diff
			);

			// Verify global rotation matches input
			let visual_global = app.world().get::<GlobalTransform>(visual_entity).unwrap();
			let (_, visual_global_rotation, _) = visual_global.to_scale_rotation_translation();
			let expected_angle = std::f32::consts::FRAC_PI_2;
			let actual_angle = visual_global_rotation.angle_between(Quat::IDENTITY);
			assert!(
				(actual_angle - expected_angle).abs() < 0.01,
				"Visual global rotation should match input (π/2 rad), got {} rad",
				actual_angle
			);
		} else {
			panic!("Visual child not found");
		}

		// Root should have the input rotation
		let expected_angle = std::f32::consts::FRAC_PI_2;
		let actual_angle = root_rotation.angle_between(Quat::IDENTITY);
		assert!(
			(actual_angle - expected_angle).abs() < 0.01,
			"Root rotation should match input (π/2 rad), got {} rad",
			actual_angle
		);
	}
}
