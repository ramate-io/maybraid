//! Combat impact ooze and spiritual corpse dissolution wired through `maybraid-vfx`.

mod impact;
mod spawn_in;
mod spirit;
mod tint;

use bevy::prelude::*;
use character_ragdoll::CharacterRagdollSystems;
use damage::{DamageSystems, tick_queued_despawns};

pub use impact::{CombatOozeTint, DEFAULT_OOZE_TINT};
pub use spawn_in::{SpiritMaterialize, SPIRIT_SPAWN_REVEAL, SPIRIT_SPAWN_SECS};
pub use spirit::{SPIRIT_DISSOLVE_SECS, SpiritDissolveStarted};
pub use tint::{SpiritTint, DEFAULT_SPIRIT_TINT};

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum CombatVfxSystems {
	Impact,
	Settle,
	Despawn,
	Materialize,
}

pub struct CombatVfxPlugin;

impl Plugin for CombatVfxPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			PostUpdate,
			impact::spawn_ooze_on_damage
				.after(DamageSystems::Apply)
				.in_set(CombatVfxSystems::Impact),
		)
		.add_systems(
			Update,
			(spawn_in::begin_spirit_materialize, spawn_in::tick_spirit_materialize)
				.chain()
				.in_set(CombatVfxSystems::Materialize),
		)
		.add_systems(
			Update,
			(
				spirit::tick_corpse_settle,
				spirit::begin_spirit_dissolve,
				spirit::tick_detached_corpse_despawns,
			)
				.chain()
				.after(CharacterRagdollSystems::Simulate)
				.in_set(CombatVfxSystems::Settle),
		)
		.add_systems(
			Last,
			spirit::despawn_corpse_bodies
				.before(tick_queued_despawns)
				.in_set(CombatVfxSystems::Despawn),
		);
		impact::configure_ooze_tints(app);
		tint::configure_spirit_tints(app);
	}
}
