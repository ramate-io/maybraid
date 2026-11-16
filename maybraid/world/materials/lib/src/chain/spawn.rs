//! Helper functions for spawning complete anchored chain assemblies.
//!
//! Provides high-level spawn functions that create the full chain: root, joints, mesh,
//! colliders, anchors, health, and material in one call.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::skinning::SkinnedMeshInverseBindposes;
use bevy::prelude::*;
use chains::{
	spawn_chain_colliders, ChainAnchor, ChainColliderConfig, ChainEndpoint, ChainMotion,
	ChainRoot, ChainTensionMode,
};

use super::params::{mythical_chain_ref_for, MythicalChainState};
use super::runtime::OwnedChainMaterial;
use super::skinned::{generate_skinned_chain, ChainSkinningStrategy, SkinnedChainConfig};

/// Configuration for spawning a complete anchored chain.
#[derive(Clone, Debug)]
pub struct AnchoredChainConfig {
	/// Chain geometry and skinning
	pub skinned: SkinnedChainConfig,
	/// Capsule collider dimensions
	pub collider: ChainColliderConfig,
	/// Motion tension mode
	pub tension: ChainTensionMode,
	/// Initial health (durability)
	pub health: f32,
	/// Material state (intact, fade, etc.)
	pub material_state: MythicalChainState,
	/// World-space position offset for the root entity
	pub root_transform: Transform,
}

impl Default for AnchoredChainConfig {
	fn default() -> Self {
		Self {
			skinned: SkinnedChainConfig::default(),
			collider: ChainColliderConfig::default(),
			tension: ChainTensionMode::default(),
			health: 100.0,
			material_state: MythicalChainState::Intact,
			root_transform: Transform::default(),
		}
	}
}

/// Result of spawning an anchored chain assembly.
pub struct AnchoredChainEntities {
	/// The root entity (despawn this to clean up the entire chain).
	pub root: Entity,
	/// The mesh entity (has Mesh3d, SkinnedMesh, material).
	pub mesh: Entity,
	/// Joint entities (one per link, children of root).
	pub joints: Vec<Entity>,
	/// Collider entities (one per link, children of root).
	pub colliders: Vec<Entity>,
	/// Start anchor entity (not parented to chain, should be parented to column/character).
	pub start_anchor: Entity,
	/// End anchor entity (not parented to chain, should be parented to column/character).
	pub end_anchor: Entity,
}

/// Spawn a complete anchored chain with joints, colliders, health, and material.
///
/// This creates:
/// - Chain root with ChainRoot, ChainMotion, Transform
/// - Joint entities (children of root)
/// - Skinned mesh with material (child of root)
/// - Capsule colliders with ChainLinkId (children of root)
/// - Two anchor entities (NOT parented to root - caller must attach to sockets)
/// - Health component on root
/// - OwnedChainMaterial for runtime damage
///
/// The anchor entities are spawned but NOT parented. Caller must parent them to
/// column/character socket entities using `commands.entity(socket).add_child(anchor)`.
#[allow(clippy::too_many_arguments)]
pub fn spawn_anchored_chain(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
	materials: &mut ResMut<Assets<crate::chain::ChainMaterial>>,
	config: AnchoredChainConfig,
	name: &str,
) -> AnchoredChainEntities {
	// Generate skinned chain data
	let chain_data = generate_skinned_chain(config.skinned.clone());

	// Create mesh and bindpose handles
	let mesh_handle = meshes.add(chain_data.mesh);
	let inverse_handle = bindposes.add(SkinnedMeshInverseBindposes::from(chain_data.inverse_bindposes));

	// Create root entity
	let root = commands
		.spawn((
			Name::new(format!("{}-chain-root", name)),
			ChainRoot { link_count: config.skinned.link_count, rest_length_m: config.skinned.length_m },
			ChainMotion { mode: config.tension, active: true },
			config.root_transform,
			Visibility::default(),
		))
		.id();

	// Create joint entities
	let mut joint_entities = Vec::new();
	for (i, pos) in chain_data.rest_joint_positions.iter().enumerate() {
		let joint = commands
			.spawn((
				Name::new(format!("{}-joint-{}", name, i)),
				Transform::from_translation(*pos),
				Visibility::default(),
			))
			.id();
		commands.entity(root).add_child(joint);
		joint_entities.push(joint);
	}

	// Create colliders
	let colliders = spawn_chain_colliders(commands, root, &joint_entities, config.collider);

	// Create mesh entity with skinning and material
	let material_ref = mythical_chain_ref_for(config.material_state);
	let material_handle = materials.add(crate::chain::ChainMaterial::default());

	let mesh_entity = commands
		.spawn((
			Name::new(format!("{}-mesh", name)),
			Mesh3d(mesh_handle),
			Transform::default(),
			Visibility::default(),
			bevy::mesh::skinning::SkinnedMesh {
				inverse_bindposes: inverse_handle,
				joints: joint_entities.clone(),
			},
			MeshMaterial3d(material_handle.clone()),
			// Mark for runtime damage (owned material)
			OwnedChainMaterial,
		))
		.id();

	commands.entity(root).add_child(mesh_entity);

	// Add Health to root (Health component must be added by caller or damage integration)
	// commands.entity(root).insert(Health { current: config.health, max: config.health });

	// Create anchor entities (NOT children of root - caller must parent to sockets)
	let start_anchor = commands
		.spawn((
			Name::new(format!("{}-start-anchor", name)),
			ChainAnchor { endpoint: ChainEndpoint::Start, chain_root: root },
			Transform::default(),
			GlobalTransform::default(),
			Visibility::default(),
		))
		.id();

	let end_anchor = commands
		.spawn((
			Name::new(format!("{}-end-anchor", name)),
			ChainAnchor { endpoint: ChainEndpoint::End, chain_root: root },
			Transform::default(),
			GlobalTransform::default(),
			Visibility::default(),
		))
		.id();

	AnchoredChainEntities {
		root,
		mesh: mesh_entity,
		joints: joint_entities,
		colliders,
		start_anchor,
		end_anchor,
	}
}

/// Shorthand for spawning a simple chain with default config between two world positions.
pub fn spawn_simple_chain(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	bindposes: &mut Assets<SkinnedMeshInverseBindposes>,
	materials: &mut ResMut<Assets<crate::chain::ChainMaterial>>,
	start_pos: Vec3,
	end_pos: Vec3,
	name: &str,
) -> AnchoredChainEntities {
	let config = AnchoredChainConfig::default();
	let entities = spawn_anchored_chain(commands, meshes, bindposes, materials, config, name);

	// Position anchors at the requested world positions
	commands.entity(entities.start_anchor).insert(Transform::from_translation(start_pos));
	commands.entity(entities.end_anchor).insert(Transform::from_translation(end_pos));

	entities
}
