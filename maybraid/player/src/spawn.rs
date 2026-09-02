//! Spawn the capsule and attach a character visual.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use crozon_characters::{character_bounds, CharacterComponents, ComponentsOnly};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::body::spawn_character_controller;
use crate::identity::{
	CameraFollow, Player, PlayerCameraAim, PlayerCapsule, PlayerLook, PlayerVisual, PlayerYawOwner,
};

pub const CAPSULE_RADIUS: f32 = 0.4;
pub const CAPSULE_LENGTH: f32 = 1.0;

pub fn spawn_player(commands: &mut Commands) -> Entity {
	let spawn = Vec3::new(0.0, CAPSULE_RADIUS + CAPSULE_LENGTH * 0.5 + 0.15, 0.0);
	let player = spawn_character_controller(commands, spawn);
	commands.entity(player).insert((
		Name::new("Player"),
		Player,
		CameraFollow,
		PlayerLook::default(),
		PlayerCameraAim::default(),
		PlayerYawOwner::Wish,
	));
	player
}

pub fn spawn_player_capsule_mesh(
	commands: &mut Commands,
	player: Entity,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	commands.spawn((
		Name::new("PlayerCapsule"),
		PlayerCapsule,
		ChildOf(player),
		Visibility::Hidden,
		Mesh3d(meshes.add(Capsule3d::new(CAPSULE_RADIUS, CAPSULE_LENGTH))),
		MeshMaterial3d(materials.add(Color::srgb(0.85, 0.55, 0.35))),
	));
}

pub fn spawn_player_visual<
	C: CharacterComponents + Clone + Default + Unpin + Send + Sync + 'static,
>(
	commands: &mut Commands,
	player: Entity,
	recipe: C,
	facing: Quat,
) -> Entity {
	let host = ComponentsOnly(recipe);
	let bounds = character_bounds(&host.0);
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let visual = commands
		.spawn_scene((
			host.host(&lod_ref),
			bsn! {
				template_value(Transform::from_rotation(facing))
			},
		))
		.id();
	commands.entity(visual).insert((
		ChildOf(player),
		PlayerVisual,
		PlayerYawOwner::Wish,
		Name::new("player-visual"),
	));
	visual
}

pub fn spawn_player_with_hidden_capsule(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) -> Entity {
	let player = spawn_player(commands);
	spawn_player_capsule_mesh(commands, player, meshes, materials);
	player
}

pub fn needs_player_visual(
	players: Query<Entity, With<Player>>,
	visuals: Query<&ChildOf, With<PlayerVisual>>,
) -> Option<Entity> {
	let player = players.single().ok()?;
	let has_visual = visuals.iter().any(|child| child.parent() == player);
	(!has_visual).then_some(player)
}
