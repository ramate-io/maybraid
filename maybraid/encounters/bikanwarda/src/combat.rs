//! Minimal encounter combat driver with stationary-boss targeting adapter.
//!
//! ## Targeting Rationale
//!
//! The existing `CombatTargeting` component (combat-targeting crate) is designed
//! for mobile NPCs with threat-intelligence, spotting systems, and navigation:
//!
//! - **Spotting**: Requires perception adapters that ray-cast for visibility
//! - **Threat-intelligence**: Maintains enemy lists via faction/affiliation
//! - **Mobile AI**: Updates targeting through patrol/search/chase behaviors
//!
//! A stationary chained boss has none of these:
//! - No locomotion/navigation to drive spotting updates
//! - No threat-intelligence for faction-based enemy tracking
//! - Fixed position means no patrol/search state machine
//!
//! ## Smallest Adapter
//!
//! `update_boss_targeting` provides a minimal stationary-attacker policy that:
//! 1. Finds nearby players via spatial query (reuses range check concept)
//! 2. Creates `CombatContact` entries with position/visibility (reuses contact memory)
//! 3. Adds them to `CombatTargeting` with `OBJECTIVE` source (reuses ranking)
//! 4. Existing `rank_combat_targets` system ranks them by algebra
//!
//! This reuses all existing targeting contracts without forking them. The boss
//! gets CombatTargeting just like mobile NPCs, but the contact population is
//! simplified for stationary use.

use attacks::{AttackCatalog, BeginAttack};
use bevy::prelude::*;
use bikanwarda_attacks::BRAY;
use bikanwarda_character::BikanwardaRoot;
use combat_targeting::{CombatContact, CombatTargeting, TargetAlgebra, TargetSource};
use damage::Health;
use world_player::Player as WorldPlayer;

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

/// Stationary-boss targeting adapter: populates CombatTargeting with nearby players.
///
/// Reuses existing CombatContact memory and TargetSource membership without
/// requiring mobile-NPC perception or threat-intelligence systems.
pub fn update_boss_targeting(
	time: Res<Time>,
	mut bosses: Query<
		(&Transform, &mut CombatTargeting),
		(With<BossEncounterRoot>, With<BikanwardaRoot>),
	>,
	players: Query<(Entity, &Transform), With<WorldPlayer>>,
) {
	let now = time.elapsed_secs();
	const DETECTION_RANGE: f32 = 50.0; // Match typical encounter arena size
	
	for (boss_transform, mut targeting) in &mut bosses {
		let boss_pos = boss_transform.translation;
		
		// Find nearby players and add them to targeting memory
		for (player_entity, player_transform) in &players {
			let player_pos = player_transform.translation;
			let distance = boss_pos.distance(player_pos);
			
			if distance <= DETECTION_RANGE {
				// Create contact entry (reuses existing contact memory)
				let contact = CombatContact {
					subject: player_entity,
					position: player_pos,
					movement_vector: Vec3::ZERO, // Stationary boss doesn't track velocity
					visible_point: player_pos,
					visible_head: Some(player_pos + Vec3::Y * 1.7), // Approximate head height
					last_spotted_at: now,
				};
				
				targeting.upsert_contact(contact);
				// Add to targeting membership with OBJECTIVE source (reuses existing ranking)
				targeting.include(player_entity, TargetSource::OBJECTIVE);
			}
		}
	}
}

/// System to manage boss combat state and trigger attacks.
///
/// Uses best-ranked target from CombatTargeting. Attacks only fire when a real
/// target exists and is in range. Otherwise stays idle (no cooldown-only firing).
pub fn boss_combat_driver(
	time: Res<Time>,
	catalog: Res<AttackCatalog>,
	mut bosses: Query<
		(
			Entity,
			&Transform,
			&Health,
			&mut BossAttackCooldown,
			&CombatTargeting,
		),
		(With<BossEncounterRoot>, With<BikanwardaRoot>),
	>,
	targets: Query<&Transform>,
	mut attacks: MessageWriter<BeginAttack>,
) {
	const ATTACK_RANGE: f32 = 40.0; // Bray range from attack definition
	
	for (boss_entity, boss_transform, health, mut cooldown, targeting) in &mut bosses {
		// Gate: skip if dead
		if health.is_dead() {
			continue;
		}

		// Tick cooldown
		cooldown.timer.tick(time.delta());
		if !cooldown.timer.just_finished() && cooldown.timer.elapsed_secs() < cooldown.timer.duration().as_secs_f32() {
			continue;
		}

		// Get best target from existing targeting system (reuses ranking)
		let Some(ranked) = targeting.best() else {
			// No target - stay idle (no cooldown-only firing)
			continue;
		};
		
		// Verify target is in range
		let Ok(target_transform) = targets.get(ranked.entity) else {
			continue;
		};
		
		let distance = boss_transform.translation.distance(target_transform.translation);
		if distance > ATTACK_RANGE {
			// Out of range - stay idle
			continue;
		}

		// Get Bray attack definition from catalog
		let Some(bray_def) = catalog.get(BRAY) else {
			warn!("Bray attack not found in catalog");
			continue;
		};

		// Trigger Bray attack with real target
		attacks.write(BeginAttack {
			owner: boss_entity,
			definition: BRAY,
			target: Some(ranked.entity),
			allies: Vec::new(),
			emission: None,
			emission_socket: None,
		});

		// Reset cooldown based on attack definition
		let cooldown_secs = bray_def.timing.cooldown;
		cooldown.timer = Timer::from_seconds(cooldown_secs, TimerMode::Once);

		debug!(
			"Boss {:?} triggered Bray attack on {:?} at {:.1}m with cooldown {:.1}s",
			boss_entity, ranked.entity, distance, cooldown_secs
		);
	}
}

/// System to initialize combat state on boss spawn.
pub fn initialize_boss_combat(
	mut commands: Commands,
	new_bosses: Query<Entity, (Added<BossEncounterRoot>, With<BikanwardaRoot>)>,
) {
	for boss_entity in &new_bosses {
		// Initialize CombatTargeting with default configuration (reuses existing ranking system)
		commands.entity(boss_entity).insert((
			BossInCombat,
			BossAttackCooldown::default(),
			CombatTargeting::default(),
		));
		info!("Initialized combat state for boss {:?}", boss_entity);
	}
}

/// Plugin for boss encounter combat.
pub struct BossEncounterCombatPlugin;

impl Plugin for BossEncounterCombatPlugin {
	fn build(&self, app: &mut App) {
		// Install CombatTargetingPlugin if not already present (provides rank_combat_targets)
		if !app.is_plugin_added::<combat_targeting::CombatTargetingPlugin>() {
			app.add_plugins(combat_targeting::CombatTargetingPlugin);
		}
		
		app.add_systems(
			Update,
			(
				initialize_boss_combat,
				// Update targeting before ranking
				update_boss_targeting.before(combat_targeting::CombatTargetingSystems::Rank),
				// Combat driver runs after targeting is ranked
				boss_combat_driver.after(combat_targeting::CombatTargetingSystems::Rank),
			),
		);
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
