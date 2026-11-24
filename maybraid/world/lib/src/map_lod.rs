//! Extra [`lod::LodViewer`] while the world map is up.
//!
//! The follow camera keeps its [`lod::LodViewer`] and climbs with the map pose.
//! A second, settled viewer stays at the last ground / death pose so detail
//! there is not dropped or evicted.

use bevy::prelude::*;
use lod::{LodNodePose, LodNodeSystems, LodViewer};
use player_camera::{map_pose, CameraController, CameraTarget, FollowCamera, PlayerCameraSystems};
use world_player::Player as VegetationPlayer;

use crate::map_view::WorldMapView;
use crate::player_lifecycle::WorldPlayerRespawnState;

/// [`LodViewer`] left at the last ground / death pose while the map camera is high.
#[derive(Component, Debug, Clone, Copy)]
pub struct GroundLodAnchor;

impl GroundLodAnchor {
	/// Pose to hold while the map is open, if a real prior / death location exists.
	pub fn pose_while_open(
		map: &WorldMapView,
		pending: Option<&WorldPlayerRespawnState>,
		players: &Query<&Transform, With<VegetationPlayer>>,
	) -> Option<Vec3> {
		if !map.open {
			return None;
		}
		if map.close_locked {
			let pending = pending?.pending.as_ref()?;
			if pending.first_life && pending.death_at == Vec3::ZERO {
				return None;
			}
			return Some(pending.death_at);
		}
		if let Some((_, actor)) = map.owner {
			return players.get(actor).ok().map(|transform| transform.translation);
		}
		players.iter().next().map(|transform| transform.translation)
	}

	/// Spawn a settled viewer at `at` so it does not look like it arrived from the origin.
	pub fn spawn(commands: &mut Commands, at: Vec3) -> Entity {
		let transform = Transform::from_translation(at);
		commands
			.spawn((
				Name::new("ground-lod-anchor"),
				Self,
				LodViewer,
				transform,
				LodNodePose::settled(transform),
			))
			.id()
	}

	/// True once the follow camera has left the map pose and LOD tracking has
	/// observed that transform. A missing follow target keeps the extra viewer.
	pub fn follow_camera_has_handed_off(
		controller: &CameraController,
		target: Option<&CameraTarget>,
		transform: &Transform,
		pose: &LodNodePose,
		follow_at: Option<Vec3>,
	) -> bool {
		if controller.pov.is_map() {
			return false;
		}
		if target.and_then(|target| target.0).is_none() || follow_at.is_none() {
			return false;
		}
		let map_at = map_pose(controller.map_focus, controller.map_ground_y, controller.map_height)
			.translation;
		let still_map =
			transform.translation.distance(map_at) < controller.map_height.max(8.0) * 0.25;
		if still_map {
			return false;
		}
		pose.current.translation == transform.translation
	}

	/// Track copies last frame's camera transform, then the extra viewer can
	/// drop, then Follow writes this frame's pose. Shared by production and the
	/// scheduled handoff test. Do not order Track after Apply: LOD refresh
	/// already leads back into Anim and that edge cycles the full schedule.
	pub fn configure_handoff(app: &mut App) {
		app.add_systems(
			Update,
			despawn_ground_lod_anchors
				.after(LodNodeSystems::Track)
				.before(PlayerCameraSystems::Follow),
		);
	}
}

/// Spawn (or keep) the ground / death viewer as soon as the map opens, before the
/// follow camera climbs. First-life at the origin does not mint a bogus anchor.
pub(crate) fn spawn_ground_lod_anchors(
	map: Res<WorldMapView>,
	pending: Option<Res<WorldPlayerRespawnState>>,
	players: Query<&Transform, With<VegetationPlayer>>,
	anchors: Query<(Entity, &Transform), With<GroundLodAnchor>>,
	mut commands: Commands,
) {
	let wanted = GroundLodAnchor::pose_while_open(&map, pending.as_deref(), &players);
	let mut existing: Vec<(Entity, Vec3)> = anchors
		.iter()
		.map(|(entity, transform)| (entity, transform.translation))
		.collect();
	match wanted {
		None if map.open => {
			for (entity, _) in existing {
				commands.entity(entity).despawn();
			}
		}
		None => {}
		Some(at) if existing.is_empty() => {
			GroundLodAnchor::spawn(&mut commands, at);
		}
		Some(at) => {
			if let Some((entity, current)) = existing.first() {
				if *current != at {
					commands.entity(*entity).insert(Transform::from_translation(at));
				}
			}
			for (entity, _) in existing.drain(1..) {
				commands.entity(entity).despawn();
			}
		}
	}
}

/// Drop the extra viewer once last frame's follow pose has left map height and
/// [`lod::track_lod_nodes`] has copied that transform. Runs after Track and
/// before Follow so a moving camera is not one write ahead of the pose.
pub(crate) fn despawn_ground_lod_anchors(
	map: Res<WorldMapView>,
	cameras: Query<
		(&CameraController, Option<&CameraTarget>, &Transform, &LodNodePose),
		(With<FollowCamera>, With<LodViewer>),
	>,
	follows: Query<&Transform>,
	anchors: Query<Entity, With<GroundLodAnchor>>,
	mut commands: Commands,
) {
	if map.open {
		return;
	}
	if cameras.is_empty() {
		for entity in &anchors {
			commands.entity(entity).despawn();
		}
		return;
	}
	let handed_off = cameras.iter().any(|(controller, target, transform, pose)| {
		let follow_at = target
			.and_then(|target| target.0)
			.and_then(|actor| follows.get(actor).ok())
			.map(|follow| follow.translation);
		GroundLodAnchor::follow_camera_has_handed_off(
			controller, target, transform, pose, follow_at,
		)
	});
	if !handed_off {
		return;
	}
	for entity in &anchors {
		commands.entity(entity).despawn();
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use game_commands::command::TextEntryFocus;
	use lod::{track_lod_nodes, LodNode, LodNodePlugin};
	use maybraid_character_controller::{ActorIntentEvent, CharacterIntent};
	use maybraid_controllers_core::{ActorTarget, InputSource, Routed};
	use maybraid_session::WorldGameplayEnabled;
	use maybraid_spawns_player::PendingPlayerRespawn;
	use maybraid_virtual_pad_controller::PrimaryVirtualPad;
	use player::{CameraFollow, PlayerCameraAim, PlayerLook};
	use player_camera::{CameraPov, PlayerCameraPlugin, PlayerCameraSystems};
	use world_player::PlaygroundMode;

	use crate::map_view::toggle_map_view;

	fn lod_viewers(world: &World) -> Vec<(Entity, Vec3, bool)> {
		world
			.iter_entities()
			.filter(|entity| entity.get::<LodViewer>().is_some())
			.map(|entity| {
				(
					entity.id(),
					entity.get::<Transform>().map(|t| t.translation).unwrap_or(Vec3::ZERO),
					entity.get::<GroundLodAnchor>().is_some(),
				)
			})
			.collect()
	}

	fn spawn_map_camera(world: &mut World, at: Vec3, pov: CameraPov) -> Entity {
		let transform = Transform::from_translation(at);
		let mut controller = CameraController { pov, ..default() };
		controller.map_focus = at.xz();
		controller.map_ground_y = 0.0;
		controller.map_height = at.y.max(1.0);
		world
			.spawn((
				FollowCamera::default(),
				LodViewer,
				controller,
				transform,
				LodNodePose::settled(transform),
			))
			.id()
	}

	fn observe_pose(world: &mut World, camera: Entity, at: Vec3) -> anyhow::Result<()> {
		world.entity_mut(camera).insert(Transform::from_translation(at));
		world
			.run_system_once(track_lod_nodes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	fn hand_off_follow(
		world: &mut World,
		camera: Entity,
		player: Entity,
		at: Vec3,
	) -> anyhow::Result<()> {
		world.entity_mut(camera).insert((
			CameraTarget(Some(player)),
			CameraController { pov: CameraPov::ThirdPerson, map_height: 420.0, ..default() },
		));
		observe_pose(world, camera, at)?;
		run_despawn(world)?;
		Ok(())
	}

	fn run_spawn(world: &mut World) -> anyhow::Result<()> {
		world
			.run_system_once(spawn_ground_lod_anchors)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	fn run_despawn(world: &mut World) -> anyhow::Result<()> {
		world
			.run_system_once(despawn_ground_lod_anchors)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn select_map_keeps_ground_anchor_beside_the_camera_viewer() -> anyhow::Result<()> {
		let mut world = World::new();
		let player_at = Vec3::new(12.0, 3.0, -8.0);
		let player = world.spawn((VegetationPlayer, Transform::from_translation(player_at))).id();
		let source = world.spawn_empty().id();
		world.insert_resource(WorldMapView {
			open: true,
			focus: player_at.xz(),
			owner: Some((InputSource::virtual_pad(source), player)),
			..default()
		});
		let camera = spawn_map_camera(&mut world, player_at, CameraPov::ThirdPerson);

		run_spawn(&mut world)?;

		let viewers = lod_viewers(&world);
		assert_eq!(viewers.len(), 2, "camera LodViewer plus ground anchor");
		assert!(world.get::<LodViewer>(camera).is_some());
		assert!(world.get::<LodNode>(camera).is_some());
		let Some((anchor, at, _)) = viewers.iter().find(|(_, _, ground)| *ground) else {
			return Err(anyhow::anyhow!("ground anchor"));
		};
		assert_eq!(*at, player_at);
		let Some(pose) = world.get::<LodNodePose>(*anchor) else {
			return Err(anyhow::anyhow!("settled pose"));
		};
		assert_eq!(pose.previous.translation, player_at);
		assert_eq!(pose.current.translation, player_at);
		Ok(())
	}

	#[test]
	fn death_map_anchors_at_the_death_pose() -> anyhow::Result<()> {
		let mut world = World::new();
		let death = Vec3::new(40.0, 5.0, 12.0);
		world.insert_resource(WorldMapView { open: true, close_locked: true, ..default() });
		let mut state = WorldPlayerRespawnState::default();
		state.pending = Some(PendingPlayerRespawn::test(death, false));
		world.insert_resource(state);
		spawn_map_camera(&mut world, Vec3::new(40.0, 420.0, 12.0), CameraPov::Map);

		run_spawn(&mut world)?;

		let viewers = lod_viewers(&world);
		assert_eq!(viewers.len(), 2);
		let Some((_, at, _)) = viewers.iter().find(|(_, _, ground)| *ground) else {
			return Err(anyhow::anyhow!("death anchor"));
		};
		assert_eq!(*at, death);
		Ok(())
	}

	#[test]
	fn first_life_at_the_origin_does_not_spawn_a_bogus_anchor() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(WorldMapView { open: true, close_locked: true, ..default() });
		let mut state = WorldPlayerRespawnState::default();
		state.pending = Some(PendingPlayerRespawn::test(Vec3::ZERO, true));
		world.insert_resource(state);
		spawn_map_camera(&mut world, Vec3::new(0.0, 420.0, 0.0), CameraPov::Map);

		run_spawn(&mut world)?;

		assert_eq!(lod_viewers(&world).len(), 1, "only the map camera LodViewer");
		assert!(lod_viewers(&world).iter().all(|(_, _, ground)| !*ground));
		Ok(())
	}

	#[test]
	fn first_life_keeps_a_real_prior_pose() -> anyhow::Result<()> {
		let mut world = World::new();
		let prior = Vec3::new(80.0, 4.0, -20.0);
		world.insert_resource(WorldMapView { open: true, close_locked: true, ..default() });
		let mut state = WorldPlayerRespawnState::default();
		state.pending = Some(PendingPlayerRespawn::test(prior, true));
		world.insert_resource(state);
		spawn_map_camera(&mut world, Vec3::new(80.0, 420.0, -20.0), CameraPov::Map);

		run_spawn(&mut world)?;

		let anchor = lod_viewers(&world).into_iter().find(|(_, _, ground)| *ground);
		assert_eq!(anchor.map(|(_, at, _)| at), Some(prior));
		Ok(())
	}

	#[test]
	fn closing_the_map_keeps_the_anchor_while_the_camera_is_still_high() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(WorldMapView { open: true, close_locked: false, ..default() });
		let player = world.spawn((VegetationPlayer, Transform::from_xyz(4.0, 1.0, 2.0))).id();
		let camera = spawn_map_camera(&mut world, Vec3::new(4.0, 420.0, 2.0), CameraPov::Map);
		run_spawn(&mut world)?;
		assert_eq!(lod_viewers(&world).len(), 2);

		world.resource_mut::<WorldMapView>().close();
		run_spawn(&mut world)?;
		run_despawn(&mut world)?;
		assert_eq!(lod_viewers(&world).len(), 2, "keep the ground anchor until the camera drops");

		world.entity_mut(camera).insert(CameraController {
			pov: CameraPov::ThirdPerson,
			map_focus: Vec2::new(4.0, 2.0),
			map_height: 420.0,
			..default()
		});
		run_despawn(&mut world)?;
		assert_eq!(
			lod_viewers(&world).len(),
			2,
			"POV alone must not drop the anchor while Y is still map height"
		);

		hand_off_follow(&mut world, camera, player, Vec3::new(4.0, 1.0, 2.0))?;
		let viewers = lod_viewers(&world);
		assert_eq!(viewers.len(), 1);
		assert!(!viewers[0].2, "only the camera LodViewer remains");
		Ok(())
	}

	#[test]
	fn missing_follow_target_keeps_the_anchor_after_map_close() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(WorldMapView { open: true, close_locked: false, ..default() });
		world.spawn((VegetationPlayer, Transform::from_xyz(4.0, 1.0, 2.0)));
		let camera = spawn_map_camera(&mut world, Vec3::new(4.0, 420.0, 2.0), CameraPov::Map);
		run_spawn(&mut world)?;
		world.resource_mut::<WorldMapView>().close();
		world.entity_mut(camera).insert((
			CameraTarget(None),
			CameraController { pov: CameraPov::ThirdPerson, map_height: 420.0, ..default() },
		));
		run_despawn(&mut world)?;
		assert_eq!(lod_viewers(&world).len(), 2);
		Ok(())
	}

	#[test]
	fn respawn_cleans_the_death_anchor_once_the_player_camera_is_live() -> anyhow::Result<()> {
		let mut world = World::new();
		let death = Vec3::new(16.0, 2.0, 8.0);
		world.insert_resource(WorldMapView { open: true, close_locked: true, ..default() });
		let mut state = WorldPlayerRespawnState::default();
		state.pending = Some(PendingPlayerRespawn::test(death, false));
		world.insert_resource(state);
		let camera = spawn_map_camera(&mut world, Vec3::new(16.0, 420.0, 8.0), CameraPov::Map);
		run_spawn(&mut world)?;
		assert_eq!(lod_viewers(&world).len(), 2);

		world.resource_mut::<WorldMapView>().close();
		world.resource_mut::<WorldPlayerRespawnState>().pending = None;
		let player = world
			.spawn((VegetationPlayer, Transform::from_translation(death + Vec3::X * 4.0)))
			.id();
		run_spawn(&mut world)?;
		run_despawn(&mut world)?;
		assert_eq!(lod_viewers(&world).len(), 2, "death anchor stays until follow is observed");

		hand_off_follow(&mut world, camera, player, death + Vec3::X * 4.0)?;
		assert_eq!(lod_viewers(&world).len(), 1);
		Ok(())
	}

	#[test]
	fn death_picker_focus_moves_the_existing_anchor() -> anyhow::Result<()> {
		let mut world = World::new();
		let first = Vec3::new(16.0, 2.0, 8.0);
		let next = Vec3::new(40.0, 2.0, -4.0);
		world.insert_resource(WorldMapView { open: true, close_locked: true, ..default() });
		let mut state = WorldPlayerRespawnState::default();
		state.pending = Some(PendingPlayerRespawn::test(first, false));
		world.insert_resource(state);
		spawn_map_camera(&mut world, Vec3::new(16.0, 420.0, 8.0), CameraPov::Map);
		run_spawn(&mut world)?;
		{
			let mut state = world.resource_mut::<WorldPlayerRespawnState>();
			let Some(pending) = state.pending.as_mut() else {
				return Err(anyhow::anyhow!("pending death"));
			};
			pending.death_at = next;
		}
		run_spawn(&mut world)?;
		let anchors: Vec<_> =
			lod_viewers(&world).into_iter().filter(|(_, _, ground)| *ground).collect();
		anyhow::ensure!(anchors.len() == 1, "one death anchor");
		assert_eq!(anchors[0].1, next);
		Ok(())
	}

	#[test]
	fn session_cleanup_drops_anchors_without_a_follow_camera() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(WorldMapView { open: true, close_locked: false, ..default() });
		world.spawn((VegetationPlayer, Transform::from_xyz(4.0, 1.0, 2.0)));
		let camera = spawn_map_camera(&mut world, Vec3::new(4.0, 420.0, 2.0), CameraPov::Map);
		run_spawn(&mut world)?;
		world.resource_mut::<WorldMapView>().close();
		world.despawn(camera);
		run_despawn(&mut world)?;
		assert!(lod_viewers(&world).is_empty());
		Ok(())
	}

	#[test]
	fn toggle_map_view_retains_the_player_ground_anchor() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(TextEntryFocus(false));
		world.init_resource::<WorldMapView>();
		let player_at = Vec3::new(12.0, 3.0, -8.0);
		let player = world.spawn((VegetationPlayer, Transform::from_translation(player_at))).id();
		let source = world.spawn_empty().id();
		world.insert_resource(PrimaryVirtualPad(source));
		let camera = spawn_map_camera(&mut world, player_at, CameraPov::ThirdPerson);
		world.add_observer(toggle_map_view);
		world.trigger(ActorIntentEvent {
			entity: player,
			input: Routed::to_actor(
				InputSource::virtual_pad(source),
				ActorTarget::new(player),
				CharacterIntent::ToggleMap,
			),
		});
		assert!(world.resource::<WorldMapView>().open);
		run_spawn(&mut world)?;

		let viewers = lod_viewers(&world);
		assert_eq!(viewers.len(), 2);
		let Some((_, at, _)) = viewers.iter().find(|(_, _, ground)| *ground) else {
			return Err(anyhow::anyhow!("ground anchor"));
		};
		assert_eq!(*at, player_at);

		let map_at = map_pose(player_at.xz(), 0.0, 420.0).translation;
		observe_pose(&mut world, camera, map_at)?;
		run_spawn(&mut world)?;
		assert_eq!(lod_viewers(&world).iter().filter(|(_, _, ground)| *ground).count(), 1);
		assert!(world.get::<Transform>(camera).is_some_and(|t| t.translation.y > 100.0));

		world.resource_mut::<WorldMapView>().close();
		run_despawn(&mut world)?;
		assert_eq!(lod_viewers(&world).len(), 2, "close without follow handoff keeps the anchor");

		hand_off_follow(&mut world, camera, player, player_at)?;
		assert_eq!(lod_viewers(&world).len(), 1);
		Ok(())
	}

	#[test]
	fn map_zoom_keeps_the_ground_anchor() -> anyhow::Result<()> {
		let mut world = World::new();
		let player_at = Vec3::new(12.0, 3.0, -8.0);
		world.insert_resource(WorldMapView { open: true, focus: player_at.xz(), ..default() });
		world.spawn((VegetationPlayer, Transform::from_translation(player_at)));
		let camera = spawn_map_camera(&mut world, Vec3::new(12.0, 420.0, -8.0), CameraPov::Map);
		run_spawn(&mut world)?;
		observe_pose(&mut world, camera, map_pose(player_at.xz(), 0.0, 800.0).translation)?;
		run_spawn(&mut world)?;
		let viewers = lod_viewers(&world);
		assert_eq!(viewers.len(), 2);
		let Some((_, at, _)) = viewers.iter().find(|(_, _, ground)| *ground) else {
			return Err(anyhow::anyhow!("ground anchor"));
		};
		assert_eq!(*at, player_at);
		Ok(())
	}

	#[test]
	fn spawn_picker_highlight_keeps_the_death_anchor() -> anyhow::Result<()> {
		let mut world = World::new();
		let death = Vec3::new(16.0, 2.0, 8.0);
		let poi = Vec3::new(80.0, 2.0, -20.0);
		world.insert_resource(WorldMapView {
			open: true,
			close_locked: true,
			focus: poi.xz(),
			..default()
		});
		let mut state = WorldPlayerRespawnState::default();
		let mut pending = PendingPlayerRespawn::test(death, false);
		pending.highlighted_at = Some(poi.xz());
		state.pending = Some(pending);
		world.insert_resource(state);
		spawn_map_camera(&mut world, Vec3::new(poi.x, 420.0, poi.z), CameraPov::Map);

		run_spawn(&mut world)?;

		let anchors: Vec<_> =
			lod_viewers(&world).into_iter().filter(|(_, _, ground)| *ground).collect();
		anyhow::ensure!(anchors.len() == 1, "one death anchor");
		assert_eq!(anchors[0].1, death, "picker highlight must not move the death LodRef");
		Ok(())
	}

	fn scheduled_handoff_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TransformPlugin, LodNodePlugin, PlayerCameraPlugin))
			.init_resource::<ButtonInput<KeyCode>>()
			.add_message::<bevy::window::WindowFocused>()
			.init_resource::<WorldMapView>()
			.add_systems(Update, spawn_ground_lod_anchors.before(PlayerCameraSystems::Follow));
		GroundLodAnchor::configure_handoff(&mut app);
		app
	}

	fn spawn_scheduled_follow(
		world: &mut World,
		player_at: Vec3,
		pov: CameraPov,
	) -> (Entity, Entity) {
		let player = world
			.spawn((
				VegetationPlayer,
				CameraFollow,
				PlayerLook::default(),
				PlayerCameraAim::default(),
				Transform::from_translation(player_at),
			))
			.id();
		let map_at = map_pose(player_at.xz(), 0.0, 420.0).translation;
		let camera_at = if pov.is_map() { map_at } else { player_at };
		let transform = Transform::from_translation(camera_at);
		let mut controller = CameraController { pov, map_height: 420.0, ..default() };
		controller.map_focus = player_at.xz();
		controller.map_ground_y = 0.0;
		let camera = world
			.spawn((
				Camera3d::default(),
				FollowCamera::default(),
				LodViewer,
				CameraTarget(Some(player)),
				controller,
				transform,
				LodNodePose::settled(transform),
			))
			.id();
		(player, camera)
	}

	#[test]
	fn follow_then_track_clears_the_anchor_after_map_close() -> anyhow::Result<()> {
		let mut app = scheduled_handoff_app();
		let player_at = Vec3::new(12.0, 3.0, -8.0);
		app.world_mut().insert_resource(WorldMapView {
			open: true,
			focus: player_at.xz(),
			..default()
		});
		let (player, camera) = spawn_scheduled_follow(app.world_mut(), player_at, CameraPov::Map);
		app.update();
		assert_eq!(lod_viewers(app.world()).len(), 2, "ground anchor beside the map camera");

		app.world_mut().resource_mut::<WorldMapView>().close();
		app.world_mut().entity_mut(camera).insert(CameraTarget(None));
		app.world_mut().entity_mut(camera).insert(CameraController {
			pov: CameraPov::ThirdPerson,
			map_focus: player_at.xz(),
			map_ground_y: 0.0,
			map_height: 420.0,
			..default()
		});
		app.update();
		assert_eq!(
			lod_viewers(app.world()).len(),
			2,
			"missing follow target keeps the ground anchor"
		);

		app.world_mut().entity_mut(camera).insert(CameraTarget(Some(player)));
		let first_walk = player_at + Vec3::X * 8.0;
		app.world_mut()
			.entity_mut(player)
			.insert(Transform::from_translation(first_walk));
		app.update();
		assert_eq!(
			lod_viewers(app.world()).len(),
			2,
			"this frame's Follow write is not tracked yet"
		);
		let Some(first_camera) = app.world().get::<Transform>(camera).map(|t| t.translation) else {
			return Err(anyhow::anyhow!("camera transform"));
		};
		let Some(pose) = app.world().get::<LodNodePose>(camera) else {
			return Err(anyhow::anyhow!("camera LodNodePose"));
		};
		assert!(first_camera.y < 50.0, "Follow must write the ground camera, y={first_camera:?}");
		assert!(
			pose.current.translation.y > 100.0,
			"Track still holds the map pose, pose={:?}",
			pose.current.translation
		);
		assert!(
			(first_camera.xz() - first_walk.xz()).length() < 8.0,
			"live follow should track the walked actor, camera={first_camera:?} player={first_walk:?}"
		);

		let second_walk = first_walk + Vec3::Z * 6.0;
		app.world_mut()
			.entity_mut(player)
			.insert(Transform::from_translation(second_walk));
		app.update();
		let viewers = lod_viewers(app.world());
		assert_eq!(viewers.len(), 1, "next Track retires using the prior ground pose");
		assert!(!viewers[0].2);
		let Some(followed) = app.world().get::<Transform>(camera).map(|t| t.translation) else {
			return Err(anyhow::anyhow!("followed transform"));
		};
		let Some(pose) = app.world().get::<LodNodePose>(camera) else {
			return Err(anyhow::anyhow!("followed LodNodePose"));
		};
		let Some(player_now) = app.world().get::<Transform>(player).map(|t| t.translation) else {
			return Err(anyhow::anyhow!("player transform"));
		};
		assert_eq!(
			pose.current.translation, first_camera,
			"retirement must use the prior tracked ground pose"
		);
		assert_eq!(player_now, second_walk);
		assert!(
			(followed.xz() - second_walk.xz()).length() < 8.0,
			"camera keeps following, camera={followed:?} player={second_walk:?}"
		);
		assert!(
			(followed.xz() - first_camera.xz()).length() > 1.0,
			"camera must keep moving, first={first_camera:?} next={followed:?}"
		);
		Ok(())
	}

	#[test]
	fn leaving_the_session_drops_anchors_with_the_follow_camera() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(WorldMapView { open: true, close_locked: false, ..default() });
		world.insert_resource(WorldGameplayEnabled(false));
		world.spawn((VegetationPlayer, Transform::from_xyz(4.0, 1.0, 2.0)));
		let camera = spawn_map_camera(&mut world, Vec3::new(4.0, 420.0, 2.0), CameraPov::Map);
		run_spawn(&mut world)?;
		world.resource_mut::<WorldMapView>().close();
		world.despawn(camera);
		run_despawn(&mut world)?;
		assert!(lod_viewers(&world).is_empty());
		Ok(())
	}
}
