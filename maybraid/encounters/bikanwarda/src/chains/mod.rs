//! Bikanwarda's mythical chain physics and collision.
//!
//! Anchored chain motion, per-link capsule collision, and contact deduplication for the
//! Bikanwarda boss encounter. These are restraint chains that anchor between the boss/environment
//! and respond to player attacks.

mod collision;
mod damage;
mod damage_coalesce;
#[cfg(test)]
mod integration_tests;
mod motion;
mod plugin;

pub use collision::{
	retire_chain_colliders, spawn_chain_colliders, sync_chain_colliders, ChainColliderConfig,
	ChainLinkId,
};
pub use damage::{
	detect_and_signal_broken_chains, durability_fraction, update_chain_durability_fraction,
	BrokenChain, ChainBrokenHistory,
};
pub use damage_coalesce::{cleanup_despawned_projectiles, coalesce_chain_contacts, ChainDamageHistory, ChainHitProjectiles};
pub use motion::{
	compute_joint_positions, deactivate_released_chains, update_chain_joints, ChainAnchor,
	ChainEndpoint, ChainMotion, ChainRoot, ChainTensionMode, ReleasedChain,
};
pub use plugin::BikanwardaChainPhysicsPlugin;
