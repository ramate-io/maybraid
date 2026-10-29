//! World gameplay filter and scenery. Motor requests are declared, not applied here.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::{Inventory, InventorySlot};
use combat_hud::{CombatHudEquippedQueue, CombatHudEquippedSlot};
use durham::{
	terrain_collider_covers_xz, CascadeChunk, DurhamSurface, TerrainCellLayout,
	TerrainTrimeshCollider,
};
use game_commands::command::{CommandConsoleOutput, TextEntryFocus};
use maybraid_character_controller::{
	ActorControlStatus, ActorIntentEvent, CharacterIntent, InventoryCyclingCategory,
};
use maybraid_controllers_core::{CaptureKind, InputRouting, InputSource, ScopeOwner};
use maybraid_session::{SessionCapabilities, WorldGameplayEnabled, WorldPlaySessionEnded};
use maybraid_skill_map::SkillMapEnabled;
use maybraid_sky::{SkyDome, SKY_HORIZON};
use maybraid_virtual_pad_controller::PrimaryVirtualPad;
use player::{
	apply_character_controller, Buoyant, CharacterController, CharacterStance, JumpWish, Jumping,
	LocomotionCapsule, MotorIntent, MotorTraction, MoveWish as PlayerMoveWish,
	Player as MaybraidPlayer, PlayerCameraAim, PlayerLook, PlayerYawOwner, RestLocomotionCapsule,
	Sprinting, StanceKind, Wading,
};
use world_player::{MoveWish, Player, PlayerPhysicsEnabled, PlayerSpawnXz, PlaygroundMode};

use crate::inventory_cycling::ActiveInventoryCyclingCategory;
use crate::map_view::WorldMapView;

/// Keep third-person follow while the in-game inventory editor is open.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InventoryEditCameraFollow(pub bool);

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

/// Home / Characters shell. The game sets this from [`GameFlow`] so the agent
/// can start Home-first scripts without inferring the menu from streaming.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct WorldMenuShell(pub bool);

/// Terrain column under the player. The shell unveils after this.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WorldSurfaceSet;

pub(crate) fn update_world_surface_ready(
	streaming: Res<terrain_layer_model::TerrainStreaming<durham::Durham>>,
	surface: DurhamSurface,
	spawn: Res<PlayerSpawnXz>,
	players: Query<&Transform, With<Player>>,
	colliders: Query<&CascadeChunk, With<TerrainTrimeshCollider>>,
	mut ready: ResMut<WorldSurfaceReady>,
) {
	// Menu shells keep streaming off. Leave the ready bit alone.
	if !streaming.enabled {
		return;
	}
	let xz = discovery_xz(&spawn, &players, surface.layout());
	let at = Vec3::new(xz.x, 0.0, xz.y);
	let height_ready = matches!(surface.height_at(xz), Ok(Some(_)));
	ready.0 = terrain_collider_covers_xz(at, colliders.iter()) && height_ready;
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

type WorldOnlyPlayerQuery<'w, 's> =
	Query<'w, 's, Entity, (With<Player>, Without<MaybraidPlayer>, Added<Player>)>;

/// Mark the world capsule as a Lubikan owner. [`lubikans::LubikansPlugin`]
/// attaches the Self ledger in `LubikanSystems::Owner`; do not insert
/// `LubikanLedger::with_self` here. [#1112](https://github.com/ramate-io/maybraid/issues/1112)
/// can drop its temporary attach once this grant is on the stack.
///
/// Also inserts `player::Player` (the generic player marker from the player crate)
/// so that `combat-hud` vitals and continuous bars can query this entity.
pub fn grant_world_player_lubikan_owner(
	mut commands: Commands,
	// Match world players that don't have generic player::Player marker
	// This runs when the world player spawns AND when it respawns after death
	world_only: WorldOnlyPlayerQuery,
) {
	for player in &world_only {
		commands.entity(player).insert((
			MaybraidPlayer, // This is player::Player
			lubikans::LubikanOwner,
		));
	}
}

pub fn sync_lubikan_session(
	mut ended: MessageReader<WorldPlaySessionEnded>,
	mut lives: MessageWriter<lubikans::LubikanOwnerLifetime>,
) {
	if ended.read().next().is_none() {
		return;
	}
	lives.write(lubikans::LubikanOwnerLifetime::new(
		None,
		None,
		lubikans::LubikanOwnerLifetimeKind::RestartSession,
	));
}

pub(crate) fn sync_combat_hud_visible(
	gameplay: Res<WorldGameplayEnabled>,
	map: Option<Res<WorldMapView>>,
	hud: Option<ResMut<combat_hud::CombatHudVisible>>,
) {
	let Some(mut hud) = hud else {
		return;
	};
	let next = gameplay.combat_hud_visible(map.is_some_and(|map| map.open));
	if hud.0 != next {
		hud.0 = next;
	}
}

/// Compose the selected inventory queue into the small snapshot consumed by the HUD.
pub(crate) fn sync_combat_hud_equipped_queue(
	category: Res<ActiveInventoryCyclingCategory>,
	queue: Option<ResMut<CombatHudEquippedQueue>>,
	players: Query<&InventoryUser, With<Player>>,
	bags: Query<&Inventory>,
) {
	let Some(mut queue) = queue else {
		return;
	};
	let Some(user) = players.iter().next() else {
		if !queue.slots.is_empty() {
			*queue = CombatHudEquippedQueue::default();
		}
		return;
	};
	let Ok(inventory) = bags.get(user.bag) else {
		if !queue.slots.is_empty() {
			*queue = CombatHudEquippedQueue::default();
		}
		return;
	};
	let next = equipped_queue_from_inventory(inventory, category.0);
	if *queue != next {
		*queue = next;
	}
}

pub(crate) fn equipped_queue_from_inventory(
	inventory: &Inventory,
	category: InventoryCyclingCategory,
) -> CombatHudEquippedQueue {
	let slot = match category {
		InventoryCyclingCategory::Weapons => InventorySlot::Weapons,
		InventoryCyclingCategory::SkillMaps => InventorySlot::Skills,
		InventoryCyclingCategory::Lubikans => return CombatHudEquippedQueue::default(),
	};
	let indices = inventory.selected(slot);
	let mut slots: Vec<_> = indices
		.iter()
		.filter_map(|&index| {
			inventory.items.get(index).map(|item| CombatHudEquippedSlot {
				key: index as u64,
				label: item.label().to_string(),
			})
		})
		.collect();
	// Queue cycling rotates the active entry to the front. Keep chip positions
	// stable by bag identity so the highlight visibly moves between entries.
	slots.sort_by_key(|slot| slot.key);
	let selected = indices
		.first()
		.and_then(|index| slots.iter().position(|slot| slot.key == *index as u64))
		.unwrap_or(0);
	CombatHudEquippedQueue { slots, selected }
}

#[derive(Resource, Default)]
pub(crate) struct ConsoleCapture {
	token: Option<maybraid_controllers_core::CaptureToken>,
}

pub(crate) fn sync_console_capture(
	focus: Option<Res<TextEntryFocus>>,
	mut router: InputRouting,
	mut capture: ResMut<ConsoleCapture>,
) {
	router.replace_owned_capture(
		&mut capture.token,
		InputSource::local_pad(),
		focus.is_some_and(|focus| focus.0).then_some(ScopeOwner::console()),
		CaptureKind::TextEntry,
	);
}

pub(crate) fn sync_skill_map_enabled(
	gameplay: Res<WorldGameplayEnabled>,
	text_focus: Res<TextEntryFocus>,
	map: Option<Res<WorldMapView>>,
	primary: Option<Res<PrimaryVirtualPad>>,
	players: Query<&ActorControlStatus, With<Player>>,
	mut enabled: ResMut<SkillMapEnabled>,
) {
	let delivered = primary.as_deref().is_some_and(|primary| {
		players.iter().any(|control| {
			control.source().and_then(InputSource::entity) == Some(primary.0)
				&& control.is_delivered()
		})
	});
	let next = delivered
		&& gameplay.accepts_character_input(text_focus.0, map.is_some_and(|map| map.open));
	if enabled.0 != next {
		enabled.0 = next;
	}
}

/// WorldFilter: declare suppress when pause / text / map / non-character mode
/// win. Does not write live `player::MoveWish` or the Discovery vegetation
/// motor / `MovementAction` path.
pub(crate) fn apply_world_motor_filter(
	mode: Res<PlaygroundMode>,
	text_focus: Res<TextEntryFocus>,
	gameplay: Res<WorldGameplayEnabled>,
	session: Option<Res<SessionCapabilities>>,
	map: Option<Res<WorldMapView>>,
	primary: Option<Res<PrimaryVirtualPad>>,
	mut vegetation: Query<(Entity, &mut MoveWish, Option<&ActorControlStatus>), With<Player>>,
	mut motors: Query<
		(Entity, &mut MotorIntent, Option<&ActorControlStatus>),
		(With<CharacterController>, With<MaybraidPlayer>),
	>,
	mut world_motors: Query<&mut MotorIntent, (With<CharacterController>, Without<MaybraidPlayer>)>,
) {
	let simulation_frozen = session.as_deref().is_some_and(|session| !session.simulation_active());
	if simulation_frozen {
		for (_, mut wish, _) in &mut vegetation {
			wish.0 = Vec3::ZERO;
		}
		for (_, mut intent, _) in &mut motors {
			intent.suppress(player::MotorSuppressionToken::SIMULATION);
		}
		for mut intent in &mut world_motors {
			intent.suppress(player::MotorSuppressionToken::SIMULATION);
		}
		return;
	}
	let blocked = |entity: Entity, control: &ActorControlStatus| {
		let primary_text_focus = text_focus.0
			&& primary.as_deref().is_some_and(|primary| {
				control.source().and_then(InputSource::entity) == Some(primary.0)
			});
		let owns_map = map
			.as_deref()
			.is_some_and(|map| map.open && map.owner.is_some_and(|(_, owner)| owner == entity));
		control.is_controlled()
			&& (*mode != PlaygroundMode::Character
				|| !gameplay.0
				|| primary_text_focus
				|| owns_map || !control.is_delivered())
	};
	for (entity, mut wish, control) in &mut vegetation {
		if control.is_some_and(|control| blocked(entity, control)) {
			wish.0 = Vec3::ZERO;
		}
	}
	for (entity, mut intent, control) in &mut motors {
		if control.is_some_and(|control| blocked(entity, control)) {
			intent.suppress(player::MotorSuppressionToken::WORLD_GAMEPLAY);
		}
	}
}

pub(crate) fn echo_character_intents(
	event: On<ActorIntentEvent>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	let part = match *event.input.payload() {
		CharacterIntent::Move(value) => format!("move=({:.2},{:.2})", value.x, value.y),
		CharacterIntent::Look(value) => format!("look=({:.2},{:.2})", value.x, value.y),
		CharacterIntent::Focus(value) => format!("focus={value:.2}"),
		CharacterIntent::Ads(value) => format!("ads={value:.2}"),
		CharacterIntent::SkillMap => "skill-map".into(),
		CharacterIntent::UseItem(value) => format!("use={value:.2}"),
		other => other.label().to_string(),
	};
	console.0 = part;
}

fn world_distance_fog() -> DistanceFog {
	DistanceFog {
		color: SKY_HORIZON,
		directional_light_color: Color::srgba(1.0, 0.86, 0.62, 0.4),
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

type PlayerMotorMissingQuery<'w, 's> =
	Query<'w, 's, Entity, (With<Player>, Without<CharacterController>)>;
type PlayerMotorPresentQuery<'w, 's> = Query<
	'w,
	's,
	(
		Entity,
		Option<&'static CharacterStance>,
		Option<&'static RestLocomotionCapsule>,
		Option<&'static LocomotionCapsule>,
	),
	(With<Player>, With<CharacterController>),
>;

/// Put the world player on the player-crate capsule motor so column buoyancy runs.
pub(crate) fn stamp_world_player_motor(
	physics: Res<PlayerPhysicsEnabled>,
	mut commands: Commands,
	missing: PlayerMotorMissingQuery,
	present: PlayerMotorPresentQuery,
) {
	if physics.0 {
		for entity in &missing {
			apply_world_player_motor(&mut commands, entity);
		}
		for (entity, stance, rest, live) in &present {
			if stance.is_none() {
				commands.entity(entity).insert(CharacterStance::settled(StanceKind::Stand));
			}
			if rest.is_none() {
				commands.entity(entity).insert(RestLocomotionCapsule(
					live.copied().unwrap_or(LocomotionCapsule::HUMANOID),
				));
			}
		}
	} else {
		for (entity, _, _, _) in &present {
			strip_world_player_motor(&mut commands, entity);
		}
	}
}

pub(crate) fn apply_world_player_motor(commands: &mut Commands, body: Entity) {
	apply_character_controller(commands, body, LocomotionCapsule::HUMANOID);
	commands.entity(body).insert((
		MaybraidPlayer,
		PlayerLook::default(),
		PlayerCameraAim::default(),
		PlayerYawOwner::Wish,
	));
}

pub(crate) fn strip_world_player_motor(commands: &mut Commands, body: Entity) {
	commands.entity(body).remove::<(
		CharacterController,
		PlayerMoveWish,
		JumpWish,
		Jumping,
		Sprinting,
		CharacterStance,
		RestLocomotionCapsule,
		player::Grounded,
		Buoyant,
		Wading,
		player::MotorIntent,
		player::MotorSelection,
		player::ResolvedMotorRequest,
	)>();
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use character_items::{FirearmMesh, InventoryItem};
	use durham::{terrain_collider_covers_xz, CascadeChunk};
	use maybraid_controllers_core::InputRouter;

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

	#[test]
	fn skill_map_follows_gameplay_and_text_focus() {
		assert!(WorldGameplayEnabled(true).accepts_character_input(false, false));
		assert!(!WorldGameplayEnabled(false).accepts_character_input(false, false));
		assert!(!WorldGameplayEnabled(true).accepts_character_input(true, false));
	}

	#[test]
	fn equipped_queue_snapshot_keeps_chip_positions_and_tracks_rotating_primary() {
		let mut inventory = Inventory {
			items: vec![
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::firearm(FirearmMesh::Reltor),
				InventoryItem::firearm(FirearmMesh::Snailer),
			],
			weapons: vec![2, 0, 1],
			..Default::default()
		};
		let queue = equipped_queue_from_inventory(&inventory, InventoryCyclingCategory::Weapons);
		assert_eq!(queue.slots.iter().map(|slot| slot.key).collect::<Vec<_>>(), vec![0, 1, 2]);
		assert_eq!(queue.selected, 2);
		assert_eq!(queue.slots[queue.selected].label, "snailer");
		inventory.cycle_weapons(1);
		let next = equipped_queue_from_inventory(&inventory, InventoryCyclingCategory::Weapons);
		assert_eq!(next.slots, queue.slots);
		assert_eq!(next.selected, 0);
		assert!(equipped_queue_from_inventory(&inventory, InventoryCyclingCategory::Lubikans)
			.slots
			.is_empty());
	}

	#[test]
	fn world_player_receives_player_crate_motor_when_physics_is_on() {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.insert_resource(PlayerPhysicsEnabled(true));
		let player = world.spawn(Player).id();
		world
			.run_system_once(stamp_world_player_motor)
			.expect("stamp world player motor");
		assert!(world.get::<CharacterController>(player).is_some());
		assert!(world.get::<LocomotionCapsule>(player).is_some());
		assert!(world.get::<MaybraidPlayer>(player).is_some());
		assert!(world.get::<PlayerMoveWish>(player).is_some());
	}

	#[test]
	fn live_gameplay_does_not_write_vegetation_or_player_wishes() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(WorldGameplayEnabled(true));
		let vegetation = world
			.spawn((
				Player,
				MoveWish(Vec3::X),
				ActorControlStatus::assigned(InputSource::local_pad()),
			))
			.id();
		let mut intent = MotorIntent::default();
		intent.set_standard(player::MotorSample::translation(Vec3::Z));
		let body = world
			.spawn((CharacterController, MaybraidPlayer, PlayerMoveWish(Vec3::X), intent))
			.id();
		world
			.run_system_once(apply_world_motor_filter)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.get::<MoveWish>(vegetation).map(|wish| wish.0), Some(Vec3::X));
		assert_eq!(world.get::<PlayerMoveWish>(body).map(|wish| wish.0), Some(Vec3::X));
		assert!(!world.get::<MotorIntent>(body).is_some_and(MotorIntent::suppressed));
		Ok(())
	}

	#[test]
	fn paused_gameplay_suppresses_the_player_and_zeros_vegetation() -> anyhow::Result<()> {
		use player::{
			apply_motor, compose_motor, MotorSample, MotorSelection, Npc, ResolvedMotorRequest,
		};

		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(WorldGameplayEnabled(false));
		let vegetation = world
			.spawn((
				Player,
				MoveWish(Vec3::X),
				ActorControlStatus::assigned(InputSource::local_pad()),
			))
			.id();
		let mut player_intent = MotorIntent::default();
		player_intent.set_standard(MotorSample::translation(Vec3::X));
		let player = world
			.spawn((
				CharacterController,
				MaybraidPlayer,
				PlayerMoveWish(Vec3::X),
				player_intent,
				ActorControlStatus::assigned(InputSource::local_pad()),
				MotorSelection::default(),
				ResolvedMotorRequest::default(),
			))
			.id();
		let mut npc_intent = MotorIntent::default();
		npc_intent.set_npc(MotorSample::translation(Vec3::Z));
		let npc = world
			.spawn((
				Npc,
				CharacterController,
				PlayerMoveWish(Vec3::Z),
				npc_intent,
				MotorSelection::default(),
				ResolvedMotorRequest::default(),
			))
			.id();
		world
			.run_system_once(apply_world_motor_filter)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(compose_motor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(apply_motor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.get::<MoveWish>(vegetation).map(|wish| wish.0), Some(Vec3::ZERO));
		assert_eq!(world.get::<PlayerMoveWish>(player).map(|wish| wish.0), Some(Vec3::ZERO));
		assert!(world.get::<MotorIntent>(player).is_some_and(MotorIntent::suppressed));
		assert_eq!(world.get::<PlayerMoveWish>(npc).map(|wish| wish.0), Some(Vec3::Z));
		assert!(!world.get::<MotorIntent>(npc).is_some_and(MotorIntent::suppressed));
		Ok(())
	}

	#[test]
	fn map_capture_suppresses_only_its_owner() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(WorldGameplayEnabled(true));
		let source = InputSource::local_pad();
		let owner = world
			.spawn((Player, MoveWish(Vec3::X), ActorControlStatus::assigned(source)))
			.id();
		let other = world
			.spawn((Player, MoveWish(Vec3::Z), ActorControlStatus::assigned(source)))
			.id();
		world.insert_resource(WorldMapView {
			open: true,
			owner: Some((source, owner)),
			..default()
		});
		world
			.run_system_once(apply_world_motor_filter)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.get::<MoveWish>(owner).map(|wish| wish.0), Some(Vec3::ZERO));
		assert_eq!(world.get::<MoveWish>(other).map(|wish| wish.0), Some(Vec3::Z));
		Ok(())
	}

	#[test]
	fn pause_keeps_installed_motors() {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.insert_resource(PlayerPhysicsEnabled(true));
		world.insert_resource(WorldGameplayEnabled(false));
		let player = world.spawn(Player).id();
		apply_world_player_motor(&mut world.commands(), player);
		world.flush();
		world
			.run_system_once(stamp_world_player_motor)
			.expect("keep world player motor while paused");
		assert!(world.get::<CharacterController>(player).is_some());
		assert!(world.get::<PlayerMoveWish>(player).is_some());
	}

	#[test]
	fn world_player_motor_is_stripped_when_physics_is_off() {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.insert_resource(PlayerPhysicsEnabled(false));
		let player = world.spawn(Player).id();
		apply_world_player_motor(&mut world.commands(), player);
		world.flush();
		assert!(world.get::<CharacterController>(player).is_some());
		world
			.run_system_once(stamp_world_player_motor)
			.expect("strip world player motor");
		assert!(world.get::<CharacterController>(player).is_none());
		assert!(world.get::<PlayerMoveWish>(player).is_none());
	}

	#[test]
	fn stamp_restores_stance_after_it_is_stripped() {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.insert_resource(PlayerPhysicsEnabled(true));
		let player = world.spawn(Player).id();
		apply_world_player_motor(&mut world.commands(), player);
		world.flush();
		world.entity_mut(player).remove::<CharacterStance>();
		assert!(world.get::<CharacterStance>(player).is_none());
		world.run_system_once(stamp_world_player_motor).expect("restore stance");
		assert!(world.get::<CharacterStance>(player).is_some());
		assert!(world.get::<RestLocomotionCapsule>(player).is_some());
	}

	#[test]
	fn grant_marks_vegetation_player_without_inserting_a_ledger() {
		use bevy::ecs::system::RunSystemOnce;
		use lubikans::{LubikanLedger, LubikanOwner};

		let mut world = World::new();
		let player = world.spawn(Player).id();
		world.run_system_once(grant_world_player_lubikan_owner).expect("grant owner");
		assert!(world.get::<LubikanOwner>(player).is_some());
		assert!(world.get::<LubikanLedger>(player).is_none());
		world
			.run_system_once(grant_world_player_lubikan_owner)
			.expect("grant is idempotent");
		assert!(world.get::<LubikanOwner>(player).is_some());
	}

	#[test]
	fn pause_does_not_end_the_play_session() {
		use bevy::ecs::system::RunSystemOnce;
		use lubikans::{LubikanOwnerLifetime, LubikanOwnerLifetimeKind};

		fn take_notices(
			mut notices: MessageReader<LubikanOwnerLifetime>,
		) -> Vec<LubikanOwnerLifetime> {
			notices.read().copied().collect()
		}

		let mut world = World::new();
		world.init_resource::<Messages<LubikanOwnerLifetime>>();
		world.init_resource::<Messages<WorldPlaySessionEnded>>();
		world.insert_resource(WorldGameplayEnabled(true));
		world.run_system_once(sync_lubikan_session).expect("initial sync");
		world.resource_mut::<WorldGameplayEnabled>().0 = false;
		world.run_system_once(sync_lubikan_session).expect("paused sync");
		let paused = world.run_system_once(take_notices).expect("read pause notices");
		assert!(paused.is_empty());
		world.write_message(WorldPlaySessionEnded);
		world.run_system_once(sync_lubikan_session).expect("ended sync");
		let notices = world.run_system_once(take_notices).expect("read restart notices");
		assert_eq!(notices.len(), 1);
		assert_eq!(notices[0].kind, LubikanOwnerLifetimeKind::RestartSession);
	}

	#[test]
	fn same_frame_pause_suppresses_the_player_before_item_realization() -> anyhow::Result<()> {
		use maybraid_controllers_core::InputRouter;
		use player::{
			apply_motor, compose_motor, MotorSample, MotorSelection, Npc, ResolvedMotorRequest,
		};

		#[derive(Resource, Default)]
		struct ItemFired(bool);

		fn realize_item(gameplay: Res<WorldGameplayEnabled>, mut fired: ResMut<ItemFired>) {
			fired.0 = gameplay.0;
		}

		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(WorldGameplayEnabled(false));
		let mut capabilities = SessionCapabilities::default();
		capabilities.install_play(true);
		world.insert_resource(capabilities);
		let mut router = InputRouter::default();
		router.set_paused(true);
		world.insert_resource(router);
		world.init_resource::<ItemFired>();
		let mut player_intent = MotorIntent::default();
		player_intent.set_standard(MotorSample::translation(Vec3::X));
		let player = world
			.spawn((
				CharacterController,
				MaybraidPlayer,
				PlayerMoveWish(Vec3::X),
				player_intent,
				ActorControlStatus::assigned(InputSource::local_pad()),
				MotorSelection::default(),
				ResolvedMotorRequest::default(),
			))
			.id();
		let mut npc_intent = MotorIntent::default();
		npc_intent.set_npc(MotorSample::translation(Vec3::Z));
		let npc = world
			.spawn((
				Npc,
				CharacterController,
				PlayerMoveWish(Vec3::Z),
				npc_intent,
				MotorSelection::default(),
				ResolvedMotorRequest::default(),
			))
			.id();
		world
			.run_system_once(apply_world_motor_filter)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(compose_motor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(apply_motor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(realize_item)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.get::<PlayerMoveWish>(player).map(|wish| wish.0), Some(Vec3::ZERO));
		assert_eq!(world.get::<PlayerMoveWish>(npc).map(|wish| wish.0), Some(Vec3::Z));
		assert!(!world.resource::<ItemFired>().0);
		Ok(())
	}

	#[test]
	fn simulation_scope_is_the_world_wide_motor_gate() -> anyhow::Result<()> {
		use maybraid_session::SessionBit;
		use player::{
			apply_motor, compose_motor, MotorSample, MotorSelection, Npc, ResolvedMotorRequest,
		};

		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(WorldGameplayEnabled(true));
		let mut capabilities = SessionCapabilities::default();
		capabilities.install_play(true);
		capabilities.release(SessionBit::Simulation);
		world.insert_resource(capabilities);
		world.init_resource::<InputRouter>();
		let mut player_intent = MotorIntent::default();
		player_intent.set_standard(MotorSample::translation(Vec3::X));
		let player = world
			.spawn((
				CharacterController,
				MaybraidPlayer,
				PlayerMoveWish(Vec3::X),
				player_intent,
				MotorSelection::default(),
				ResolvedMotorRequest::default(),
			))
			.id();
		let mut npc_intent = MotorIntent::default();
		npc_intent.set_npc(MotorSample::translation(Vec3::Z));
		let npc = world
			.spawn((
				Npc,
				CharacterController,
				PlayerMoveWish(Vec3::Z),
				npc_intent,
				MotorSelection::default(),
				ResolvedMotorRequest::default(),
			))
			.id();
		world
			.run_system_once(apply_world_motor_filter)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(compose_motor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(apply_motor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.get::<PlayerMoveWish>(player).map(|wish| wish.0), Some(Vec3::ZERO));
		assert_eq!(
			world.get::<PlayerMoveWish>(npc).map(|wish| wish.0),
			Some(Vec3::ZERO),
			"Simulation bit release freezes NPC motors; pause Gameplay must not"
		);
		Ok(())
	}
}
