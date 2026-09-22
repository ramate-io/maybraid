//! Training Ground mount: a small terrain patch, a seated Les Halles pad, and
//! a free-for-all roster around the stair mouth.

use avian3d::prelude::{Collider, LinearVelocity};
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	holding_elevation, player_spawn_point_at, AwaitingTerrainSurface, OffTerrainAnchor,
	Player as VegetationPlayer, PlaygroundConfig,
};
use crozon_characters::species::braidman::BraidmanConfig;
use crozon_characters::CharacterRecipe;
use durham_terrain_models::{
	playable_world_cell_layout, training_grounds_cell_layout, AvianTerrainIndex, TerrainCellLayout,
	TerrainEntryStore, TerrainPresentEnabled, TerrainPresentPending, TerrainPresentationDirty,
	TerrainPresenterState, WorldBaseTerrain,
};
use firearm_intelligence::FirearmEngagement;
use firearm_user::{spawn_held_kit, FirearmUserSettings, LiveWeapon};
use firearms::FirearmConcept;
use les_halles_arena::{
	spawn_training_perimeter, training_perimeter_samples, ArenaMount, ArenaPad, LesHallesSpawn,
	TrainingArena,
};
use maybraid_mobs::{MobBrain, MobKind};
use npc_intelligence::{NpcBody, NpcInstall, Personality};
use player::{spawn_npc_visual, spawn_npc_with_hidden_capsule, LocomotionCapsule, PlayerLook};
use player_camera::CameraController;
use richmond_building_physics::BuildingWalkCollider;
use richmond_developments_on_terrain_playground::UrbanizationStreamingEnabled;
use spotting_intelligence::{InterestLayers, SpotBounds, SpotSubject};
use std::f32::consts::TAU;
use threat_intelligence::{ThreatId, ThreatSubject};
use threat_management_intelligence::ThreatManagementIntelligence;

use crate::WorldSurfaceReady;

/// Set by the game shell while Training is the live world mount.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrainingGrounds(pub bool);

/// Origin height has been applied to the pad, the stack, and the perimeter.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub(crate) struct TrainingSeated;

const FFA_NPCS: u16 = 6;
const FFA_RING: f32 = 26.0;

/// While [`LesHallesSpawn`] is present, stand the player on that stair mouth.
/// When the session drops the resource, park them back on the region center so
/// the next Discovery enter is not left at the arena.
pub(crate) fn sync_off_terrain_player(
	spawn: Option<Res<LesHallesSpawn>>,
	seated: Option<Res<TrainingSeated>>,
	mut commands: Commands,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	mut players: Query<
		(Entity, &mut Transform, &mut LinearVelocity, Has<OffTerrainAnchor>),
		(With<VegetationPlayer>, Without<Camera3d>),
	>,
	mut cameras: Query<
		(&mut Transform, &mut CameraController),
		(With<Camera3d>, Without<VegetationPlayer>),
	>,
) {
	if let Some(spawn) = spawn.as_deref().copied() {
		for (entity, mut transform, mut velocity, anchored) in &mut players {
			if !anchored {
				transform.translation = spawn.player;
				**velocity = Vec3::ZERO;
				commands.entity(entity).insert(OffTerrainAnchor { translation: spawn.player });
			}
		}
		// Keep the viewer on the grounds while the patch generates. Follow takes
		// over once the pad is seated; until then the menu eye would stream
		// terrain somewhere else and the seat would never see a height.
		if seated.is_none() {
			for (mut transform, mut camera) in &mut cameras {
				camera.yaw = spawn.look_yaw;
				transform.translation = spawn.player + Vec3::Y * 6.0;
			}
		}
		return;
	}

	for (entity, mut transform, mut velocity, anchored) in &mut players {
		if !anchored {
			continue;
		}
		let xz = layout.region_center_xz().xz();
		let elevation = holding_elevation(&base.0, xz.x, xz.y);
		transform.translation = player_spawn_point_at(xz, elevation);
		**velocity = Vec3::ZERO;
		commands.entity(entity).insert(AwaitingTerrainSurface);
		commands.entity(entity).remove::<OffTerrainAnchor>();
	}
}

/// Pad collider plus one building walk collider, or the pad alone when the fit failed.
/// A terrain training mount also waits until the pad has been seated on the patch.
pub(crate) fn sync_arena_surface_ready(
	mount: Option<Res<ArenaMount>>,
	grounds: Option<Res<TrainingGrounds>>,
	seated: Option<Res<TrainingSeated>>,
	pads: Query<(), (With<ArenaPad>, With<Collider>)>,
	walks: Query<&ChildOf, With<BuildingWalkCollider>>,
	arena: Query<(), With<TrainingArena>>,
	mut ready: ResMut<WorldSurfaceReady>,
) {
	let Some(mount) = mount else {
		return;
	};
	if grounds.is_some_and(|grounds| grounds.0) && seated.is_none() {
		ready.0 = false;
		return;
	}
	let pad = !pads.is_empty();
	let walk = walks.iter().any(|child| arena.contains(child.parent()));
	ready.0 = pad && (walk || mount.failed);
}

/// Shrink Durham and the forest to the training patch, and keep hopscotch off.
/// Restores the playable rings when the shell clears [`TrainingGrounds`].
pub(crate) fn apply_training_grounds(
	grounds: Res<TrainingGrounds>,
	mut active: Local<bool>,
	mut saved_forest_radius: Local<Option<u32>>,
	mut index: AvianTerrainIndex,
	mut present: ResMut<TerrainPresentEnabled>,
	mut dirty: ResMut<TerrainPresentationDirty>,
	mut pending: ResMut<TerrainPresentPending>,
	mut forest: Option<ResMut<PlaygroundConfig>>,
	mut urban: Option<ResMut<UrbanizationStreamingEnabled>>,
) {
	if *active == grounds.0 {
		return;
	}
	let enable = grounds.0;
	*active = enable;
	let layout = if enable { training_grounds_cell_layout() } else { playable_world_cell_layout() };
	index.set_layout(layout);
	present.0 = enable;
	dirty.0 = true;
	pending.0 = true;
	if let Some(urban) = urban.as_deref_mut() {
		urban.0 = !enable;
	}
	if let Some(config) = forest.as_deref_mut() {
		if let Some(spec) = config.forest.as_mut() {
			if enable {
				if saved_forest_radius.is_none() {
					*saved_forest_radius = Some(spec.stream_radius);
				}
				spec.stream_radius = 0;
			} else if let Some(radius) = saved_forest_radius.take() {
				spec.stream_radius = radius;
			}
		}
	}
}

/// Drop raw training terrain meshes once present is turned back off.
pub(crate) fn clear_training_terrain_present(
	present: Res<TerrainPresentEnabled>,
	mut was_present: Local<bool>,
	mut commands: Commands,
	mut state: ResMut<TerrainPresenterState>,
) {
	if *was_present && !present.0 {
		state.clear(&mut commands);
		commands.remove_resource::<TrainingSeated>();
	}
	*was_present = present.0;
}

/// Lift the pad and stack onto the origin's composed height, then close the
/// grounds with a terrain-following wall and a free-for-all ring.
pub(crate) fn seat_training_grounds(
	mut commands: Commands,
	grounds: Res<TrainingGrounds>,
	seated: Option<Res<TrainingSeated>>,
	spawn: Option<Res<LesHallesSpawn>>,
	store: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	mut arena: Query<&mut Transform, (With<TrainingArena>, Without<VegetationPlayer>)>,
	mut players: Query<
		(&mut Transform, Option<&mut OffTerrainAnchor>),
		(With<VegetationPlayer>, Without<TrainingArena>),
	>,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	if !grounds.0 || seated.is_some() {
		return;
	}
	let Some(spawn) = spawn.as_deref().copied() else {
		return;
	};
	let samples = training_perimeter_samples();
	let mut terrain_y = Vec::with_capacity(samples.len());
	for xz in &samples {
		let Some(y) = store.composed_height_at(&layout, xz.x, xz.y) else {
			return;
		};
		terrain_y.push(y);
	}
	let Some(plaza) = store.composed_height_at(&layout, 0.0, 0.0) else {
		return;
	};

	for mut transform in &mut arena {
		transform.translation.y += plaza;
	}
	for (mut transform, anchor) in &mut players {
		if let Some(mut anchor) = anchor {
			transform.translation.y += plaza;
			anchor.translation.y += plaza;
		}
	}
	let mut seated_spawn = spawn;
	seated_spawn.player.y += plaza;
	seated_spawn.npc.y += plaza;
	seated_spawn.floor_y[0] += plaza;
	seated_spawn.floor_y[1] += plaza;
	commands.insert_resource(seated_spawn);
	spawn_training_perimeter(&mut commands, &terrain_y, plaza);
	spawn_ffa_roster(&mut commands, &mut meshes, &mut materials, &seated_spawn);
	commands.insert_resource(TrainingSeated);
}

fn spawn_ffa_roster(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	spawn: &LesHallesSpawn,
) {
	let hull = LocomotionCapsule::HUMANOID;
	let appearance = BraidmanConfig::default_preview();
	for index in 0..FFA_NPCS {
		let theta = spawn.look_yaw + TAU * (index as f32 + 0.5) / FFA_NPCS as f32;
		let translation = Vec3::new(
			spawn.player.x + theta.sin() * FFA_RING,
			spawn.floor_y[0],
			spawn.player.z - theta.cos() * FFA_RING,
		);
		let npc = spawn_npc_with_hidden_capsule(
			commands,
			translation,
			PlayerLook { yaw: theta + std::f32::consts::PI, ..default() },
			meshes,
			materials,
		);
		spawn_npc_visual(
			commands,
			npc,
			CharacterRecipe::clothed(&appearance),
			Quat::from_rotation_y(theta),
		);
		Personality::Brawler.install(
			commands,
			npc,
			NpcInstall {
				at: translation,
				body: NpcBody {
					agent_radius: hull.radius,
					feet_below_origin: hull.half_height(),
					eye_height: 1.45,
				},
				health: damage::Health::default(),
				armed: true,
				engagement: Some(FirearmEngagement::hold()),
				threat_override: Some(ThreatManagementIntelligence::ffa()),
				selection_salt: index as u64 + 1,
				..NpcInstall::default()
			},
		);
		let id = ThreatId(npc.to_bits());
		let gun = spawn_held_kit(
			commands,
			npc,
			FirearmUserSettings::default(),
			FirearmConcept::Bullpup.kit(),
			LiveWeapon::default(),
		);
		commands.entity(npc).insert((
			TrainingArena,
			ThreatSubject::new(id),
			MobBrain::for_kind(MobKind::Brawler).affiliations.for_member(id),
			SpotSubject::new(
				InterestLayers::CHARACTER,
				SpotBounds::capsule(hull.radius, hull.half_height()),
			),
		));
		commands.entity(gun).insert(TrainingArena);
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use durham_terrain_models::{BaseTerrainNoise, TerrainConfig};

	use super::*;

	#[test]
	fn seating_queries_do_not_alias_transform() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(TrainingGrounds(false));
		world.insert_resource(TerrainCellLayout::default());
		world.insert_resource(TerrainEntryStore::default());
		world.init_resource::<Assets<Mesh>>();
		world.init_resource::<Assets<StandardMaterial>>();
		world.spawn((TrainingArena, Transform::default()));
		world.spawn((
			VegetationPlayer,
			Transform::default(),
			OffTerrainAnchor { translation: Vec3::ZERO },
		));
		world
			.run_system_once(seat_training_grounds)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn viewer_snap_queries_do_not_alias_transform() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(TerrainCellLayout::default());
		world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(
			1,
		))));
		world.spawn((
			VegetationPlayer,
			Transform::default(),
			LinearVelocity::default(),
		));
		world.spawn((Camera3d::default(), Transform::default()));
		world
			.run_system_once(sync_off_terrain_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn pad_and_walk_are_ready_without_a_terrain_column() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(ArenaMount { floors: 2, failed: false });
		world.insert_resource(WorldSurfaceReady(false));
		let host = world.spawn(TrainingArena).id();
		world.spawn((ArenaPad, Collider::cuboid(100.0, 0.2, 80.0), TrainingArena));
		world.spawn((BuildingWalkCollider, ChildOf(host)));
		world
			.run_system_once(sync_arena_surface_ready)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource::<WorldSurfaceReady>().0);
		Ok(())
	}

	#[test]
	fn fit_failure_is_ready_on_the_pad_alone() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(ArenaMount { floors: 0, failed: true });
		world.insert_resource(WorldSurfaceReady(false));
		world.spawn((ArenaPad, Collider::cuboid(100.0, 0.2, 80.0)));
		world
			.run_system_once(sync_arena_surface_ready)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource::<WorldSurfaceReady>().0);
		Ok(())
	}

	#[test]
	fn missing_walk_collider_stays_unready() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(ArenaMount { floors: 2, failed: false });
		world.insert_resource(WorldSurfaceReady(false));
		world.spawn((ArenaPad, Collider::cuboid(100.0, 0.2, 80.0)));
		world
			.run_system_once(sync_arena_surface_ready)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.resource::<WorldSurfaceReady>().0);
		Ok(())
	}
}
