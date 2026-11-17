//! Chain durability and break logic for [#1127](https://github.com/ramate-io/maybraid/issues/1127).
//!
//! Routes applied damage to chain durability and emits exactly one ChainBroken signal
//! when durability reaches zero. Material deterioration is handled separately by a
//! visual system that reads durability fraction.

use std::collections::HashSet;

use bevy::prelude::*;
use damage::DamageApplied;
use encounters::EncounterSignal;

use super::collision::ChainLinkId;
use super::motion::{ChainRoot, ReleasedChain};

/// Tracks which chain roots have already emitted a ChainBroken signal.
///
/// Prevents duplicate signals from multiple lethal hits in the same frame or
/// across frames (if a chain somehow takes damage after breaking but before cleanup).
#[derive(Resource, Default)]
pub struct ChainBrokenHistory {
	pub emitted: HashSet<Entity>,
}

/// Marker for a chain that has broken (durability reached zero).
///
/// Used to prevent further damage application after break.
#[derive(Component, Clone, Copy, Debug)]
pub struct BrokenChain;

/// Apply damage to chain durability.
///
/// Runs in PostUpdate after DamageSystems::Apply. Routes DamageApplied messages
/// that target chain link colliders to their owning ChainRoot's durability.
/// Material deterioration is handled by a separate visual system.
pub fn apply_chain_damage(
	mut damage_applied: MessageReader<DamageApplied>,
	link_ids: Query<&ChainLinkId>,
	mut chains: Query<(Entity, &mut ChainRoot, Option<&BrokenChain>), Without<ReleasedChain>>,
) {
	for event in damage_applied.read() {
		// Check if this damage was applied to a chain link collider
		let Ok(link_id) = link_ids.get(event.target) else {
			continue;
		};

		// Find the owning chain root
		let Ok((chain_entity, mut chain_root, broken)) = chains.get_mut(link_id.chain_root) else {
			continue;
		};

		// Skip if chain is already broken (should not happen due to collider retirement, but belt-and-suspenders)
		if broken.is_some() {
			continue;
		}

		// Apply damage to durability
		let before = chain_root.durability;
		chain_root.durability = (chain_root.durability - event.applied).max(0.0);

		info!(
			"[chain-damage] Chain {:?} took {} damage: durability {} → {} (max {})",
			chain_entity, event.applied, before, chain_root.durability, chain_root.max_durability
		);
	}
}

/// Detect broken chains and emit exactly one ChainBroken signal per chain.
///
/// Runs after apply_chain_damage. Checks for chains that have reached zero durability,
/// marks them as broken, emits the signal, and inserts the ReleasedChain marker.
pub fn detect_and_signal_broken_chains(
	mut commands: Commands,
	chains: Query<(Entity, &ChainRoot), (Without<BrokenChain>, Without<ReleasedChain>)>,
	mut history: ResMut<ChainBrokenHistory>,
	mut signals: MessageWriter<EncounterSignal>,
) {
	for (entity, root) in &chains {
		if root.durability <= 0.0 {
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

			// Emit the signal exactly once
			let signal = EncounterSignal::chain_broken(
				crate::chained_encounter_id(),
				None, // TODO: Track the killing blow source if needed
			);
			signals.write(signal);

			// Record in history
			history.emitted.insert(entity);

			info!(
				"[chain-damage] Chain {:?} broke! Durability reached zero. Signal emitted.",
				entity
			);
		}
	}
}

/// Get durability fraction for material/VFX systems.
///
/// This is a helper for visual systems that need to drive deterioration based on durability.
impl ChainRoot {
	pub fn durability_fraction(&self) -> f32 {
		if self.max_durability > 0.0 {
			(self.durability / self.max_durability).clamp(0.0, 1.0)
		} else {
			0.0
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use damage::{DamagePlugin, Health};

	fn test_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, DamagePlugin))
			.init_resource::<ChainBrokenHistory>()
			.add_message::<EncounterSignal>()
			.add_systems(
				PostUpdate,
				(apply_chain_damage, detect_and_signal_broken_chains).chain(),
			);
		app
	}

	#[test]
	fn chain_durability_decreases_on_damage() -> Result<()> {
		let mut app = test_app();

		let chain_root = app
			.world_mut()
			.spawn((
				ChainRoot {
					link_count: 3,
					rest_length_m: 3.0,
					joint_entities: vec![],
					durability: 100.0,
					max_durability: 100.0,
				},
				Health::from_max(100.0),
			))
			.id();

		let link_collider = app
			.world_mut()
			.spawn(ChainLinkId { link_id: 0, chain_root })
			.id();

		// Simulate damage applied to the link
		app.world_mut().write_message(DamageApplied {
			target: link_collider,
			source: None,
			amount: 25.0,
			applied: 25.0,
			remaining: 75.0,
			point: Vec3::ZERO,
		});

		app.update();

		let durability = app.world().get::<ChainRoot>(chain_root).unwrap().durability;
		assert_eq!(durability, 75.0, "Chain durability should decrease by applied damage");

		Ok(())
	}

	#[test]
	fn zero_durability_emits_one_chain_broken_signal() -> Result<()> {
		let mut app = test_app();

		let chain_root = app
			.world_mut()
			.spawn((
				ChainRoot {
					link_count: 3,
					rest_length_m: 3.0,
					joint_entities: vec![],
					durability: 10.0,
					max_durability: 100.0,
				},
				Health::from_max(100.0),
			))
			.id();

		let link_collider = app
			.world_mut()
			.spawn(ChainLinkId { link_id: 0, chain_root })
			.id();

		// Apply lethal damage
		app.world_mut().write_message(DamageApplied {
			target: link_collider,
			source: None,
			amount: 10.0,
			applied: 10.0,
			remaining: 0.0,
			point: Vec3::ZERO,
		});

		app.update();

		// Check that durability is zero
		let durability = app.world().get::<ChainRoot>(chain_root).unwrap().durability;
		assert_eq!(durability, 0.0);

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
					durability: 20.0,
					max_durability: 100.0,
				},
				Health::from_max(100.0),
			))
			.id();

		let link_collider = app
			.world_mut()
			.spawn(ChainLinkId { link_id: 0, chain_root })
			.id();

		// Apply two lethal hits in the same frame
		app.world_mut().write_message(DamageApplied {
			target: link_collider,
			source: None,
			amount: 15.0,
			applied: 15.0,
			remaining: 0.0,
			point: Vec3::ZERO,
		});
		app.world_mut().write_message(DamageApplied {
			target: link_collider,
			source: None,
			amount: 10.0,
			applied: 10.0,
			remaining: 0.0,
			point: Vec3::ZERO,
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
	fn broken_chain_does_not_take_further_damage() -> Result<()> {
		let mut app = test_app();

		let chain_root = app
			.world_mut()
			.spawn((
				ChainRoot {
					link_count: 3,
					rest_length_m: 3.0,
					joint_entities: vec![],
					durability: 10.0,
					max_durability: 100.0,
				},
				Health::from_max(100.0),
			))
			.id();

		let link_collider = app
			.world_mut()
			.spawn(ChainLinkId { link_id: 0, chain_root })
			.id();

		// First hit breaks the chain
		app.world_mut().write_message(DamageApplied {
			target: link_collider,
			source: None,
			amount: 10.0,
			applied: 10.0,
			remaining: 0.0,
			point: Vec3::ZERO,
		});

		app.update();

		// Verify it's broken
		assert!(app.world().get::<BrokenChain>(chain_root).is_some());
		assert_eq!(app.world().get::<ChainRoot>(chain_root).unwrap().durability, 0.0);

		// Drain the first break signal
		let first_signals: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<EncounterSignal>>()
			.drain()
			.collect();
		assert_eq!(first_signals.len(), 1, "First break should emit exactly one signal");

		// Try to apply more damage
		app.world_mut().write_message(DamageApplied {
			target: link_collider,
			source: None,
			amount: 5.0,
			applied: 5.0,
			remaining: 0.0,
			point: Vec3::ZERO,
		});

		app.update();

		// Durability should still be zero (no further damage)
		assert_eq!(app.world().get::<ChainRoot>(chain_root).unwrap().durability, 0.0);

		// Should be no additional signals
		let signals: Vec<_> = app
			.world_mut()
			.resource_mut::<Messages<EncounterSignal>>()
			.drain()
			.collect();
		assert_eq!(signals.len(), 0, "No additional signals after break");

		Ok(())
	}
}
