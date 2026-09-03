//! Duel vs free-for-all session. FFA rebuilds the field from generated loadouts.

use bevy::prelude::*;
use crozon_character_items::{FirearmStats, ItemRng};
use crozon_characters::{species::braidman::BraidmanConfig, CharacterRecipe, CharacterRoot};
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

use crate::damage::{CombatRespawn, Health};
use crate::engagement::NpcEngagement;
use crate::les_halles::LesHallesSpawn;
use crate::loadout::{roll_combatant, CombatantLoadout};

pub(crate) const DEFAULT_FFA_NPCS: u16 = 6;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum RangeMode {
	#[default]
	Duel,
	FreeForAll,
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

	pub fn is_free_for_all(&self) -> bool {
		self.mode == RangeMode::FreeForAll
	}
}

#[derive(Resource, Default)]
pub(crate) struct AppliedSession {
	pub epoch: u32,
}

#[derive(Component, Clone, Debug)]
pub(crate) struct CombatantKit {
	pub appearance: BraidmanConfig,
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
	(Or<(With<Player>, With<Npc>)>, Without<FirearmUser>),
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
	if session.mode == RangeMode::FreeForAll {
		rng.0 = session.seed.map(ItemRng::from_seed).unwrap_or_else(ItemRng::from_entropy);
		rebuild_free_for_all(
			&mut commands,
			&spawn,
			&mut rng.0,
			session.npc_count,
			&mut meshes,
			&mut materials,
		);
	} else {
		crate::spawn_player_at(&mut commands, &spawn, &mut meshes, &mut materials);
		crate::spawn_npc_at(&mut commands, &spawn, &mut meshes, &mut materials);
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
	let player = spawn_player_with_hidden_capsule(commands, meshes, materials);
	commands.entity(player).insert((
		Transform::from_translation(spawn.player),
		PlayerLook { yaw: spawn.look_yaw, ..default() },
		Health::from_max(loadout.sheet.health as f32),
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
	let mut movement =
		MovementIntelligence::new(MovementObjective::Reach(MovementLocation::new(at, 0.4)));
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
	));
	if let Some(kit) = kit {
		entity.insert(kit);
	}
}

fn kit_component(loadout: &CombatantLoadout) -> CombatantKit {
	CombatantKit {
		appearance: loadout.appearance.clone(),
		firearm: loadout.kit,
		live: live_weapon_from_stats(loadout.stats, loadout.sheet.damage),
		stats: loadout.stats,
	}
}

fn npc_translation(spawn: &LesHallesSpawn, index: u16, count: u16) -> Vec3 {
	let count = count.max(1) as f32;
	let t = if count <= 1.0 { 0.5 } else { index as f32 / (count - 1.0) };
	let yaw = spawn.look_yaw + (t - 0.5) * 2.4;
	let radius = 9.0;
	Vec3::new(
		spawn.player.x + yaw.sin() * radius,
		spawn.player.y,
		spawn.player.z - yaw.cos() * radius,
	)
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
		let firearm = kit.map(|kit| kit.firearm).unwrap_or_else(|| FirearmConcept::Bullpup.kit());
		let live = kit.map(|kit| kit.live).unwrap_or_default();
		spawn_held_kit(&mut commands, body, settings, firearm, live);
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
		spawn_npc_visual(&mut commands, npc, clothed, Quat::from_rotation_y(FRAC_PI_2));
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
		spawn_player_visual(&mut commands, player, clothed, Quat::from_rotation_y(FRAC_PI_2));
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
		assert!(a.distance(spawn.player) > 7.0);
		assert!(a.distance(b) > 1.0);
	}
}
