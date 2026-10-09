//! Mob presentation helpers. Install goes through [`layer_stack::Present`].
//!
//! The `Last` despawn of hosts and [`mob_intelligence::MemberOf`] members stays here.

mod present;

pub use present::{
	drain_retired_mob_cells, install_mob_cell_teardown, retire_members_with_their_mob,
	MobPresenterState, PresentedMobCell,
};

/// Marker for mob-presenter subscriptions on [`mob_layer_model::Mobs`].
pub struct MobPresent;

#[cfg(test)]
mod tests;
