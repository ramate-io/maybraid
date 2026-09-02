//! Spawn the capsule and attach a character visual.

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value};
use crozon_characters::{character_bounds, CharacterComponents, CharacterRoot, ComponentsOnly};
use lod::gen::LodScene;
use lod::lod_ref::LodRef;

use crate::body::spawn_character_controller;
use crate::identity::{
	CameraFollow, Npc, Player, PlayerCameraAim, PlayerCapsule, PlayerLook, PlayerVisual,
	PlayerYawOwner,
};

pub const CAPSULE_RADIUS: f32 = 0.4;
pub const CAPSULE_LENGTH: f32 = 1.0;

pub fn capsule_spawn_height() -> f32 {
	CAPSULE_RADIUS + CAPSULE_LENGTH * 0.5 + 0.15
}

pub fn spawn_player(commands: &mut Commands) -> Entity {
	let player = spawn_character_controller(commands, Vec3::new(0.0, capsule_spawn_height(), 0.0));
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

pub fn spawn_npc(commands: &mut Commands, translation: Vec3, look: PlayerLook) -> Entity {
	let npc = spawn_character_controller(commands, translation);
	commands.entity(npc).insert((Name::new("Npc"), Npc, look, PlayerYawOwner::Wish));
	npc
}

pub fn spawn_player_capsule_mesh(
	commands: &mut Commands,
	body: Entity,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	spawn_capsule_mesh(commands, body, "PlayerCapsule", meshes, materials);
}

pub fn spawn_capsule_mesh(
	commands: &mut Commands,
	body: Entity,
	name: &'static str,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	commands.spawn((
		Name::new(name),
		PlayerCapsule,
		ChildOf(body),
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
	spawn_character_visual(commands, player, recipe, facing, "player-visual", PlayerVisual)
}

pub fn spawn_npc_visual<
	C: CharacterComponents + Clone + Default + Unpin + Send + Sync + 'static,
>(
	commands: &mut Commands,
	npc: Entity,
	recipe: C,
	facing: Quat,
) -> Entity {
	spawn_character_visual(commands, npc, recipe, facing, "npc-visual", ())
}

fn spawn_character_visual<
	C: CharacterComponents + Clone + Default + Unpin + Send + Sync + 'static,
>(
	commands: &mut Commands,
	body: Entity,
	recipe: C,
	facing: Quat,
	name: &'static str,
	extra: impl Bundle,
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
	commands
		.entity(visual)
		.insert((ChildOf(body), PlayerYawOwner::Wish, Name::new(name), extra));
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

pub fn spawn_npc_with_hidden_capsule(
	commands: &mut Commands,
	translation: Vec3,
	look: PlayerLook,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) -> Entity {
	let npc = spawn_npc(commands, translation, look);
	spawn_capsule_mesh(commands, npc, "NpcCapsule", meshes, materials);
	npc
}

pub fn needs_player_visual(
	players: Query<Entity, With<Player>>,
	visuals: Query<&ChildOf, With<PlayerVisual>>,
) -> Option<Entity> {
	needs_body_visual(players, visuals)
}

pub fn needs_npc_visual(
	npcs: Query<Entity, With<Npc>>,
	visuals: Query<&ChildOf, With<CharacterRoot>>,
) -> Option<Entity> {
	needs_body_visual(npcs, visuals)
}

fn needs_body_visual<BodyFilter: QueryFilter, VisualFilter: QueryFilter>(
	bodies: Query<Entity, BodyFilter>,
	visuals: Query<&ChildOf, VisualFilter>,
) -> Option<Entity> {
	let body = bodies.single().ok()?;
	let has_visual = visuals.iter().any(|child| child.parent() == body);
	(!has_visual).then_some(body)
}
