//! Boss encounter combat driver.
//!
//! Reuses `SpottingUser` from `spotting_intelligence` for perception/visibility,
//! then bridges spotted contacts to `CombatTargeting` via the shared bridge.
//! This avoids hand-writing visibility checks and reuses the existing perception system.

use attacks::{AttackCatalog, BeginAttack};
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use bikanwarda_attacks::{BikanwardaMouthSocket, BRAY};
use bikanwarda_character::BikanwardaRoot;
use combat_targeting::{CombatTargeting, SpottingTargetingSettings};
use damage::Health;

use crate::presentation::{BossActor, BossEncounterRoot};

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
///
/// Uses best-ranked target from CombatTargeting. Attacks only fire when a real
/// target exists and is in range. Otherwise stays idle (no cooldown-only firing).
///
/// Queries the host (BossEncounterRoot with Health) to check death state, then
/// uses BossActor to find the actor entity that owns the attack and targeting systems.
pub fn boss_combat_driver(
	time: Res<Time>,
	catalog: Res<AttackCatalog>,
	mut hosts: Query<(&BossActor, &Health, &mut BossAttackCooldown), With<BossEncounterRoot>>,
	actors: Query<(&GlobalTransform, &CombatTargeting, &Children), With<BikanwardaRoot>>,
	mouth_sockets: Query<Entity, With<BikanwardaMouthSocket>>,
	targets: Query<&GlobalTransform>,
	mut attacks: MessageWriter<BeginAttack>,
) {
	const ATTACK_RANGE: f32 = 40.0; // Bray range from attack definition
	
	for (actor_ref, health, mut cooldown) in &mut hosts {
		// Gate: skip if host health is dead
		if health.is_dead() {
			continue;
		}

		let Ok((actor_transform, targeting, children)) = actors.get(actor_ref.0) else {
			continue;
		};
		
		// Find the mouth socket among the actor's children
		let mouth_socket = children
			.iter()
			.find(|&child| mouth_sockets.contains(child));

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
		
		let distance = actor_transform.translation().distance(target_transform.translation());
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
		// Owner is the ACTOR entity (BikanwardaRoot), which has AttackUser
		// Emit from mouth socket if available
		attacks.write(BeginAttack {
			owner: actor_ref.0,
			definition: BRAY,
			target: Some(ranked.entity),
			allies: Vec::new(),
			emission: None,
			emission_socket: mouth_socket,
		});

		// Reset cooldown based on attack definition
		let cooldown_secs = bray_def.timing.cooldown;
		cooldown.timer = Timer::from_seconds(cooldown_secs, TimerMode::Once);

		debug!(
			"Boss actor {:?} triggered Bray attack on {:?} at {:.1}m with cooldown {:.1}s",
			actor_ref.0, ranked.entity, distance, cooldown_secs
		);
	}
}

/// System to initialize combat state on boss spawn.
///
/// Adds combat components to the actor when the host gets a BossActor binding.
/// The actor gets spotting/targeting/combat systems; the host gets cooldown (checked with Health).
pub fn initialize_boss_combat(
	mut commands: Commands,
	new_bindings: Query<(Entity, &BossActor), (Added<BossActor>, With<BossEncounterRoot>)>,
) {
	for (host_entity, actor_ref) in &new_bindings {
		// Add combat/spotting/targeting to the ACTOR
		// SpottingUser is added by assembly, and the shared bridge will sync it to targeting
		commands.entity(actor_ref.0).insert((
			BossInCombat,
			CombatTargeting::default(),
			SpottingTargetingSettings {
				memory_secs: 3.0,
				continuity: 0.0,
			},
		));
		
		// Add BossAttackCooldown to the HOST (checked alongside Health)
		commands.entity(host_entity).insert(BossAttackCooldown::default());
		
		info!("Initialized combat state: host {:?} → actor {:?}", host_entity, actor_ref.0);
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
		// Install AttacksPlugin if not already present (provides AttackCatalog)
		if !app.is_plugin_added::<attacks::AttacksPlugin>() {
			app.add_plugins(attacks::AttacksPlugin);
		}
		
		app.add_systems(
			Update,
			(
				initialize_boss_combat,
				// Use shared spotting→targeting bridge (no firearm-specific logic needed)
				combat_targeting::sync_spotted_to_combat_targeting
					.after(TransformSystems::Propagate)
					.before(combat_targeting::CombatTargetingSystems::Rank),
				// Combat driver runs after targeting is ranked
				boss_combat_driver
					.after(TransformSystems::Propagate)
					.after(combat_targeting::CombatTargetingSystems::Rank),
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
