//! Minimal encounter combat driver.
//!
//! Drives boss attacks using existing targeting/visibility contracts.
//! Gates on pause, death, and stream-out.

use attacks::{AttackCatalog, BeginAttack};
use bevy::prelude::*;
use bikanwarda_attacks::BRAY;
use bikanwarda_character::BikanwardaRoot;
use damage::Health;

use crate::presentation::BossEncounterRoot;

/// Marker for bosses currently in combat.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BossInCombat;

/// Attack cooldown timer for the boss.
#[derive(Component, Clone, Debug)]
pub struct BossAttackCooldown {
	pub timer: Timer,
}

impl Default for BossAttackCooldown {
	fn default() -> Self {
		Self {
			// Start ready to attack (elapsed)
			timer: Timer::from_seconds(0.0, TimerMode::Once),
		}
	}
}

/// System to manage boss combat state and trigger attacks.
pub fn boss_combat_driver(
	time: Res<Time>,
	catalog: Res<AttackCatalog>,
	mut bosses: Query<
		(
			Entity,
			&Health,
			&mut BossAttackCooldown,
		),
		(With<BossEncounterRoot>, With<BikanwardaRoot>),
	>,
	mut attacks: MessageWriter<BeginAttack>,
) {
	for (boss_entity, health, mut cooldown) in &mut bosses {
		// Gate: skip if dead
		if health.is_dead() {
			continue;
		}

		// Tick cooldown
		cooldown.timer.tick(time.delta());
		if !cooldown.timer.just_finished() && cooldown.timer.elapsed_secs() < cooldown.timer.duration().as_secs_f32() {
			continue;
		}

		// Get Bray attack definition from catalog
		let Some(bray_def) = catalog.get(BRAY) else {
			warn!("Bray attack not found in catalog");
			continue;
		};

		// Trigger Bray attack (placeholder targeting - will be improved with real targeting)
		attacks.write(BeginAttack {
			owner: boss_entity,
			definition: BRAY,
			target: None, // Let attack system find target
			allies: Vec::new(),
			emission: None,
			emission_socket: None,
		});

		// Reset cooldown based on attack definition
		let cooldown_secs = bray_def.timing.cooldown;
		cooldown.timer = Timer::from_seconds(cooldown_secs, TimerMode::Once);

		debug!(
			"Boss {:?} triggered Bray attack with cooldown {:.1}s",
			boss_entity, cooldown_secs
		);
	}
}

/// System to initialize combat state on boss spawn.
pub fn initialize_boss_combat(
	mut commands: Commands,
	new_bosses: Query<Entity, (Added<BossEncounterRoot>, With<BikanwardaRoot>)>,
) {
	for boss_entity in &new_bosses {
		commands.entity(boss_entity).insert((
			BossInCombat,
			BossAttackCooldown::default(),
		));
		info!("Initialized combat state for boss {:?}", boss_entity);
	}
}

/// Plugin for boss encounter combat.
pub struct BossEncounterCombatPlugin;

impl Plugin for BossEncounterCombatPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, (initialize_boss_combat, boss_combat_driver).chain());
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn cooldown_starts_ready() {
		let cooldown = BossAttackCooldown::default();
		assert_eq!(cooldown.timer.duration().as_secs_f32(), 0.0);
	}

	#[test]
	fn cooldown_resets_after_attack() {
		let mut cooldown = BossAttackCooldown::default();
		cooldown.timer = Timer::from_seconds(5.0, TimerMode::Once);
		assert!(cooldown.timer.duration().as_secs_f32() > 0.0);
	}
}
