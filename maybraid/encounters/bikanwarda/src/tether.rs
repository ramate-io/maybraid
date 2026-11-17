//! Chain tether setup for the Bikanwarda encounter.
//!
//! Spawns and manages the chain connecting the pillar to the boss's rear leg.

use bevy::prelude::*;
use bikanwarda_character::BikanwardaChainAnchor;
use damage::Health;

use crate::chains::{
	spawn_chain_colliders, ChainAnchor, ChainColliderConfig, ChainEndpoint, ChainMotion, ChainRoot,
	ChainTensionMode,
};
use crate::presentation::{BossColumn, BossEncounterRoot};

/// Marker for the encounter's chain tether.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct BossTetherChain;

use crate::presentation::BossActor;

/// System to spawn the chain tether when boss and pillar are present.
///
/// Queries the host (BossEncounterRoot) to get the actor via BossActor, then finds
/// the column (BossColumn) child and rear leg anchor on the actor. Spawns exactly
/// ONE tether per host, gated by Without<BossTetherChain> on the HOST.
pub fn spawn_tether_chain(
	mut commands: Commands,
	hosts: Query<
		(Entity, &BossActor, &Children),
		(
			With<BossEncounterRoot>,
			Without<BossTetherChain>,
		),
	>,
	columns: Query<Entity, With<BossColumn>>,
	boss_anchors: Query<(Entity, &BikanwardaChainAnchor)>,
) {
	for (host_entity, actor_ref, children) in &hosts {
		// Find the column (pillar) entity from host's children
		let column_entity = children
			.iter()
			.find(|&child| columns.contains(child));
		
		let Some(column_entity) = column_entity else {
			warn!("Could not find column child for host {:?}", host_entity);
			continue;
		};

		// Find the rear leg anchor (neg_z) on the ACTOR (actor_ref.0)
		// The anchor is a descendant of the actor entity
		let rear_anchor_entity = boss_anchors
			.iter()
			.find_map(|(entity, anchor)| {
				if anchor.name == "chain_anchor_neg_z" {
					Some(entity)
				} else {
					None
				}
			});

		let Some(rear_anchor_entity) = rear_anchor_entity else {
			warn!("Could not find rear leg anchor for actor {:?}", actor_ref.0);
			continue;
		};

		// Create chain with 10 links, 8m rest length (pillar is 8m from boss center)
		let link_count = 10;
		let rest_length_m = 8.0;

		// Create joint entities
		let mut joint_entities = Vec::new();
		for i in 0..link_count {
			let joint = commands
				.spawn((
					Name::new(format!("tether-chain-joint-{}", i)),
					Transform::default(),
					GlobalTransform::default(),
				))
				.id();
			joint_entities.push(joint);
		}

		// Spawn chain root
		let chain_root = commands
			.spawn((
				Name::new("boss-tether-chain"),
				BossTetherChain,
				ChainRoot {
					link_count,
					rest_length_m,
					joint_entities: joint_entities.clone(),
				},
				ChainMotion {
					mode: ChainTensionMode::Catenary { sag_ratio: 15 },
					active: true,
				},
				Health::from_max(500.0), // Chain durability
				Transform::default(),
				GlobalTransform::default(),
			))
			.id();

		// Spawn chain anchors
		commands.entity(column_entity).insert(ChainAnchor {
			endpoint: ChainEndpoint::Start,
			chain_root,
		});

		commands.entity(rear_anchor_entity).insert(ChainAnchor {
			endpoint: ChainEndpoint::End,
			chain_root,
		});

		// Spawn colliders for the chain
		let config = ChainColliderConfig {
			radius: 0.15,
			half_length: 0.4,
		};

		spawn_chain_colliders(&mut commands, chain_root, &joint_entities, config);

		// Mark the HOST with BossTetherChain to prevent repeat spawns
		commands.entity(host_entity).insert(BossTetherChain);

		info!(
			"Spawned tether chain connecting pillar {:?} to boss rear anchor {:?} for host {:?}",
			column_entity, rear_anchor_entity, host_entity
		);
	}
}

/// Plugin for boss tether chain management.
pub struct BossTetherPlugin;

impl Plugin for BossTetherPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(Update, spawn_tether_chain);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tether_chain_spawns_with_health() {
		// Placeholder test - actual test would need full boss assembly
		let health = Health::from_max(500.0);
		assert_eq!(health.max, 500.0);
		assert_eq!(health.current, 500.0);
	}
}
