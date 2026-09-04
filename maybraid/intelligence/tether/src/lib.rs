//! Stay near, or on a ring around, a live entity.
//!
//! [`TetherIntelligenceUser`] is the installed brain. [`TetherMemory`] is the last
//! observation and survives uninstall. This crate does not write [`player::MoveWish`].

mod memory;
mod objective;
mod plugin;
mod user;

pub use memory::TetherMemory;
pub use objective::TetherObjective;
pub use plugin::{write_tether_objectives, TetherPlugin, TetherSystems};
pub use user::{install_tether, Tether, TetherAction, TetherIntelligenceUser};

#[cfg(test)]
mod tests;
