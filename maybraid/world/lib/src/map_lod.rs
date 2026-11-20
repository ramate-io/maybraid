//! Extra [`lod::LodViewer`] while the world map is up.
//!
//! The follow camera keeps its [`lod::LodViewer`] and climbs with the map pose.
//! A second, settled viewer stays at the last ground / death pose so detail
//! there is not dropped or evicted.

use bevy::prelude::*;
use lod::{LodNodePose, LodViewer};
use player_camera::{CameraController, CameraPov, FollowCamera};
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
	let mut existing: Vec<Entity> = anchors.iter().map(|(entity, _)| entity).collect();
	match wanted {
		None if map.open => {
			for entity in existing {
				commands.entity(entity).despawn();
			}
		}
		None => {}
		Some(at) if existing.is_empty() => {
			GroundLodAnchor::spawn(&mut commands, at);
		}
		Some(_) => {
			for entity in existing.drain(1..) {
				commands.entity(entity).despawn();
			}
		}
	}
}

/// Drop the extra viewer once the map is closed and the camera has left map POV.
pub(crate) fn despawn_ground_lod_anchors(
	map: Res<WorldMapView>,
	cameras: Query<&CameraController, (With<FollowCamera>, With<LodViewer>)>,
	anchors: Query<Entity, With<GroundLodAnchor>>,
	mut commands: Commands,
) {
	if map.open || cameras.iter().any(|controller| controller.pov.is_map()) {
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
	use lod::LodNode;
	use maybraid_character_controller::{ActorIntentEvent, CharacterIntent};
	use maybraid_controllers_core::{ActorTarget, InputSource, Routed};
	use maybraid_session::WorldGameplayEnabled;
	use maybraid_spawns_player::PendingPlayerRespawn;
	use maybraid_virtual_pad_controller::PrimaryVirtualPad;
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
		world
			.spawn((
				FollowCamera::default(),
				LodViewer,
				CameraController { pov, ..default() },
				Transform::from_translation(at),
			))
			.id()
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
		let anchor = viewers.iter().find(|(_, _, ground)| *ground).expect("ground anchor");
		assert_eq!(anchor.1, player_at);
		let pose = world.get::<LodNodePose>(anchor.0).expect("settled pose");
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
		let anchor = viewers.iter().find(|(_, _, ground)| *ground).expect("death anchor");
		assert_eq!(anchor.1, death);
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
	fn closing_the_map_waits_for_the_camera_to_leave_map_pov() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(WorldMapView { open: true, close_locked: false, ..default() });
		world.spawn((VegetationPlayer, Transform::from_xyz(4.0, 1.0, 2.0)));
		let camera = spawn_map_camera(&mut world, Vec3::new(4.0, 420.0, 2.0), CameraPov::Map);
		run_spawn(&mut world)?;
		assert_eq!(lod_viewers(&world).len(), 2);

		world.resource_mut::<WorldMapView>().close();
		run_spawn(&mut world)?;
		run_despawn(&mut world)?;
		assert_eq!(lod_viewers(&world).len(), 2, "keep the ground anchor until the camera drops");

		world
			.entity_mut(camera)
			.insert(CameraController { pov: CameraPov::ThirdPerson, ..default() });
		run_despawn(&mut world)?;
		let viewers = lod_viewers(&world);
		assert_eq!(viewers.len(), 1);
		assert!(!viewers[0].2, "only the camera LodViewer remains");
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
		world.spawn((VegetationPlayer, Transform::from_translation(death + Vec3::X * 4.0)));
		world
			.entity_mut(camera)
			.insert(CameraController { pov: CameraPov::ThirdPerson, ..default() });
		run_spawn(&mut world)?;
		run_despawn(&mut world)?;
		assert_eq!(lod_viewers(&world).len(), 1);
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
		spawn_map_camera(&mut world, player_at, CameraPov::ThirdPerson);
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
		let anchor = viewers.iter().find(|(_, _, ground)| *ground).expect("ground anchor");
		assert_eq!(anchor.1, player_at);

		world.resource_mut::<WorldMapView>().close();
		run_despawn(&mut world)?;
		assert_eq!(lod_viewers(&world).len(), 1);
		Ok(())
	}
}
