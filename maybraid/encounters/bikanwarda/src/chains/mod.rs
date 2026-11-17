//! Bikanwarda's mythical chain physics and collision.
//!
//! Anchored chain motion, per-link capsule collision, and contact deduplication for the
//! Bikanwarda boss encounter. These are restraint chains that anchor between the boss/environment
//! and respond to player attacks.

mod collision;
mod damage_coalesce;
#[cfg(test)]
mod integration_tests;
mod motion;
mod plugin;

pub use collision::{
	retire_chain_colliders, spawn_chain_colliders, sync_chain_colliders, ChainColliderConfig,
	ChainLinkId,
};
pub use damage_coalesce::coalesce_chain_contacts;
pub use motion::{
	compute_joint_positions, deactivate_released_chains, update_chain_joints, ChainAnchor,
	ChainEndpoint, ChainMotion, ChainRoot, ChainTensionMode, ReleasedChain,
};
pub use plugin::BikanwardaChainPhysicsPlugin;
