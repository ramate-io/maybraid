//! Duel vs free-for-all session. FFA rebuilds the field from generated loadouts.

use bevy::prelude::*;
use crozon_character_items::{FirearmSpec, FirearmStats, ItemRng};
use crozon_characters::{
	species::braidman::BraidmanConfig, CharacterRecipe, CharacterRoot, LocomotionCapsule,
};
use firearm_intelligence::{
	FirearmIntelligence, FirearmMovementIntelligence, FirearmMovementObjective, FirearmObjective,
	FirearmSpotting,
};
use firearm_user::{
	live_weapon_from_stats, spawn_held_kit, FirearmUser, FirearmUserSettings, LiveWeapon,
};
use firearms::FirearmConcept;
use movement_intelligence::{MovementIntelligence, MovementLocation, MovementObjective};
use player::{
	spawn_npc_visual, spawn_npc_with_hidden_capsule, spawn_player_visual,
	spawn_player_with_hidden_capsule, Npc, Player, PlayerLook, PlayerVisual,
};
use std::f32::consts::FRAC_PI_2;

use crate::damage::{headshot_band_for, CombatRespawn, Health};
use crate::engagement::NpcEngagement;
use crate::les_halles::LesHallesSpawn;
use crate::loadout::{roll_combatant, CombatantLoadout};
use crate::spec_kit::RolledFirearm;

pub(crate) const DEFAULT_FFA_NPCS: u16 = 6;

/// Outside the 36 m Les Halles stack, on the pad. Grows with count so neighbors stay apart.
const FFA_RING_MIN: f32 = 24.0;
const FFA_RING_MAX: f32 = 36.0;
const FFA_NEIGHBOR: f32 = 14.0;
const FFA_PLAYER_CLEARANCE: f32 = 10.0;
/// Inside the tower footprint, on the upper storey gallery.
const FFA_UPPER_RING_MIN: f32 = 10.0;
const FFA_UPPER_RING_MAX: f32 = 15.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RangeMode {
	#[default]
	Duel,
	FreeForAll,
	TestDummy,
}

#[derive(Resource, Debug)]
pub(crate) struct RangeSession {
	pub mode: RangeMode,
	pub npc_count: u16,
	pub seed: Option<u64>,
	pub epoch: u32,
}

impl Default for RangeSession {
	fn default() -> Self {
		Self { mode: RangeMode::Duel, npc_count: 1, seed: None, epoch: 0 }
	}
}

impl RangeSession {
	pub fn enter_duel(&mut self) {
		self.mode = RangeMode::Duel;
		self.npc_count = 1;
		self.epoch = self.epoch.wrapping_add(1);
	}

	pub fn enter_free_for_all(&mut self, npcs: u16, seed: Option<u64>) {
		self.mode = RangeMode::FreeForAll;
		self.npc_count = npcs.max(1);
		self.seed = seed;
		self.epoch = self.epoch.wrapping_add(1);
	}

	pub fn enter_test_dummy(&mut self) {
		self.mode = RangeMode::TestDummy;
		self.npc_count = 1;
		self.epoch = self.epoch.wrapping_add(1);
	}

	pub fn is_free_for_all(&self) -> bool {
		self.mode == RangeMode::FreeForAll
	}

	pub fn is_test_dummy(&self) -> bool {
		self.mode == RangeMode::TestDummy
	}
}

#[derive(Resource, Default)]
pub(crate) struct AppliedSession {
	pub epoch: u32,
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub(crate) struct TestDummy;

#[derive(Component, Clone, Debug)]
pub(crate) struct CombatantKit {
	pub appearance: BraidmanConfig,
	pub spec: FirearmSpec,
	pub firearm: firearms::FirearmKit,
	pub live: LiveWeapon,
	pub stats: FirearmStats,
}

#[derive(Resource)]
pub(crate) struct LoadoutRng(pub ItemRng);

type Combatants<'w, 's> =
	Query<'w, 's, (Entity, Option<&'static FirearmUser>), Or<(With<Player>, With<Npc>)>>;
type UnarmedBodies<'w, 's> = Query<
	'w,
	's,
	(Entity, Has<Npc>, Option<&'static CombatantKit>),
	(Or<(With<Player>, With<Npc>)>, Without<FirearmUser>, Without<TestDummy>),
>;

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_session(
	session: Res<RangeSession>,
	mut applied: ResMut<AppliedSession>,
	mut engagement: ResMut<NpcEngagement>,
	mut respawn: ResMut<CombatRespawn>,
	mut rng: ResMut<LoadoutRng>,
	spawn: Res<LesHallesSpawn>,
	combatants: Combatants,
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	if session.epoch == applied.epoch {
		return;
	}
	applied.epoch = session.epoch;
	engagement.reset();
	respawn.clear();
	despawn_combatants(&mut commands, combatants);
	match session.mode {
		RangeMode::FreeForAll => {
			rng.0 = session.seed.map(ItemRng::from_seed).unwrap_or_else(ItemRng::from_entropy);
			rebuild_free_for_all(
				&mut commands,
				&spawn,
				&mut rng.0,
				session.npc_count,
				&mut meshes,
				&mut materials,
			);
		}
		RangeMode::Duel => {
			crate::spawn_player_at(&mut commands, &spawn, &mut meshes, &mut materials);
			crate::spawn_npc_at(&mut commands, &spawn, &mut meshes, &mut materials);
		}
		RangeMode::TestDummy => {
			crate::spawn_player_at(&mut commands, &spawn, &mut meshes, &mut materials);
			crate::spawn_dummy_at(&mut commands, &spawn, &mut meshes, &mut materials);
		}
	}
}

pub(crate) fn spawn_generated_player(
	commands: &mut Commands,
	spawn: &LesHallesSpawn,
	rng: &mut ItemRng,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	let loadout = roll_combatant(rng);
	let hull = hull_from_appearance(&loadout.appearance);
	let player = spawn_player_with_hidden_capsule(commands, meshes, materials);
	commands.entity(player).insert((
		Transform::from_translation(spawn.player),
		PlayerLook { yaw: spawn.look_yaw, ..default() },
		Health::from_max(loadout.sheet.health as f32),
		headshot_band_for(hull),
		kit_component(&loadout),
	));
}

pub(crate) fn spawn_generated_npc(
	commands: &mut Commands,
	spawn: &LesHallesSpawn,
	index: u16,
	count: u16,
	rng: &mut ItemRng,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	let loadout = roll_combatant(rng);
	let translation = npc_translation(spawn, index, count);
	let npc = spawn_npc_with_hidden_capsule(
		commands,
		translation,
		PlayerLook { yaw: spawn.look_yaw, ..default() },
		meshes,
		materials,
	);
	install_npc_combat(
		commands,
		npc,
		translation,
		Some(kit_component(&loadout)),
		Some(Health::from_max(loadout.sheet.health as f32)),
	);
}

fn rebuild_free_for_all(
	commands: &mut Commands,
	spawn: &LesHallesSpawn,
	rng: &mut ItemRng,
	npc_count: u16,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	spawn_generated_player(commands, spawn, rng, meshes, materials);
	for index in 0..npc_count {
		spawn_generated_npc(commands, spawn, index, npc_count, rng, meshes, materials);
	}
}

pub(crate) fn install_npc_combat(
	commands: &mut Commands,
	npc: Entity,
	at: Vec3,
	kit: Option<CombatantKit>,
	health: Option<Health>,
) {
	let hull = hull_from_kit(kit.as_ref());
	let mut movement =
		MovementIntelligence::new(MovementObjective::Reach(MovementLocation::new(at, hull.radius)));
	movement.ability.agent_radius = hull.radius;
	movement.ability.feet_below_origin = hull.half_height();
	movement.ability.candidate_budget.horizon = 80.0;
	let mut combat = FirearmIntelligence::new(FirearmObjective::default());
	combat.settings.accuracy = 0.88;
	combat.settings.vision = 9;
	combat.settings.trigger_happiness = 0.9;
	let mut combat_movement = FirearmMovementIntelligence::new(FirearmMovementObjective::default());
	combat_movement.settings.range = (8.0, 1.0);
	combat_movement.settings.cover = 0.5;
	combat_movement.settings.flee = (0.0, 8.0);
	let mut entity = commands.entity(npc);
	entity.insert((
		movement,
		combat_movement,
		combat,
		FirearmSpotting::default(),
		health.unwrap_or_default(),
		headshot_band_for(hull),
	));
	if let Some(kit) = kit {
		entity.insert(kit);
	}
}

fn hull_from_appearance(appearance: &BraidmanConfig) -> LocomotionCapsule {
	appearance.locomotion_capsule()
}

fn hull_from_kit(kit: Option<&CombatantKit>) -> LocomotionCapsule {
	kit.map(|kit| hull_from_appearance(&kit.appearance)).unwrap_or_default()
}

fn kit_component(loadout: &CombatantLoadout) -> CombatantKit {
	CombatantKit {
		appearance: loadout.appearance.clone(),
		spec: loadout.spec,
		firearm: loadout.kit,
		live: live_weapon_from_stats(loadout.stats, loadout.sheet.damage),
		stats: loadout.stats,
	}
}

fn npc_translation(spawn: &LesHallesSpawn, index: u16, count: u16) -> Vec3 {
	let count = count.max(1);
	let split = spawn.has_upper() && count >= 2;
	let on_upper = split && index % 2 == 1;
	let (slot, slots, radius, y) = if on_upper {
		let slots = count / 2;
		let radius = (FFA_NEIGHBOR * slots as f32 / std::f32::consts::TAU)
			.max(FFA_UPPER_RING_MIN)
			.min(FFA_UPPER_RING_MAX);
		(index / 2, slots, radius, spawn.floor_y[1])
	} else {
		let slots = if split { (count + 1) / 2 } else { count };
		let slot = if split { index / 2 } else { index };
		let radius = (FFA_NEIGHBOR * slots as f32 / std::f32::consts::TAU)
			.max(FFA_RING_MIN)
			.min(FFA_RING_MAX);
		(slot, slots, radius, spawn.floor_y[0])
	};
	let theta = spawn.look_yaw + std::f32::consts::TAU * (slot as f32 + 0.5) / slots.max(1) as f32;
	let mut pos = Vec3::new(theta.sin() * radius, y, -theta.cos() * radius);
	let planar = Vec2::new(pos.x - spawn.player.x, pos.z - spawn.player.z);
	if planar.length() < FFA_PLAYER_CLEARANCE {
		let dir = planar.normalize_or(Vec2::new(theta.sin(), -theta.cos()));
		pos.x = spawn.player.x + dir.x * FFA_PLAYER_CLEARANCE;
		pos.z = spawn.player.z + dir.y * FFA_PLAYER_CLEARANCE;
	}
	pos
}

fn despawn_combatants(commands: &mut Commands, combatants: Combatants) {
	for (entity, user) in &combatants {
		if let Some(user) = user {
			commands.entity(user.held).try_despawn();
		}
		commands.entity(entity).try_despawn();
	}
}

pub(crate) fn spawn_held_system(mut commands: Commands, bodies: UnarmedBodies) {
	for (body, is_npc, kit) in &bodies {
		let mut settings = FirearmUserSettings::default();
		if is_npc {
			settings.aim_yaw_limit = std::f32::consts::PI;
		}
		let live = kit.map(|kit| kit.live).unwrap_or_default();
		if let Some(kit) = kit {
			spawn_held_kit(&mut commands, body, settings, RolledFirearm::from_spec(kit.spec), live);
		} else {
			spawn_held_kit(&mut commands, body, settings, FirearmConcept::Bullpup.kit(), live);
		}
	}
}

pub(crate) fn spawn_npc_character(
	mut commands: Commands,
	npcs: Query<(Entity, Option<&CombatantKit>), With<Npc>>,
	visuals: Query<&ChildOf, With<CharacterRoot>>,
) {
	for (npc, kit) in &npcs {
		if visuals.iter().any(|child| child.parent() == npc) {
			continue;
		}
		let appearance = kit
			.map(|kit| kit.appearance.clone())
			.unwrap_or_else(BraidmanConfig::default_preview);
		let clothed = CharacterRecipe::clothed(&appearance);
		let hull = appearance.locomotion_capsule();
		spawn_npc_visual(&mut commands, npc, clothed, Quat::from_rotation_y(FRAC_PI_2));
		commands.entity(npc).insert(headshot_band_for(hull));
	}
}

pub(crate) fn spawn_player_character(
	mut commands: Commands,
	players: Query<(Entity, Option<&CombatantKit>), With<Player>>,
	visuals: Query<&ChildOf, With<PlayerVisual>>,
) {
	for (player, kit) in &players {
		if visuals.iter().any(|child| child.parent() == player) {
			continue;
		}
		let appearance = kit
			.map(|kit| kit.appearance.clone())
			.unwrap_or_else(BraidmanConfig::default_preview);
		let clothed = CharacterRecipe::clothed(&appearance);
		let hull = appearance.locomotion_capsule();
		spawn_player_visual(&mut commands, player, clothed, Quat::from_rotation_y(FRAC_PI_2));
		commands.entity(player).insert(headshot_band_for(hull));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn npc_ring_stays_on_the_spawn_plane() {
		let spawn = LesHallesSpawn::default();
		let a = npc_translation(&spawn, 0, 6);
		let b = npc_translation(&spawn, 5, 6);
		assert!((a.y - spawn.player.y).abs() < 1e-4);
		assert!((b.y - spawn.player.y).abs() < 1e-4);
		assert!(a.distance(spawn.player) > 18.0);
		assert!(a.distance(b) > 12.0);
	}

	#[test]
	fn npc_ring_keeps_neighbors_apart() {
		let spawn = LesHallesSpawn::default();
		let poses: Vec<Vec3> = (0..6).map(|index| npc_translation(&spawn, index, 6)).collect();
		for i in 0..poses.len() {
			for j in (i + 1)..poses.len() {
				assert!(
					poses[i].distance(poses[j]) > 12.0,
					"npcs {i} and {j} are too close: {}",
					poses[i].distance(poses[j])
				);
			}
		}
	}

	#[test]
	fn npc_ring_clears_an_offset_player() {
		let mut spawn = LesHallesSpawn::default();
		spawn.player.x = 16.0;
		spawn.player.z = 4.0;
		for index in 0..8 {
			let at = npc_translation(&spawn, index, 8);
			let xz = Vec2::new(at.x - spawn.player.x, at.z - spawn.player.z).length();
			assert!(xz >= FFA_PLAYER_CLEARANCE - 1e-3);
		}
	}

	#[test]
	fn npc_ring_uses_both_storeys() {
		let mut spawn = LesHallesSpawn::default();
		spawn.floor_y = [spawn.player.y, spawn.player.y + 5.0];
		let poses: Vec<Vec3> = (0..6).map(|index| npc_translation(&spawn, index, 6)).collect();
		let ground = poses.iter().filter(|p| (p.y - spawn.floor_y[0]).abs() < 1e-3).count();
		let upper = poses.iter().filter(|p| (p.y - spawn.floor_y[1]).abs() < 1e-3).count();
		assert_eq!(ground, 3);
		assert_eq!(upper, 3);
		for (i, p) in poses.iter().enumerate() {
			if i % 2 == 1 {
				let xz = Vec2::new(p.x, p.z).length();
				assert!(xz < 18.0, "upper npc {i} should sit on the deck, xz={xz}");
			} else {
				let xz = Vec2::new(p.x, p.z).length();
				assert!(xz > 18.0, "ground npc {i} should sit on the pad, xz={xz}");
			}
		}
	}

	#[test]
	fn test_dummy_mode_is_a_single_inert_target() {
		let mut session = RangeSession::default();
		session.enter_test_dummy();
		assert!(session.is_test_dummy());
		assert_eq!(session.npc_count, 1);
		assert_ne!(session.mode, RangeMode::Duel);
	}
}
