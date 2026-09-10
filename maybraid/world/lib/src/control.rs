//! Apply [`CharacterIntent`] to the vegetation capsule / camera-relative wish.

use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	MoveWish, MovementAction, Player, PlayerSpawnXz, PlaygroundMode,
};
use durham_terrain_models::{
	terrain_collider_covers_xz, CascadeChunk, TerrainCellLayout, TerrainEntryStore,
	TerrainTrimeshCollider,
};
use game_commands::command::{CommandConsoleOutput, TextEntryFocus};
use maybraid_character_controller::CharacterIntent;
use maybraid_sky::SkyDome;
use player::MotorTraction;
use player_camera::CameraController;

/// When `false`, world movement / POV intents are ignored (menus, pause overlay).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGameplayEnabled(pub bool);

impl Default for WorldGameplayEnabled {
	fn default() -> Self {
		Self(true)
	}
}

/// Sky, world player, and fog. Off on menu shells so navy clear is the preview backdrop.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldSceneryVisible(pub bool);

impl Default for WorldSceneryVisible {
	fn default() -> Self {
		Self(true)
	}
}

/// Local spawn collider + composed height are ready for Discovery drop-in.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct WorldSurfaceReady(pub bool);

pub(crate) fn update_world_surface_ready(
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	spawn: Res<PlayerSpawnXz>,
	players: Query<&Transform, With<Player>>,
	colliders: Query<&CascadeChunk, With<TerrainTrimeshCollider>>,
	mut ready: ResMut<WorldSurfaceReady>,
) {
	let xz = discovery_xz(&spawn, &players, &layout);
	let at = Vec3::new(xz.x, 0.0, xz.y);
	ready.0 = terrain_collider_covers_xz(at, colliders.iter())
		&& store.composed_height_at(&layout, xz.x, xz.y).is_some();
}

fn discovery_xz(
	spawn: &PlayerSpawnXz,
	players: &Query<&Transform, With<Player>>,
	layout: &TerrainCellLayout,
) -> Vec2 {
	if let Some(xz) = spawn.0 {
		return xz;
	}
	if let Ok(player) = players.single() {
		return player.translation.xz();
	}
	layout.region_center_xz().xz()
}

pub(crate) fn apply_intents_to_movement(
	mode: Res<PlaygroundMode>,
	text_focus: Res<TextEntryFocus>,
	gameplay: Res<WorldGameplayEnabled>,
	mut intents: MessageReader<CharacterIntent>,
	cameras: Query<&CameraController, With<Camera3d>>,
	mut wishes: Query<&mut MoveWish, With<Player>>,
	mut movement: MessageWriter<MovementAction>,
) {
	if !gameplay.0 || *mode != PlaygroundMode::Character || text_focus.0 {
		for _ in intents.read() {}
		for mut wish in &mut wishes {
			wish.0 = Vec3::ZERO;
		}
		return;
	}

	let mut move_stick = Vec2::ZERO;
	let mut jump = false;
	for intent in intents.read() {
		match *intent {
			CharacterIntent::Move(value) => move_stick = value,
			CharacterIntent::Jump => jump = true,
			_ => {}
		}
	}

	let wish_dir = if move_stick != Vec2::ZERO {
		if let Ok(camera) = cameras.single() {
			let yaw = Quat::from_axis_angle(Vec3::Y, camera.yaw);
			let forward = yaw * -Vec3::Z;
			let right_dir = yaw * Vec3::X;
			(right_dir * move_stick.x + forward * move_stick.y).normalize_or_zero()
		} else {
			Vec3::ZERO
		}
	} else {
		Vec3::ZERO
	};
	for mut wish in &mut wishes {
		wish.0 = wish_dir;
	}

	if move_stick != Vec2::ZERO {
		movement.write(MovementAction::Move(move_stick));
	}
	if jump {
		movement.write(MovementAction::Jump);
	}
}

pub(crate) fn echo_character_intents(
	mut intents: MessageReader<CharacterIntent>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	let mut parts = Vec::new();
	for intent in intents.read() {
		parts.push(match *intent {
			CharacterIntent::Move(value) => format!("move=({:.2},{:.2})", value.x, value.y),
			CharacterIntent::Look(value) => format!("look=({:.2},{:.2})", value.x, value.y),
			CharacterIntent::Focus(value) => format!("focus={value:.2}"),
			CharacterIntent::Ads(value) => format!("ads={value:.2}"),
			CharacterIntent::UseItem(value) => format!("use={value:.2}"),
			other => other.label().to_string(),
		});
	}
	if !parts.is_empty() {
		console.0 = parts.join(" ");
	}
}

fn world_distance_fog() -> DistanceFog {
	DistanceFog {
		color: Color::srgba(0.55, 0.65, 0.72, 1.0),
		directional_light_color: Color::srgba(1.0, 0.92, 0.78, 0.35),
		directional_light_exponent: 24.0,
		falloff: FogFalloff::Linear { start: 700.0, end: 4500.0 },
	}
}

pub(crate) fn sync_world_scenery(
	visible: Res<WorldSceneryVisible>,
	mut commands: Commands,
	mut sky: Query<&mut Visibility, (With<SkyDome>, Without<Player>)>,
	mut players: Query<&mut Visibility, (With<Player>, Without<SkyDome>)>,
	cameras: Query<(Entity, Has<DistanceFog>), With<Camera3d>>,
) {
	let visibility = if visible.0 { Visibility::Inherited } else { Visibility::Hidden };
	for mut sky in &mut sky {
		*sky = visibility;
	}
	for mut player in &mut players {
		*player = visibility;
	}
	for (entity, has_fog) in &cameras {
		if visible.0 && !has_fog {
			commands.entity(entity).insert(world_distance_fog());
		} else if !visible.0 && has_fog {
			commands.entity(entity).remove::<DistanceFog>();
		}
	}
}

/// Vegetation capsule already has `ActiveCollisionHooks`; pair it with the shared motor marker.
pub(crate) fn stamp_vegetation_motor_traction(
	mut commands: Commands,
	players: Query<Entity, (With<Player>, Without<MotorTraction>)>,
) {
	for entity in &players {
		commands.entity(entity).insert(player::motor_traction_bundle());
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use durham_terrain_models::{terrain_collider_covers_xz, CascadeChunk};

	#[test]
	fn surface_ready_requires_local_column() {
		let spawn = Vec3::ZERO;
		let local = CascadeChunk::unit_chunk();
		let distant = CascadeChunk {
			origin: Vec3::new(1_000.0, -2_000.0, 1_000.0),
			size: 160.0,
			..CascadeChunk::unit_chunk()
		};
		assert!(!terrain_collider_covers_xz(spawn, [&distant]));
		assert!(terrain_collider_covers_xz(spawn, [&local]));
	}
}
