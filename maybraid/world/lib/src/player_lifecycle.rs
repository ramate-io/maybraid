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
use poi_intelligence::{PoiId, PoiInterest, PoiInterests, PoiRecord, PoiRegistry, PoiSystems};
use richmond_development_models::DevelopmentEntryStore;
use spotting_intelligence::SpotSubject;
use threat_intelligence::{Affiliations, ThreatSubject};

use crate::{WorldGameplayEnabled, WorldPlayerLoadout};

const PLAYER_RESPAWN_FALLBACK_MIN_RADIUS: f32 = 8.0;
const PLAYER_RESPAWN_FALLBACK_MAX_RADIUS: f32 = 16.0;

/// World-player downed duration and nearby POI search extent.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct WorldPlayerRespawnConfig {
	pub delay_secs: f32,
	pub poi_radius: f32,
}

impl Default for WorldPlayerRespawnConfig {
	fn default() -> Self {
		Self { delay_secs: 4.0, poi_radius: 160.0 }
	}
}

#[derive(Debug)]
struct PendingPlayerRespawn {
	timer: Timer,
	death_at: Vec3,
	seed: u64,
}

#[derive(Resource, Default)]
struct WorldPlayerRespawnState {
	pending: Option<PendingPlayerRespawn>,
	generation: u64,
	last_poi: Option<PoiId>,
}

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
			.add_systems(
				PostUpdate,
				queue_downed_world_player
					.after(DamageSystems::Down)
					.after(CharacterRagdollSystems::Handoff),
			)
			.add_systems(Update, respawn_world_player.after(PoiSystems::Index));
	}
}

fn queue_downed_world_player(
	config: Res<WorldPlayerRespawnConfig>,
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

#[allow(clippy::too_many_arguments)]
fn respawn_world_player(
	time: Res<Time>,
	gameplay: Res<WorldGameplayEnabled>,
	config: Res<WorldPlayerRespawnConfig>,
	registry: Res<PoiRegistry>,
	loadout: Option<Res<WorldPlayerLoadout>>,
	locomotion: Res<CharacterLocomotion>,
	surface: WorldPlayerSurface,
	live_player: Query<(), With<VegetationPlayer>>,
	mut state: ResMut<WorldPlayerRespawnState>,
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	if !gameplay.0 {
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
	if !pending.timer.is_finished() {
		return;
	}
	let death_at = pending.death_at;
	let seed = pending.seed;

	let interests = player_respawn_interests();
	let poi = registry.choose_nearby(death_at, config.poi_radius, &interests, state.last_poi, seed);
	let mut surface_point = poi.map_or_else(
		|| fallback_player_surface(death_at, seed),
		|poi| player_surface_at_poi(poi, seed),
	);
	let terrain_y = surface.surface_height(surface_point.xz());
	if terrain_y.is_finite() {
		surface_point.y = terrain_y;
	}
	let position = player_position_above_surface(surface_point);
	state.last_poi = poi.map(|poi| poi.id);
	state.pending = None;

	spawn_player_body(&mut commands, &mut meshes, &mut materials, locomotion.as_ref(), position);
	if let Some(loadout) = loadout {
		commands.spawn(RequestSetCharacterAppearance { appearance: loadout.appearance.clone() });
	} else {
		commands.spawn(RequestSetCharacter { species: CharacterSpecies::Braidman });
	}
}

fn player_respawn_interests() -> PoiInterests {
	PoiInterests::new([
		PoiInterest::new(LOCAL_POI, 1.25),
		PoiInterest::new(URBAN_POI, 1.5),
		PoiInterest::new(VEGETATION_POI, 1.0),
	])
}

fn player_surface_at_poi(poi: PoiRecord, seed: u64) -> Vec3 {
	let radius = poi.arrival_radius.clamp(2.0, 12.0);
	let distance = unit_f32(mixed(seed ^ 0x736f_6d65_706c_6179)).sqrt() * radius;
	let angle = unit_f32(mixed(seed ^ 0x6572_5f72_6573_7061)) * std::f32::consts::TAU;
	poi.position + Vec3::new(angle.cos() * distance, 0.0, angle.sin() * distance)
}

fn fallback_player_surface(death_at: Vec3, seed: u64) -> Vec3 {
	let t = unit_f32(seed);
	let distance = PLAYER_RESPAWN_FALLBACK_MIN_RADIUS
		+ t * (PLAYER_RESPAWN_FALLBACK_MAX_RADIUS - PLAYER_RESPAWN_FALLBACK_MIN_RADIUS);
	let angle = unit_f32(mixed(seed ^ 0x776f_726c_6470_6c79)) * std::f32::consts::TAU;
	death_at + Vec3::new(angle.cos() * distance, 0.0, angle.sin() * distance)
}

fn respawn_seed(generation: u64, death_at: Vec3) -> u64 {
	mixed(
		generation
			^ u64::from(death_at.x.to_bits()).rotate_left(11)
			^ u64::from(death_at.y.to_bits()).rotate_left(29)
			^ u64::from(death_at.z.to_bits()).rotate_left(47),
	)
}

fn mixed(mut value: u64) -> u64 {
	value ^= value >> 30;
	value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value ^= value >> 27;
	value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ (value >> 31)
}

fn unit_f32(value: u64) -> f32 {
	((value >> 40) as f32) / ((1_u32 << 24) as f32)
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

	#[test]
	fn fallback_respawn_moves_away_from_the_death_point() {
		let death = Vec3::new(10.0, 4.0, -5.0);
		let respawn = fallback_player_surface(death, 42);
		let distance = (respawn - death).xz().length();
		assert!((PLAYER_RESPAWN_FALLBACK_MIN_RADIUS..=PLAYER_RESPAWN_FALLBACK_MAX_RADIUS)
			.contains(&distance));
		assert_eq!(respawn.y, death.y);
	}

	#[test]
	fn player_respawn_prefers_urban_pois() {
		let interests = player_respawn_interests();
		assert_eq!(interests.weight(URBAN_POI), Some(1.5));
		assert_eq!(interests.weight(LOCAL_POI), Some(1.25));
		assert!(interests.contains(VEGETATION_POI));
	}

	#[test]
	fn default_respawn_waits_four_seconds_and_scans_nearby() {
		let config = WorldPlayerRespawnConfig::default();
		assert_eq!(config.delay_secs, 4.0);
		assert_eq!(config.poi_radius, 160.0);
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
}
