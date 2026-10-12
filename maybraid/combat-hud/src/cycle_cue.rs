//! D-pad inventory cycling category cue ([#1112](https://github.com/ramate-io/maybraid/issues/1112)).
//!
//! Gameplay sets these resources; the HUD only reads them.

use bevy::prelude::*;

/// Which equipped queue Y will cycle. Upstream inventory code owns updates.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CombatHudInventoryCycleCategory {
	#[default]
	Weapons,
	SkillMaps,
	Lubikans,
}

impl CombatHudInventoryCycleCategory {
	pub const fn is_lubikans(self) -> bool {
		matches!(self, Self::Lubikans)
	}

	pub const fn is_equipped_items(self) -> bool {
		matches!(self, Self::Weapons | Self::SkillMaps)
	}
}

/// One equipped weapon or skill-map slot for the HUD strip.
///
/// `key` is a stable bag index (or other gameplay id). The HUD never writes
/// selection; world input updates this snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CombatHudEquippedSlot {
	pub key: u64,
	pub label: String,
}

/// Read-only equipped-item queue for the cycle strip.
///
/// Empty means the HUD falls back to owned Lubikan accounts. `selected` is an
/// index into [`Self::slots`] after the writer's stable sort.
#[derive(Resource, Clone, Debug, Default, PartialEq, Eq)]
pub struct CombatHudEquippedQueue {
	pub slots: Vec<CombatHudEquippedSlot>,
	pub selected: usize,
}

impl CombatHudEquippedQueue {
	pub fn selected_key(&self) -> Option<u64> {
		self.slots.get(self.selected).map(|slot| slot.key)
	}
}
