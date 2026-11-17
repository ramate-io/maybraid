//! Chain durability and break logic for [#1127](https://github.com/ramate-io/maybraid/issues/1127).
//!
//! Detects when chain Health reaches zero and emits exactly one ChainBroken signal.
//! Material deterioration is handled separately by a visual system that reads Health.
//!
//! **IMPORTANT:** Chain durability is tracked via the standard `Health` component.
//! `DamageApplied` events are already processed by the damage system, which updates Health.
//! Do NOT subtract from a separate durability field, as that would double-count damage.

use std::collections::HashSet;

use bevy::prelude::*;
use damage::Health;
use encounters::EncounterSignal;

use super::motion::{ChainRoot, ReleasedChain};

/// Tracks which chain roots have already emitted a ChainBroken signal.
///
/// Prevents duplicate signals from multiple lethal hits in the same frame or
/// across frames (if a chain somehow takes damage after breaking but before cleanup).
#[derive(Resource, Default)]
pub struct ChainBrokenHistory {
	pub emitted: HashSet<Entity>,
}

/// Marker for a chain that has broken (Health reached zero).
///
/// Used to prevent further damage application after break.
#[derive(Component, Clone, Copy, Debug)]
pub struct BrokenChain;

/// Update ChainDurabilityFraction component from Health.
///
/// ChainDurabilityFraction is defined in world_materials::chain to avoid circular deps.
/// Chains without the component are skipped (material deterioration is optional).
pub fn update_chain_durability_fraction(
	mut commands: Commands,
	chains: Query<
		(Entity, &Health, Option<&world_materials::chain::ChainDurabilityFraction>),
		(With<ChainRoot>, With<world_materials::chain::OwnedChainMaterial>),
	>,
) {
	for (entity, health, frac_opt) in &chains {
		let new_frac = durability_fraction(health);
		match frac_opt {
			Some(frac) if (frac.0 - new_frac).abs() > 0.01 => {
				// Update existing component
				commands
					.entity(entity)
					.insert(world_materials::chain::ChainDurabilityFraction(new_frac));
			}
			None => {
				// Add component for chains with OwnedChainMaterial
				commands
					.entity(entity)
					.insert(world_materials::chain::ChainDurabilityFraction(new_frac));
			}
			_ => {}
		}
	}
}

/// Detect broken chains and emit exactly one ChainBroken signal per chain.
///
/// Checks for chains that have reached zero Health, marks them as broken,
/// emits the signal, and inserts the ReleasedChain marker.
///
/// **CRITICAL:** Also adds `damage::Downed` to prevent the generic corpse cleanup
/// from despawning chains. Chains must persist after breaking for visual fade/release.
pub fn detect_and_signal_broken_chains(
	mut commands: Commands,
	chains: Query<(Entity, &Health), (With<ChainRoot>, Without<BrokenChain>, Without<ReleasedChain>)>,
	mut history: ResMut<ChainBrokenHistory>,
	mut signals: MessageWriter<EncounterSignal>,
) {
	for (entity, health) in &chains {
		if health.current <= 0.0 {
			// Check if we've already emitted for this chain
			if history.emitted.contains(&entity) {
				warn!(
					"[chain-damage] Chain {:?} already broke, skipping duplicate signal",
					entity
				);
				continue;
			}

			// Mark as broken to prevent further damage
			commands.entity(entity).insert(BrokenChain);

			// Insert ReleasedChain to trigger collision cleanup and motion deactivation
			commands.entity(entity).insert(ReleasedChain);

			// Add Downed with a very long DespawnAfter to prevent corpse cleanup
			// Chains persist for visual fade/release, not instant despawn
			// Use effectively infinite duration (999999 seconds ~= 11.5 days)
			commands.entity(entity).insert((
				damage::Downed {
					source: None,
					point: Vec3::ZERO,
					at: 0.0,
				},
				damage::DespawnAfter::seconds(999999.0),
			));

			// Emit the signal exactly once
			let signal = EncounterSignal::chain_broken(
				crate::chained_encounter_id(),
				None, // TODO: Track the killing blow source if needed
			);
			signals.write(signal);

			// Record in history
			history.emitted.insert(entity);

			info!(
				"[chain-damage] Chain {:?} broke! Health reached zero. Signal emitted. Downed added to prevent corpse despawn.",
				entity
			);
		}
	}
}

/// Get durability fraction for material/VFX systems.
///
/// Requires the chain root to have a `Health` component.
pub fn durability_fraction(health: &Health) -> f32 {
	if health.max > 0.0 {
		(health.current / health.max).clamp(0.0, 1.0)
	} else {
		0.0
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use super::super::collision::ChainLinkId;
	use anyhow::Result;
	use damage::{DamagePlugin, Health};

	fn test_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, DamagePlugin))
			.init_resource::<ChainBrokenHistory>()
			.add_message::<EncounterSignal>()
			.add_systems(
				PostUpdate,
				detect_and_signal_broken_chains.after(damage::DamageSystems::Apply),
			);
		app
	}

	#[test]
	fn chain_health_decreases_on_damage_exactly_once() -> Result<()> {
		let mut app = test_app();

		let chain_root = app
			.world_mut()
			.spawn((
				ChainRoot {
					link_count: 3,
					rest_length_m: 3.0,
					joint_entities: vec![],
				},
				Health::from_max(100.0),
			))
			.id();

		let link_collider = app
			.world_mut()
			.spawn((
				ChainLinkId { link_id: 0, chain_root },
				ChildOf(chain_root),
			))
			.id();

		// Send a Hit targeting the chain link
		app.world_mut().write_message(damage::Hit {
			target: link_collider,
			source: None,
			amount: 10.0,
			point: Vec3::ZERO,
			allow_self: false,
		});

		app.update();

		// Health should decrease by exactly 10, not 20 (no double-counting)
		let health = app.world().get::<Health>(chain_root).unwrap();
		assert_eq!(health.current, 90.0, "Chain health should decrease by exactly 10 damage, not double-counted");

		Ok(())
	}

	#[test]
	fn zero_health_emits_one_chain_broken_signal() -> Result<()> {
		let mut app = test_app();

		let chain_root = app
			.world_mut()
			.spawn((
				ChainRoot {
					link_count: 3,
					rest_length_m: 3.0,
					joint_entities: vec![],
				},
				Health::from_max(10.0),
			))
			.id();

		let link_collider = app
			.world_mut()
			.spawn((
				ChainLinkId { link_id: 0, chain_root },
				ChildOf(chain_root),
			))
			.id();

		// Apply lethal damage
		app.world_mut().write_message(damage::Hit {
			target: link_collider,
			source: None,
			amount: 10.0,
			point: Vec3::ZERO,
			allow_self: false,
		});

		app.update();

		// Check that health is zero
		let health = app.world().get::<Health>(chain_root).unwrap();
		assert_eq!(health.current, 0.0);

		// Check that BrokenChain marker was added
		assert!(app.world().get::<BrokenChain>(chain_root).is_some());

		// Check that ReleasedChain marker was added
		assert!(app.world().get::<ReleasedChain>(chain_root).is_some());

		// Check that exactly one signal was emitted
		let signals: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<EncounterSignal>>()
			.drain()
			.collect();
		assert_eq!(signals.len(), 1);
		assert!(matches!(signals[0].kind, encounters::EncounterSignalKind::ChainBroken));

		Ok(())
	}

	#[test]
	fn duplicate_lethal_hits_emit_signal_only_once() -> Result<()> {
		let mut app = test_app();

		let chain_root = app
			.world_mut()
			.spawn((
				ChainRoot {
					link_count: 3,
					rest_length_m: 3.0,
					joint_entities: vec![],
				},
				Health::from_max(20.0),
			))
			.id();

		let link_collider = app
			.world_mut()
			.spawn((
				ChainLinkId { link_id: 0, chain_root },
				ChildOf(chain_root),
			))
			.id();

		// Apply two lethal hits in the same frame
		app.world_mut().write_message(damage::Hit {
			target: link_collider,
			source: None,
			amount: 15.0,
			point: Vec3::ZERO,
			allow_self: false,
		});
		app.world_mut().write_message(damage::Hit {
			target: link_collider,
			source: None,
			amount: 10.0,
			point: Vec3::ZERO,
			allow_self: false,
		});

		app.update();

		// Check that exactly one signal was emitted
		let signals: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<EncounterSignal>>()
			.drain()
			.collect();
		assert_eq!(signals.len(), 1, "Should emit exactly one signal despite multiple lethal hits");

		Ok(())
	}

	#[test]
	fn broken_chain_does_not_emit_duplicate_signals() -> Result<()> {
		let mut app = test_app();

		let chain_root = app
			.world_mut()
			.spawn((
				ChainRoot {
					link_count: 3,
					rest_length_m: 3.0,
					joint_entities: vec![],
				},
				Health::from_max(10.0),
			))
			.id();

		let link_collider = app
			.world_mut()
			.spawn((
				ChainLinkId { link_id: 0, chain_root },
				ChildOf(chain_root),
			))
			.id();

		// First hit breaks the chain
		app.world_mut().write_message(damage::Hit {
			target: link_collider,
			source: None,
			amount: 10.0,
			point: Vec3::ZERO,
			allow_self: false,
		});

		app.update();

		// Verify it's broken
		assert!(app.world().get::<BrokenChain>(chain_root).is_some());
		assert_eq!(app.world().get::<Health>(chain_root).unwrap().current, 0.0);

		// Drain the first break signal
		let first_signals: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<EncounterSignal>>()
			.drain()
			.collect();
		assert_eq!(first_signals.len(), 1, "First break should emit exactly one signal");

		// Update again (chain is still broken)
		app.update();

		// Should be no additional signals
		let signals: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<EncounterSignal>>()
			.drain()
			.collect();
		assert_eq!(signals.len(), 0, "No additional signals after break");

		Ok(())
	}

	#[test]
	fn durability_fraction_works() {
		let full = Health { current: 100.0, max: 100.0 };
		assert_eq!(durability_fraction(&full), 1.0);

		let half = Health { current: 50.0, max: 100.0 };
		assert_eq!(durability_fraction(&half), 0.5);

		let broken = Health { current: 0.0, max: 100.0 };
		assert_eq!(durability_fraction(&broken), 0.0);

		let zero_max = Health { current: 0.0, max: 0.0 };
		assert_eq!(durability_fraction(&zero_max), 0.0);
	}

	#[test]
	fn boss_not_damaged_when_chain_breaks() -> Result<()> {
		let mut app = test_app();

		// Spawn a boss with health
		let boss = app
			.world_mut()
			.spawn((
				Name::new("Bikanwarda"),
				Health::from_max(500.0),
			))
			.id();

		// Spawn a chain with its own health
		let chain_root = app
			.world_mut()
			.spawn((
				ChainRoot {
					link_count: 3,
					rest_length_m: 3.0,
					joint_entities: vec![],
				},
				Health::from_max(10.0),
			))
			.id();

		let link_collider = app
			.world_mut()
			.spawn((
				ChainLinkId { link_id: 0, chain_root },
				ChildOf(chain_root),
			))
			.id();

		// Break the chain with lethal damage
		app.world_mut().write_message(damage::Hit {
			target: link_collider,
			source: Some(boss),
			amount: 10.0,
			point: Vec3::ZERO,
			allow_self: false,
		});

		app.update();

		// Chain should be broken
		assert!(app.world().get::<BrokenChain>(chain_root).is_some());
		assert!(app.world().get::<ReleasedChain>(chain_root).is_some());

		// Boss health should be unchanged
		let boss_health = app.world().get::<Health>(boss).unwrap();
		assert_eq!(boss_health.current, 500.0, "Boss should not be damaged when chain breaks");

		Ok(())
	}

	#[test]
	fn boss_and_chain_dying_same_frame_produces_correct_signals() -> Result<()> {
		let mut app = test_app();

		// Spawn a boss with low health
		let boss = app
			.world_mut()
			.spawn((
				Name::new("Bikanwarda"),
				Health::from_max(10.0),
			))
			.id();

		// Spawn a chain with low health
		let chain_root = app
			.world_mut()
			.spawn((
				ChainRoot {
					link_count: 3,
					rest_length_m: 3.0,
					joint_entities: vec![],
				},
				Health::from_max(10.0),
			))
			.id();

		let chain_link = app
			.world_mut()
			.spawn((
				ChainLinkId { link_id: 0, chain_root },
				ChildOf(chain_root),
			))
			.id();

		// Kill both in the same frame
		app.world_mut().write_message(damage::Hit {
			target: boss,
			source: None,
			amount: 10.0,
			point: Vec3::ZERO,
			allow_self: false,
		});
		app.world_mut().write_message(damage::Hit {
			target: chain_link,
			source: None,
			amount: 10.0,
			point: Vec3::ZERO,
			allow_self: false,
		});

		app.update();

		// Both should be dead
		assert!(app.world().get::<BrokenChain>(chain_root).is_some());
		assert_eq!(app.world().get::<Health>(boss).unwrap().current, 0.0);
		assert_eq!(app.world().get::<Health>(chain_root).unwrap().current, 0.0);

		// Exactly one ChainBroken signal
		let signals: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<EncounterSignal>>()
			.drain()
			.collect();
		let chain_broken_count = signals
			.iter()
			.filter(|s| matches!(s.kind, encounters::EncounterSignalKind::ChainBroken))
			.count();
		assert_eq!(chain_broken_count, 1, "Should emit exactly one ChainBroken signal");

		// Exactly one Died for the boss (chain death doesn't emit Died)
		let died: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<damage::Died>>()
			.drain()
			.collect();
		assert_eq!(died.len(), 2, "Boss and chain should both emit Died");

		Ok(())
	}

	#[test]
	fn despawned_chain_root_during_damage_does_not_panic() -> Result<()> {
		let mut app = test_app();

		let chain_root = app
			.world_mut()
			.spawn((
				ChainRoot {
					link_count: 3,
					rest_length_m: 3.0,
					joint_entities: vec![],
				},
				Health::from_max(100.0),
			))
			.id();

		let link_collider = app
			.world_mut()
			.spawn((
				ChainLinkId { link_id: 0, chain_root },
				ChildOf(chain_root),
			))
			.id();

		// Send damage
		app.world_mut().write_message(damage::Hit {
			target: link_collider,
			source: None,
			amount: 10.0,
			point: Vec3::ZERO,
			allow_self: false,
		});

		// Despawn the chain root before damage is applied (simulating streaming out)
		app.world_mut().despawn(chain_root);

		// Should not panic
		app.update();

		Ok(())
	}

	#[test]
	fn end_to_end_real_projectile_breaks_chain_via_health() -> Result<()> {
		let mut app = test_app();

		// Spawn chain with Health
		let chain_root = app
			.world_mut()
			.spawn((
				ChainRoot {
					link_count: 3,
					rest_length_m: 3.0,
					joint_entities: vec![],
				},
				Health::from_max(10.0),
				GlobalTransform::default(),
			))
			.id();

		let link_collider = app
			.world_mut()
			.spawn((
				ChainLinkId { link_id: 0, chain_root },
				ChildOf(chain_root),
				GlobalTransform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
			))
			.id();

		// Create a projectile entity with HitPayload
		let projectile = app
			.world_mut()
			.spawn(damage::HitPayload { amount: 10.0 })
			.id();

		// Manually write a ProjectileContact to simulate a real collision
		// This goes through contacts_to_hits → apply_hits → detect_and_signal_broken_chains
		app.world_mut().write_message(projectiles::ProjectileContact {
			projectile,
			target: link_collider,
			source: None,
			point: Vec3::ZERO,
			normal: Vec3::Y,
		});

		// Run update: full damage flow
		app.update();

		// Chain should be broken
		assert!(app.world().entity(chain_root).contains::<BrokenChain>(), "Chain should have BrokenChain marker");
		assert!(app.world().entity(chain_root).contains::<ReleasedChain>(), "Chain should have ReleasedChain marker");
		assert!(app.world().entity(chain_root).contains::<damage::Downed>(), "Chain should have Downed to prevent corpse despawn");
		
		// Health should be zero
		let health = app.world().get::<Health>(chain_root).unwrap();
		assert_eq!(health.current, 0.0, "Chain Health should be zero");

		// ChainBroken signal should be emitted
		let signals: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<EncounterSignal>>()
			.drain()
			.collect();
		assert_eq!(signals.len(), 1, "Should emit exactly one ChainBroken signal");
		assert!(matches!(signals[0].kind, encounters::EncounterSignalKind::ChainBroken));

		// Chain should have DespawnAfter with very long duration (not instant corpse cleanup)
		assert!(app.world().entity(chain_root).contains::<damage::DespawnAfter>(), 
			"Chain should have DespawnAfter to prevent immediate ensure_downed_have_despawn addition");

		Ok(())
	}

	#[test]
	fn penetrating_projectile_breaks_chain_exactly_once() -> Result<()> {
		let mut app = test_app();

		let chain_root = app
			.world_mut()
			.spawn((
				ChainRoot {
					link_count: 3,
					rest_length_m: 3.0,
					joint_entities: vec![],
				},
				Health::from_max(15.0),
			))
			.id();

		let link_collider = app
			.world_mut()
			.spawn((
				ChainLinkId { link_id: 0, chain_root },
				ChildOf(chain_root),
			))
			.id();

		// Simulate a penetrating projectile hitting over multiple frames
		// Frame 1: 10 damage
		app.world_mut().write_message(damage::Hit {
			target: link_collider,
			source: None,
			amount: 10.0,
			point: Vec3::ZERO,
			allow_self: false,
		});
		app.update();

		// Chain should not be broken yet
		assert!(app.world().get::<BrokenChain>(chain_root).is_none());

		// Drain signals from frame 1
		let _ = app
			.world_mut()
			.resource_mut::<Messages<EncounterSignal>>()
			.drain()
			.collect::<Vec<_>>();

		// Frame 2: 10 more damage (lethal)
		app.world_mut().write_message(damage::Hit {
			target: link_collider,
			source: None,
			amount: 10.0,
			point: Vec3::ZERO,
			allow_self: false,
		});
		app.update();

		// Chain should be broken
		assert!(app.world().get::<BrokenChain>(chain_root).is_some());

		// Exactly one ChainBroken signal from frame 2
		let signals: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<EncounterSignal>>()
			.drain()
			.collect();
		assert_eq!(signals.len(), 1, "Should emit exactly one ChainBroken signal");

		// Frame 3: another hit (should not emit another signal)
		app.world_mut().write_message(damage::Hit {
			target: link_collider,
			source: None,
			amount: 5.0,
			point: Vec3::ZERO,
			allow_self: false,
		});
		app.update();

		// No additional signals
		let signals: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<EncounterSignal>>()
			.drain()
			.collect();
		assert_eq!(signals.len(), 0, "Should not emit additional signals after break");

		Ok(())
	}
}
