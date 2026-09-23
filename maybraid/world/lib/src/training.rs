//! Training Ground on the live world: two fixed mobs at a generated building,
//! inside a terrain perimeter. The world streams stay as they are for Discovery.

use avian3d::prelude::LinearVelocity;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	holding_elevation, player_spawn_point_at, AwaitingTerrainSurface, OffTerrainAnchor,
	Player as VegetationPlayer,
};
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore, WorldBaseTerrain};
use firearm_user::FirearmUser;
use maybraid_mobs::{MobBrain, MobKind};
use mob_characters::{
	CharacterBrains, CharacterBuild, CharacterInventory, CharacterSpecies, FromMobNumber,
	MobCharacter,
};
use player_camera::CameraController;
use procedural_common::Bounds2;
use richmond_building_components::{building_bounds, spawn_building_components};
use richmond_building_physics::{spawn_building_walk_colliders, BUILDING_FRICTION};
use richmond_buildings::wall_demo::TerrainPerimeterWall;
use richmond_development_models::{DevelopmentCell, DevelopmentEntryStore};
use threat_intelligence::{ThreatId, ThreatSubject};

use crate::WorldSurfaceReady;

/// Set by the game shell while Training is the live world mount.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrainingGrounds(pub bool);

/// Wall, mobs, and their guns. Leave despawns these and leaves Discovery mobs alone.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct TrainingFixture;

/// The grounds have been spawned for this enter. Cleared when the session ends.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct TrainingPlaced;

/// Player was moved onto the grounds, so Leave can park them back on the region.
#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct TrainingPlacement;

const SEARCH_M: f32 = 1_600.0;
const GROUNDS_MARGIN: f32 = 24.0;
const WALL_STEP: f32 = 8.0;
const WALL_CLEARANCE: f32 = 4.0;
const STAND_PAST_PAD: f32 = 8.0;
const STAND_INSET: f32 = 4.0;
const MOB_SPACING: f32 = 6.0;

/// Stable pair: different clothing seeds, armed profiles, and biped species.
fn training_roster() -> [MobCharacter; 2] {
	[
		training_mob(0.17, CharacterInventory::Grunt, 0),
		training_mob(0.83, CharacterInventory::Mercenary, 4),
	]
}

fn training_mob(num: f32, inventory: CharacterInventory, species: usize) -> MobCharacter {
	let bipeds = CharacterSpecies::BIPEDS;
	MobCharacter {
		num,
		build: CharacterBuild::from_num(num),
		species: bipeds[species % bipeds.len()],
		inventory,
		brains: CharacterBrains::Brawler,
	}
}

/// Keep the unveil closed until the building, wall, and mobs exist.
pub(crate) fn hold_training_until_placed(
	grounds: Res<TrainingGrounds>,
	placed: Option<Res<TrainingPlaced>>,
	mut ready: ResMut<WorldSurfaceReady>,
) {
	if grounds.0 && placed.is_none() {
		ready.0 = false;
	}
}

/// Once a filled development is near the player, stand them on its terrace,
/// spawn the roster, and close a stone strip around the pads.
pub(crate) fn place_training_grounds(
	grounds: Res<TrainingGrounds>,
	placed: Option<Res<TrainingPlaced>>,
	mut commands: Commands,
	developments: Res<DevelopmentEntryStore>,
	terrain: Res<TerrainEntryStore>,
	layout: Res<TerrainCellLayout>,
	mut players: Query<
		(Entity, &mut Transform, &mut LinearVelocity),
		(With<VegetationPlayer>, Without<Camera3d>),
	>,
	mut cameras: Query<
		(&mut Transform, &mut CameraController),
		(With<Camera3d>, Without<VegetationPlayer>),
	>,
) {
	if !grounds.0 || placed.is_some() {
		return;
	}
	let Some(player_at) = players.iter_mut().next().map(|(_, transform, _)| transform.translation)
	else {
		return;
	};
	let Some(plan) = plan_grounds(&developments, &terrain, &layout, player_at) else {
		snap_viewer(&mut cameras, player_at);
		return;
	};

	for (mob, at) in training_roster().into_iter().zip(plan.mobs) {
		let look = Vec3::new(plan.pad_center.x, at.y, plan.pad_center.y);
		let entity = mob
			.scene_recipe()
			.spawn(&mut commands, Transform::from_translation(at).looking_at(look, Vec3::Y));
		let id = ThreatId(entity.to_bits());
		let affiliations = MobBrain::for_kind(MobKind::Brawler).affiliations.for_member(id);
		commands
			.entity(entity)
			.insert((TrainingFixture, ThreatSubject::new(id), affiliations));
	}

	let hosts = spawn_building_components(
		&mut commands,
		&plan.wall,
		Transform::IDENTITY,
		building_bounds(&plan.wall),
	);
	for entity in hosts {
		spawn_building_walk_colliders(&mut commands, entity, &plan.wall, BUILDING_FRICTION);
		commands.entity(entity).insert(TrainingFixture);
	}

	let stand = plan.stand;
	let look = Vec3::new(plan.pad_center.x, stand.y, plan.pad_center.y);
	for (entity, mut transform, mut velocity) in &mut players {
		*transform = Transform::from_translation(stand).looking_at(look, Vec3::Y);
		**velocity = Vec3::ZERO;
		commands.entity(entity).insert((AwaitingTerrainSurface, TrainingPlacement));
		commands.entity(entity).remove::<OffTerrainAnchor>();
	}
	snap_viewer(&mut cameras, stand);
	commands.insert_resource(TrainingPlaced);
	info!(
		target: "world.training",
		"placed training grounds at ({:.1}, {:.1})",
		stand.x, stand.z
	);
}

/// After Leave, put a moved player back on the region center. Waypoints stay gated
/// on [`TrainingGrounds`], so this park is not written to `positions/{id}.json`.
pub(crate) fn park_player_after_training(
	grounds: Res<TrainingGrounds>,
	mut commands: Commands,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	mut players: Query<
		(Entity, &mut Transform, &mut LinearVelocity),
		(With<VegetationPlayer>, With<TrainingPlacement>, Without<Camera3d>),
	>,
) {
	if grounds.0 {
		return;
	}
	let xz = layout.region_center_xz().xz();
	let elevation = holding_elevation(&base.0, xz.x, xz.y);
	let at = player_spawn_point_at(xz, elevation);
	for (entity, mut transform, mut velocity) in &mut players {
		transform.translation = at;
		**velocity = Vec3::ZERO;
		commands.entity(entity).insert(AwaitingTerrainSurface);
		commands.entity(entity).remove::<(TrainingPlacement, OffTerrainAnchor)>();
	}
}

/// Guns are world-posed, not parented. Tag them so Leave despawns the kit with the body.
pub(crate) fn mark_training_guns(
	mut commands: Commands,
	mobs: Query<&FirearmUser, With<TrainingFixture>>,
	guns: Query<(), Without<TrainingFixture>>,
) {
	for user in &mobs {
		if guns.get(user.held).is_ok() {
			commands.entity(user.held).insert(TrainingFixture);
		}
	}
}

struct GroundsPlan {
	wall: TerrainPerimeterWall,
	pad_center: Vec2,
	stand: Vec3,
	mobs: [Vec3; 2],
}

fn plan_grounds(
	developments: &DevelopmentEntryStore,
	terrain: &TerrainEntryStore,
	layout: &TerrainCellLayout,
	player_at: Vec3,
) -> Option<GroundsPlan> {
	let region = Aabb3d::from_min_max(
		Vec3::new(player_at.x - SEARCH_M, -2_000.0, player_at.z - SEARCH_M),
		Vec3::new(player_at.x + SEARCH_M, 2_000.0, player_at.z + SEARCH_M),
	);
	let player_xz = player_at.xz();
	let cell = developments.filled_cells_overlapping(region).into_iter().min_by(|a, b| {
		let da = cell_center_xz(a).distance_squared(player_xz);
		let db = cell_center_xz(b).distance_squared(player_xz);
		da.total_cmp(&db)
	})?;
	let (pads, plaza) = pad_union(cell)?;
	let wall_min = pads.min - Vec2::splat(GROUNDS_MARGIN);
	let wall_max = pads.max + Vec2::splat(GROUNDS_MARGIN);
	let samples = TerrainPerimeterWall::sample_rectangle(wall_min, wall_max, WALL_STEP);
	if samples.len() < 4 {
		return None;
	}
	let mut heights = Vec::with_capacity(samples.len());
	for sample in &samples {
		heights.push(terrain.composed_height_at(layout, sample.x, sample.y)?);
	}
	let stand_xz = Vec2::new(
		(pads.max.x + STAND_PAST_PAD).min(wall_max.x - STAND_INSET),
		pads.center().y.clamp(wall_min.y + STAND_INSET, wall_max.y - STAND_INSET),
	);
	let stand_y = terrain.composed_height_at(layout, stand_xz.x, stand_xz.y)?;
	let mob_xz = [
		clamp_inside(stand_xz + Vec2::new(0.0, MOB_SPACING), wall_min, wall_max),
		clamp_inside(stand_xz - Vec2::new(0.0, MOB_SPACING), wall_min, wall_max),
	];
	let mobs = [
		Vec3::new(
			mob_xz[0].x,
			terrain.composed_height_at(layout, mob_xz[0].x, mob_xz[0].y)?,
			mob_xz[0].y,
		),
		Vec3::new(
			mob_xz[1].x,
			terrain.composed_height_at(layout, mob_xz[1].x, mob_xz[1].y)?,
			mob_xz[1].y,
		),
	];
	Some(GroundsPlan {
		wall: TerrainPerimeterWall::from_samples(&samples, &heights, plaza, WALL_CLEARANCE),
		pad_center: pads.center(),
		stand: player_spawn_point_at(stand_xz, stand_y),
		mobs,
	})
}

fn pad_union(cell: &DevelopmentCell) -> Option<(Bounds2, f32)> {
	let mut pads = cell.pads();
	let first = pads.next()?;
	let mut bounds = first.complex.bounds;
	let mut plaza = first.height;
	for pad in pads {
		bounds.min = bounds.min.min(pad.complex.bounds.min);
		bounds.max = bounds.max.max(pad.complex.bounds.max);
		plaza = plaza.max(pad.height);
	}
	Some((bounds, plaza))
}

fn cell_center_xz(cell: &DevelopmentCell) -> Vec2 {
	let min = Vec3::from(cell.cell.min);
	let max = Vec3::from(cell.cell.max);
	Vec2::new((min.x + max.x) * 0.5, (min.z + max.z) * 0.5)
}

fn clamp_inside(xz: Vec2, min: Vec2, max: Vec2) -> Vec2 {
	Vec2::new(
		xz.x.clamp(min.x + STAND_INSET, max.x - STAND_INSET),
		xz.y.clamp(min.y + STAND_INSET, max.y - STAND_INSET),
	)
}

fn snap_viewer(
	cameras: &mut Query<
		(&mut Transform, &mut CameraController),
		(With<Camera3d>, Without<VegetationPlayer>),
	>,
	at: Vec3,
) {
	for (mut transform, mut controller) in cameras.iter_mut() {
		transform.translation = at + Vec3::new(-12.0, 8.0, 10.0);
		transform.look_at(at + Vec3::Y * 1.6, Vec3::Y);
		let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
		controller.yaw = yaw;
		controller.pitch = pitch;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use durham_terrain_models::{BaseTerrainNoise, TerrainConfig};
	use maybraid_mobs::player_affiliations;

	#[test]
	fn roster_is_two_armed_bipeds_with_different_kits() {
		let roster = training_roster();
		assert_eq!(roster.len(), 2);
		let recipes = [roster[0].scene_recipe(), roster[1].scene_recipe()];
		assert!(recipes[0].armed() && recipes[1].armed());
		assert_ne!(recipes[0].num, recipes[1].num);
		assert_ne!(recipes[0].inventory_profile, recipes[1].inventory_profile);
		assert_eq!(recipes[0].brains, CharacterBrains::Brawler);
		assert_eq!(recipes[1].brains, CharacterBrains::Brawler);
		assert!(recipes[0].species.supports_inventory());
		assert!(recipes[1].species.supports_inventory());
		assert_ne!(recipes[0].species, recipes[1].species);
	}

	#[test]
	fn brawler_pack_treats_the_world_player_as_hostile() {
		let member = MobBrain::for_kind(MobKind::Brawler).affiliations.for_member(ThreatId(7));
		let player = player_affiliations(ThreatId(1));
		assert!(member.threat_weight(&player, 0.0) >= 1.0);
	}

	#[test]
	fn hold_closes_unveil_until_the_grounds_exist() {
		let mut world = World::new();
		world.insert_resource(TrainingGrounds(true));
		world.insert_resource(WorldSurfaceReady(true));
		world.run_system_once(hold_training_until_placed).unwrap();
		assert!(!world.resource::<WorldSurfaceReady>().0);

		world.insert_resource(TrainingPlaced);
		world.resource_mut::<WorldSurfaceReady>().0 = true;
		world.run_system_once(hold_training_until_placed).unwrap();
		assert!(world.resource::<WorldSurfaceReady>().0);

		world.insert_resource(TrainingGrounds(false));
		world.resource_mut::<WorldSurfaceReady>().0 = true;
		world.run_system_once(hold_training_until_placed).unwrap();
		assert!(world.resource::<WorldSurfaceReady>().0);
	}

	#[test]
	fn place_waits_for_a_filled_cell_without_aliasing_transforms() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(TrainingGrounds(true));
		world.insert_resource(DevelopmentEntryStore::default());
		world.insert_resource(TerrainEntryStore::default());
		world.insert_resource(TerrainCellLayout::default());
		world.spawn((
			VegetationPlayer,
			Transform::from_xyz(4.0, 1.0, 4.0),
			LinearVelocity::default(),
		));
		world.spawn((Camera3d::default(), Transform::default()));
		world
			.run_system_once(place_training_grounds)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get_resource::<TrainingPlaced>().is_none());
		assert_eq!(world.query::<&TrainingFixture>().iter(&world).count(), 0);
		Ok(())
	}

	#[test]
	fn leave_parks_a_moved_player_on_the_region() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(TrainingGrounds(false));
		world.insert_resource(TerrainCellLayout::default());
		world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(
			&TerrainConfig::new(1),
		)));
		let entity = world
			.spawn((
				VegetationPlayer,
				TrainingPlacement,
				Transform::from_xyz(400.0, 12.0, -80.0),
				LinearVelocity(Vec3::X),
			))
			.id();
		world
			.run_system_once(park_player_after_training)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let transform = world.get::<Transform>(entity).unwrap();
		let center = TerrainCellLayout::default().region_center_xz().xz();
		assert!((transform.translation.xz() - center).length() < 1e-3);
		assert!(world.get::<TrainingPlacement>(entity).is_none());
		assert!(world.get::<AwaitingTerrainSurface>(entity).is_some());
		Ok(())
	}
}
