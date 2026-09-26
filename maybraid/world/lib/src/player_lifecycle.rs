//! Downed world-player retirement and POI-based replacement.

use avian3d::prelude::LinearVelocity;
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chico_vegetation_on_terrain_playground::{
	player_position_above_surface, spawn_player_body, CharacterLocomotion, CharacterSpecies,
	MoveWish, Player as VegetationPlayer, RequestSetCharacter, RequestSetCharacterAppearance,
	WorldBaseTerrain,
};
use crozon_character_ragdoll::CharacterRagdollSystems;
use crozon_inventory_user::InventoryUser;
use damage::{DamageSystems, DespawnAfter, Downed};
use durham_terrain_models::{TerrainCellLayout, TerrainEntryStore};
use firearm_user::FirearmUser;
use firearms::WeaponTrigger;
use mob_characters::{LOCAL_POI, URBAN_POI, VEGETATION_POI};
use player::{CameraFollow, Player as MaybraidPlayer, PlayerUse};
use poi_intelligence::{
	mix_seed, NearbyFallback, NearbyQuery, PoiId, PoiInterest, PoiInterests, PoiRegistry,
	PoiSystems, DEFAULT_NEARBY_RADIUS,
};
use richmond_development_models::DevelopmentEntryStore;
use spotting_intelligence::SpotSubject;
use threat_intelligence::{Affiliations, ThreatSubject};

use crate::control::strip_world_player_motor;
use crate::training::{TrainingGrounds, TrainingLifeEnded};
use crate::weapon::WorldPlayerAppearanceRequested;
use crate::{WorldGameplayEnabled, WorldPlayerLoadout};

/// World-player downed duration, nearby POI scan, and replacement interests.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct WorldPlayerRespawnConfig {
	pub delay_secs: f32,
	pub poi_radius: f32,
	pub fallback: NearbyFallback,
	pub interests: PoiInterests,
}

impl Default for WorldPlayerRespawnConfig {
	fn default() -> Self {
		Self {
			delay_secs: 4.0,
			poi_radius: DEFAULT_NEARBY_RADIUS,
			fallback: NearbyFallback::new(60.0, 100.0),
			interests: default_player_respawn_interests(),
		}
	}
}

impl WorldPlayerRespawnConfig {
	/// Nearest POI outside the fallback ring's inner radius.
	pub fn nearby_query(&self) -> NearbyQuery {
		NearbyQuery::nearest_beyond(self.poi_radius, self.fallback.min_radius)
	}
}

#[derive(Debug)]
struct PendingPlayerRespawn {
	timer: Timer,
	death_at: Vec3,
	seed: u64,
	training: bool,
}

impl PendingPlayerRespawn {
	/// A Training death whose session has since ended. Its body replaces the
	/// dead one at once, so the Leave teardown and the next session's resume
	/// have a player to seat.
	fn abandoned(&self, training_now: bool) -> bool {
		self.training && !training_now
	}
}

#[derive(Resource, Default)]
struct WorldPlayerRespawnState {
	pending: Option<PendingPlayerRespawn>,
	generation: u64,
	last_poi: Option<PoiId>,
}

#[derive(Component)]
struct PlayerDeathGlaze;

#[derive(SystemParam)]
struct WorldPlayerSurface<'w> {
	terrain: Res<'w, TerrainEntryStore>,
	layout: Res<'w, TerrainCellLayout>,
	base: Res<'w, WorldBaseTerrain>,
	developments: Res<'w, DevelopmentEntryStore>,
}

impl WorldPlayerSurface<'_> {
	fn surface_height(&self, xz: Vec2) -> f32 {
		let raw = self
			.terrain
			.composed_height_at(&self.layout, xz.x, xz.y)
			.unwrap_or_else(|| self.base.0.height_at(xz.x, xz.y));
		let probe = Aabb3d::from_min_max(
			Vec3::new(xz.x - 0.5, -10_000.0, xz.y - 0.5),
			Vec3::new(xz.x + 0.5, 10_000.0, xz.y + 0.5),
		);
		self.developments.merged_pad_complex(probe).modify_elevation(raw, xz.x, xz.y)
	}
}

type DownedWorldPlayer<'a> = (
	Entity,
	&'a Transform,
	&'a mut LinearVelocity,
	Option<&'a FirearmUser>,
	Option<&'a InventoryUser>,
);

pub struct WorldPlayerLifecyclePlugin;

impl Plugin for WorldPlayerLifecyclePlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<WorldPlayerRespawnConfig>()
			.init_resource::<WorldPlayerRespawnState>()
			.add_systems(Startup, spawn_player_death_glaze)
			.add_systems(
				PostUpdate,
				queue_downed_world_player
					.after(DamageSystems::Down)
					.after(CharacterRagdollSystems::Handoff),
			)
			.add_systems(Update, respawn_world_player.after(PoiSystems::Index))
			.add_systems(Update, sync_player_death_glaze.after(respawn_world_player));
	}
}

fn spawn_player_death_glaze(mut commands: Commands) {
	commands.spawn((
		Name::new("player-death-glaze"),
		PlayerDeathGlaze,
		Node {
			position_type: PositionType::Absolute,
			left: Val::Px(0.0),
			top: Val::Px(0.0),
			width: Val::Percent(100.0),
			height: Val::Percent(100.0),
			..default()
		},
		BackgroundColor(death_glaze_color(0.0)),
		GlobalZIndex(i32::MAX - 4),
		Visibility::Hidden,
		Pickable::IGNORE,
	));
}

fn sync_player_death_glaze(
	state: Res<WorldPlayerRespawnState>,
	mut glaze: Query<(&mut BackgroundColor, &mut Visibility), With<PlayerDeathGlaze>>,
) {
	let Ok((mut color, mut visibility)) = glaze.single_mut() else {
		return;
	};
	let Some(pending) = state.pending.as_ref() else {
		color.0 = death_glaze_color(0.0);
		*visibility = Visibility::Hidden;
		return;
	};
	color.0 = death_glaze_color(death_glaze_alpha(&pending.timer));
	*visibility = Visibility::Visible;
}

fn queue_downed_world_player(
	config: Res<WorldPlayerRespawnConfig>,
	grounds: Option<Res<TrainingGrounds>>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut commands: Commands,
	mut players: Query<DownedWorldPlayer<'_>, (With<VegetationPlayer>, Added<Downed>)>,
	mut triggers: Query<&mut WeaponTrigger>,
) {
	for (player, transform, mut velocity, firearm, inventory) in &mut players {
		state.generation = state.generation.wrapping_add(1);
		let seed = respawn_seed(state.generation, transform.translation);
		state.pending = Some(PendingPlayerRespawn {
			timer: Timer::from_seconds(config.delay_secs.max(0.0), TimerMode::Once),
			death_at: transform.translation,
			seed,
			training: grounds.as_deref().is_some_and(|grounds| grounds.0),
		});
		velocity.0 = Vec3::ZERO;
		if let Some(firearm) = firearm {
			if let Ok(mut trigger) = triggers.get_mut(firearm.held) {
				trigger.0 = false;
			}
			commands.entity(firearm.held).try_insert(DespawnAfter::seconds(0.0));
		}
		if let Some(inventory) = inventory {
			commands.entity(inventory.bag).try_despawn();
		}
		strip_world_player_motor(&mut commands, player);
		commands.entity(player).remove::<(
			VegetationPlayer,
			MaybraidPlayer,
			CameraFollow,
			PlayerUse,
			FirearmUser,
			InventoryUser,
			MoveWish,
			SpotSubject,
			ThreatSubject,
			Affiliations,
		)>();
	}
}

/// Discovery respawns near a POI. A Training respawn ends the life: the body
/// is replaced where it fell until the shell's next round seats it. Leaving
/// Training mid-respawn replaces it at once and ends nothing.
#[allow(clippy::too_many_arguments)]
fn respawn_world_player(
	time: Res<Time>,
	gameplay: Res<WorldGameplayEnabled>,
	config: Res<WorldPlayerRespawnConfig>,
	registry: Res<PoiRegistry>,
	loadout: Option<Res<WorldPlayerLoadout>>,
	locomotion: Res<CharacterLocomotion>,
	surface: WorldPlayerSurface,
	grounds: Option<Res<TrainingGrounds>>,
	mut ended: MessageWriter<TrainingLifeEnded>,
	live_player: Query<(), With<VegetationPlayer>>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	let training_now = grounds.is_some_and(|grounds| grounds.0);
	let abandoned =
		state.pending.as_ref().is_some_and(|pending| pending.abandoned(training_now));
	if !gameplay.0 && !abandoned {
		return;
	}
	if !live_player.is_empty() {
		state.pending = None;
		return;
	}
	let Some(pending) = state.pending.as_mut() else {
		return;
	};
	pending.timer.tick(time.delta());
	if !pending.timer.is_finished() && !abandoned {
		return;
	}
	let death_at = pending.death_at;
	let seed = pending.seed;
	state.pending = None;

	let position = if training_now || abandoned {
		if training_now {
			ended.write(TrainingLifeEnded);
		}
		player_position_above_surface(death_at)
	} else {
		let placed = registry.place_nearby(
			death_at,
			config.nearby_query(),
			&config.interests,
			state.last_poi,
			seed,
			config.fallback,
		);
		let mut surface_point = placed.position;
		let terrain_y = surface.surface_height(surface_point.xz());
		if terrain_y.is_finite() {
			surface_point.y = terrain_y;
		}
		state.last_poi = placed.poi;
		player_position_above_surface(surface_point)
	};

	let player = spawn_player_body(
		&mut commands,
		&mut meshes,
		&mut materials,
		locomotion.as_ref(),
		position,
	);
	crate::control::apply_world_player_motor(&mut commands, player);
	if let Some(loadout) = loadout {
		// The next life may swap the loadout (a new trainee) before the body
		// arms, so a Training body leaves its appearance to the armed loadout.
		if !training_now {
			commands.entity(player).insert(WorldPlayerAppearanceRequested);
		}
		commands.spawn(RequestSetCharacterAppearance { appearance: loadout.appearance.clone() });
	} else {
		commands.spawn(RequestSetCharacter { species: CharacterSpecies::Braidman });
	}
}

fn death_glaze_alpha(timer: &Timer) -> f32 {
	let elapsed = timer.elapsed_secs();
	let remaining = timer.remaining_secs();
	let fade_in = (elapsed / 0.18).clamp(0.0, 1.0);
	let fade_out = (remaining / 0.35).clamp(0.0, 1.0);
	0.68 * fade_in * fade_out
}

fn death_glaze_color(alpha: f32) -> Color {
	Color::srgba(0.2, 0.005, 0.025, alpha)
}

fn default_player_respawn_interests() -> PoiInterests {
	PoiInterests::new([
		PoiInterest::new(LOCAL_POI, 1.25),
		PoiInterest::new(URBAN_POI, 1.5),
		PoiInterest::new(VEGETATION_POI, 1.0),
	])
}

fn respawn_seed(generation: u64, death_at: Vec3) -> u64 {
	mix_seed(
		generation
			^ u64::from(death_at.x.to_bits()).rotate_left(11)
			^ u64::from(death_at.y.to_bits()).rotate_left(29)
			^ u64::from(death_at.z.to_bits()).rotate_left(47),
	)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

	#[test]
	fn fallback_respawn_moves_away_from_the_death_point() {
		let death = Vec3::new(10.0, 4.0, -5.0);
		let config = WorldPlayerRespawnConfig::default();
		let placed = poi_intelligence::place_nearby(
			None,
			death,
			config.nearby_query(),
			None,
			None,
			42,
			config.fallback,
		);
		let distance = (placed.position - death).xz().length();
		assert!((config.fallback.min_radius..=config.fallback.max_radius).contains(&distance));
		assert_eq!(placed.position.y, death.y);
		assert!(placed.poi.is_none());
	}

	#[test]
	fn player_respawn_prefers_urban_pois() {
		let interests = WorldPlayerRespawnConfig::default().interests;
		assert_eq!(interests.weight(URBAN_POI), Some(1.5));
		assert_eq!(interests.weight(LOCAL_POI), Some(1.25));
		assert!(interests.contains(VEGETATION_POI));
	}

	#[test]
	fn default_respawn_waits_four_seconds_and_scans_nearby() {
		let config = WorldPlayerRespawnConfig::default();
		assert_eq!(config.delay_secs, 4.0);
		assert_eq!(config.poi_radius, DEFAULT_NEARBY_RADIUS);
		assert_eq!(config.fallback, NearbyFallback::new(60.0, 100.0));
		assert_eq!(config.nearby_query().min_radius, config.fallback.min_radius);
	}

	#[test]
	fn death_glaze_fades_in_and_out() {
		let mut timer = Timer::from_seconds(4.0, TimerMode::Once);
		assert_eq!(death_glaze_alpha(&timer), 0.0);
		timer.tick(std::time::Duration::from_secs_f32(0.5));
		assert!((death_glaze_alpha(&timer) - 0.68).abs() < 1e-5);
		timer.tick(std::time::Duration::from_secs_f32(3.4));
		assert!(death_glaze_alpha(&timer) < 0.3);
	}

	#[test]
	fn downed_player_is_retired_and_queues_a_replacement() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<WorldPlayerRespawnConfig>();
		world.init_resource::<WorldPlayerRespawnState>();
		let held = world.spawn(WeaponTrigger(true)).id();
		let bag = world.spawn_empty().id();
		let player = world
			.spawn((
				VegetationPlayer,
				MaybraidPlayer,
				CameraFollow,
				Transform::from_xyz(3.0, 4.0, 5.0),
				LinearVelocity(Vec3::X),
				FirearmUser::holding(held),
				InventoryUser::carrying(bag),
				Downed { source: None, point: Vec3::ZERO, at: 0.0 },
			))
			.id();

		world
			.run_system_once(queue_downed_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.get::<VegetationPlayer>(player).is_none());
		assert!(world.get::<MaybraidPlayer>(player).is_none());
		assert_eq!(
			world.get::<LinearVelocity>(player).map(|velocity| velocity.0),
			Some(Vec3::ZERO)
		);
		assert_eq!(world.get::<WeaponTrigger>(held).map(|trigger| trigger.0), Some(false));
		assert!(world.get::<DespawnAfter>(held).is_some());
		assert!(!world.entities().contains(bag));
		let pending = &world.resource::<WorldPlayerRespawnState>().pending;
		assert!(pending.is_some());
		Ok(())
	}

	fn respawn_world(timer_secs: f32, gameplay: bool, grounds: bool) -> World {
		let mut world = World::new();
		world.insert_resource(WorldPlayerRespawnConfig { delay_secs: timer_secs, ..default() });
		world.insert_resource(WorldPlayerRespawnState {
			pending: Some(PendingPlayerRespawn {
				timer: Timer::from_seconds(timer_secs, TimerMode::Once),
				death_at: Vec3::new(3.0, 4.0, 5.0),
				seed: 7,
				training: true,
			}),
			..default()
		});
		world.insert_resource(Time::<()>::default());
		world.insert_resource(WorldGameplayEnabled(gameplay));
		world.init_resource::<PoiRegistry>();
		world.init_resource::<CharacterLocomotion>();
		world.init_resource::<TerrainEntryStore>();
		world.init_resource::<TerrainCellLayout>();
		world.init_resource::<DevelopmentEntryStore>();
		world.insert_resource(WorldBaseTerrain(
			durham_terrain_models::BaseTerrainNoise::from_config(
				&durham_terrain_models::TerrainConfig::new(42),
			),
		));
		world.init_resource::<Assets<Mesh>>();
		world.init_resource::<Assets<StandardMaterial>>();
		world.init_resource::<Messages<TrainingLifeEnded>>();
		world.insert_resource(TrainingGrounds(grounds));
		world.insert_resource(crate::TrainingRound::new(9).trainee());
		world
	}

	#[test]
	fn a_training_respawn_ends_the_life() -> anyhow::Result<()> {
		let mut world = respawn_world(0.0, true, true);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let ended: Vec<_> =
			world.resource_mut::<Messages<TrainingLifeEnded>>().drain().collect();
		assert_eq!(ended, vec![TrainingLifeEnded]);
		let mut bodies = world.query_filtered::<
			(&Transform, Has<WorldPlayerAppearanceRequested>),
			With<VegetationPlayer>,
		>();
		let (body, requested) = bodies.single(&world)?;
		assert_eq!(body.translation.xz(), Vec2::new(3.0, 5.0));
		assert!(!requested, "the next life's loadout dresses the body");
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none());
		Ok(())
	}

	#[test]
	fn leaving_training_mid_respawn_replaces_the_body_at_once() -> anyhow::Result<()> {
		let mut world = respawn_world(4.0, false, true);
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(
			world.resource::<WorldPlayerRespawnState>().pending.is_some(),
			"a paused Training death keeps waiting"
		);

		world.insert_resource(TrainingGrounds(false));
		world
			.run_system_once(respawn_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let ended: Vec<_> =
			world.resource_mut::<Messages<TrainingLifeEnded>>().drain().collect();
		assert!(ended.is_empty(), "leaving ends no life");
		let mut bodies = world.query_filtered::<(), With<VegetationPlayer>>();
		assert_eq!(bodies.iter(&world).count(), 1);
		assert!(world.resource::<WorldPlayerRespawnState>().pending.is_none(), "glaze clears");
		Ok(())
	}
}
