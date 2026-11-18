//! Encounter / Ward / Enraged spawn entry points (shared assembly).

use crate::anchors::{BikanwardaAnchors, BikanwardaChainAnchor, CHAIN_ANCHOR_NAMES};
use crate::colliders::motor_collider;
use crate::recipe::BikanwardaActorRecipe;
use crate::spec::BikanwardaSpec;
use avian3d::prelude::RigidBody;
use bevy::prelude::*;
use characters::{
	components::CharacterComponents, spawn_fixed_character_assembly, CharacterRecipe,
};
use lod_avian::PhysicsInteractionLayer;

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
	let root_transform =
		Transform::from_translation(transform.translation).with_rotation(transform.rotation);
	let root = commands
		.spawn((
			Name::new("bikanwarda"),
			root_transform,
			GlobalTransform::default(),
			Visibility::default(),
			BikanwardaRoot,
			marker,
			spec.clone(),
		))
		.id();

	// Collider shape sized to match the visible scaled boss (body Transform stays unit-scale)
	let scaled_capsule = actor.scaled_locomotion_capsule();
	spawn_physics(commands, root, scaled_capsule);

	// Anchors use scaled offsets (children of unit-scale root, positions match scaled visual)
	let scaled_capsule = actor.scaled_locomotion_capsule();
	spawn_chain_anchors(commands, root, scaled_capsule);

	// Visual: scaled child with all meshes (inherits rotation from root, only needs scale)
	let visual_transform = Transform::from_scale(Vec3::splat(scale));
	let visual = spawn_fixed_character_assembly(commands, &actor, visual_transform);
	commands.entity(visual).insert((ChildOf(root), Name::new("bikanwarda-visual")));

	root
}

fn spawn_physics(commands: &mut Commands, root: Entity, capsule: characters::LocomotionCapsule) {
	commands.entity(root).insert((
		RigidBody::Static,
		motor_collider(capsule),
		PhysicsInteractionLayer::fixed_layers(),
	));
}

fn spawn_chain_anchors(
	commands: &mut Commands,
	root: Entity,
	capsule: characters::LocomotionCapsule,
) {
	let anchors = BikanwardaAnchors::for_capsule(capsule);
	for name in CHAIN_ANCHOR_NAMES {
		let local = anchors.get(name).copied().unwrap_or(Transform::IDENTITY);
		commands.spawn((Name::new(name), ChildOf(root), local, BikanwardaChainAnchor { name }));
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

		let spawn_rotation = Quat::from_euler(EulerRot::YXZ, 0.7, 0.25, 0.1);
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TransformPlugin));
		app.add_systems(Startup, move |mut commands: Commands| {
			let spec = BikanwardaSpec::encounter();
			let transform = Transform::from_rotation(spawn_rotation);
			spawn_bikanwarda_encounter(&mut commands, &spec, transform);
		});
		app.update();

		let mut roots =
			app.world_mut().query_filtered::<(Entity, &Transform), With<BikanwardaRoot>>();
		let (root_entity, root_rotation) = {
			let (entity, transform) =
				roots.single(app.world()).expect("expected one BikanwardaRoot");
			(entity, transform.rotation)
		};

		let mut visuals = app.world_mut().query::<(Entity, &Name, &Transform, &ChildOf)>();
		let (visual_entity, visual_local_rotation) = visuals
			.iter(app.world())
			.find(|(_, name, _, parent)| {
				name.as_str() == "bikanwarda-visual" && parent.parent() == root_entity
			})
			.map(|(entity, _, transform, _)| (entity, transform.rotation))
			.expect("expected the bikanwarda-visual child");

		let local_angle = visual_local_rotation.angle_between(Quat::IDENTITY);
		assert!(
			local_angle < 0.01,
			"Visual child local rotation should be identity (inherits from root), got angle {local_angle} rad"
		);

		let visual_global = app
			.world()
			.get::<GlobalTransform>(visual_entity)
			.expect("bikanwarda-visual should have GlobalTransform");
		let (_, visual_global_rotation, _) = visual_global.to_scale_rotation_translation();
		let global_angle = visual_global_rotation.angle_between(spawn_rotation);
		assert!(
			global_angle < 0.01,
			"Visual global rotation should match the spawn rotation, got {global_angle} rad from expected"
		);

		let root_angle = root_rotation.angle_between(spawn_rotation);
		assert!(
			root_angle < 0.01,
			"Root rotation should match the spawn rotation, got {root_angle} rad from expected"
		);
	}
}
