//! Per-link capsule collision for chains ([#1126](https://github.com/ramate-io/maybraid/issues/1126)).
//!
//! Provides stable collision identity for damage routing. Each link gets one capsule collider
//! with a `ChainLinkId` component. Hits on these colliders remount to the chain root's Health
//! via the existing `Health::entity_or_parent` mechanism.

use avian3d::prelude::*;
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;

use super::motion::ChainRoot;

/// Stable per-link collision identity.
///
/// This component lives on each capsule collider entity. The link_id indexes into
/// the chain material's `link_damage[32]` array.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChainLinkId {
	/// Link index (0-based). Matches vertex color R in the skinned mesh.
	pub link_id: u8,
	/// The chain root entity this link belongs to.
	pub chain_root: Entity,
}

/// Configuration for spawning chain capsule colliders.
#[derive(Clone, Copy, Debug)]
pub struct ChainColliderConfig {
	/// Capsule radius (should match or slightly exceed the visual tube radius).
	pub radius: f32,
	/// Capsule half-length along the link.
	pub half_length: f32,
}

impl Default for ChainColliderConfig {
	fn default() -> Self {
		// Defaults that roughly match the visual chain from skinned.rs
		Self { radius: 0.052, half_length: 0.08 }
	}
}

/// Spawn capsule colliders for a chain's joints.
///
/// Each link gets one capsule collider as a child of the chain root (not parented to individual
/// joints, to simplify cleanup). The capsules follow joint positions via a sync system.
///
/// Returns the spawned collider entities.
pub fn spawn_chain_colliders(
	commands: &mut Commands,
	chain_root: Entity,
	joint_entities: &[Entity],
	config: ChainColliderConfig,
) -> Vec<Entity> {
	let mut collider_entities = Vec::new();

	for (link_id, &_joint) in joint_entities.iter().enumerate() {
		let collider = Collider::capsule(config.radius, config.half_length * 2.0);

		let entity = commands
			.spawn((
				Name::new(format!("chain-link-{link_id}")),
				Transform::IDENTITY,
				Visibility::default(),
				RigidBody::Static,
				collider,
				// Query-only collision (no physics forces)
				Sensor,
				// Fixed interaction layer (attacks and projectiles hit this)
				PhysicsInteractionLayer::fixed_layers(),
				// Stable link identity for damage routing
				ChainLinkId { link_id: link_id as u8, chain_root },
				// Parent to the root, not individual joints, for simpler cleanup
				ChildOf(chain_root),
			))
			.id();

		collider_entities.push(entity);
	}

	collider_entities
}

/// Sync capsule collider positions to their corresponding joint transforms.
///
/// This runs after joint transforms have been updated by the motion system.
pub fn sync_chain_colliders(
	chains: Query<(&ChainRoot, &Children)>,
	joints: Query<(&Transform, Option<&Name>), Without<ChainLinkId>>,
	mut colliders: Query<(&ChainLinkId, &mut Transform), Without<ChildOf>>,
) {
	for (_root, children) in &chains {
		// Collect joints (children with Transform but no ChainLinkId)
		let mut joint_transforms: Vec<(Entity, Transform)> = children
			.iter()
			.filter_map(|&entity| {
				joints.get(entity).ok().and_then(|(transform, name)| {
					// Exclude non-joint children
					if name.map_or(false, |n| n.as_str().contains("joint")) {
						Some((entity, *transform))
					} else {
						None
					}
				})
			})
			.collect();

		// Sort by entity bits for deterministic ordering
		joint_transforms.sort_by_key(|(e, _)| e.index());

		// Update collider positions to match joints
		for (link_id, (_, joint_transform)) in joint_transforms.iter().enumerate() {
			// Find the collider for this link
			for (collider_id, mut collider_transform) in &mut colliders {
				if collider_id.link_id == link_id as u8 {
					collider_transform.translation = joint_transform.translation;
					collider_transform.rotation = joint_transform.rotation;
				}
			}
		}
	}
}

/// Retire colliders for a released/broken chain.
///
/// This despawns all collider entities associated with a chain root.
pub fn retire_chain_colliders(commands: &mut Commands, colliders: Vec<Entity>) {
	for entity in colliders {
		commands.entity(entity).try_despawn();
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn chain_link_id_stores_link_and_root() {
		let root = Entity::from_bits(123);
		let id = ChainLinkId { link_id: 5, chain_root: root };

		assert_eq!(id.link_id, 5);
		assert_eq!(id.chain_root, root);
	}

	#[test]
	fn spawn_chain_colliders_creates_one_per_joint() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);

		let root = app.world_mut().spawn((ChainRoot { link_count: 3, rest_length_m: 2.0 },)).id();
		let joints = vec![
			app.world_mut().spawn(Transform::default()).id(),
			app.world_mut().spawn(Transform::default()).id(),
			app.world_mut().spawn(Transform::default()).id(),
		];

		let config = ChainColliderConfig::default();
		let colliders = spawn_chain_colliders(app.world_mut(), root, &joints, config);

		assert_eq!(colliders.len(), 3);

		// Verify each collider has correct link id
		for (i, entity) in colliders.iter().enumerate() {
			let link_id = app.world().get::<ChainLinkId>(*entity).unwrap();
			assert_eq!(link_id.link_id, i as u8);
			assert_eq!(link_id.chain_root, root);
		}
	}

	#[test]
	fn collider_config_defaults_match_visual_scale() {
		let config = ChainColliderConfig::default();
		// Should be close to the tube_radius from skinned.rs (0.052)
		assert!((config.radius - 0.052).abs() < 1e-5);
	}
}
