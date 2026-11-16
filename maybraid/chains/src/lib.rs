//! Chain physics and collision for anchored restraint chains.
//!
//! Provides motion simulation, collision detection, and damage routing for chains used in
//! encounters (e.g. Bikanwarda's mythical chains). This is a gameplay/physics crate parallel
//! to `projectiles` and `attacks`.

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
	deactivate_released_chains, update_chain_joints, ChainAnchor, ChainEndpoint, ChainMotion,
	ChainRoot, ChainTensionMode, ReleasedChain,
};
pub use plugin::ChainPhysicsPlugin;
