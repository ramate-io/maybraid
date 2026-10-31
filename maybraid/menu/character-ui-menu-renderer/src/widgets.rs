use bevy::prelude::*;
use character_ui_menu::{
	InventoryControlAction, InventoryDrawerMode, InventoryRegion, InventorySection,
};

/// Click payload stamped on a pickable leaf.
#[derive(Component, Clone, Copy, Debug)]
pub struct MenuButton<E: Copy + Send + Sync + 'static>(pub E);

/// Header payload; activate opens the overlay keyed by this IR label.
#[derive(Component, Clone, Copy, Debug)]
pub struct OpenSelectKey(pub &'static str);

/// Dismiss control on the overlay backdrop or back button.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct CloseOverlaySelect;

/// Root of the stacked picker screen.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct OverlaySelectRoot;

/// Scroll viewport inside the overlay picker.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct OverlaySelectViewport;

/// Inventory shell command attached to its navigation controls.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryControl(pub InventoryControlAction);

/// Bag index stamped on inventory tiles so reorder/focus can follow identity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryItemKey {
	pub index: usize,
	/// Domain equipped status. `region` separately identifies this representation.
	pub equipped: bool,
	/// UI region containing this representation, independent of domain equip status.
	pub region: InventoryRegion,
}

impl InventoryItemKey {
	/// Rebase bag and Lubikan pseudo-indices after a bag item is removed.
	pub fn remap_after_drop(mut self, dropped_index: usize) -> Option<Self> {
		if self.index == dropped_index {
			None
		} else {
			if self.index > dropped_index {
				self.index -= 1;
			}
			Some(self)
		}
	}
}

/// Transient inventory overlay mode. Never persisted with the bag.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InventoryInteraction {
	#[default]
	Browse,
	Reorder {
		item: InventoryItemKey,
	},
}

impl InventoryInteraction {
	pub fn reordering(self) -> Option<InventoryItemKey> {
		match self {
			Self::Reorder { item } => Some(item),
			Self::Browse => None,
		}
	}

	pub fn remap_after_drop(self, dropped_index: usize) -> Self {
		match self {
			Self::Reorder { item } => item
				.remap_after_drop(dropped_index)
				.map_or(Self::Browse, |item| Self::Reorder { item }),
			Self::Browse => Self::Browse,
		}
	}
}

/// Equipped-row extras so the renderer can emit unequip / adjacent-move events
/// without knowing the host's `MenuEvent` vocabulary.
#[derive(Component, Clone, Copy, Debug)]
pub struct EquippedOrderControls<E: Copy + Send + Sync + 'static> {
	pub unequip: Option<E>,
	pub move_left: Option<E>,
	pub move_right: Option<E>,
}

/// Hint for the next inventory overlay rebuild: follow this bag item.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InventoryFocusHint(pub Option<InventoryItemKey>);

/// Remembered position for a page while the user visits other inventory sections.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InventorySectionMemory {
	pub focused_item: Option<InventoryItemKey>,
	pub gallery_scroll: usize,
}

/// State for one inventory viewport. Primary editor hosts may mirror their legacy resources.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InventoryMenuSession {
	pub interaction: InventoryInteraction,
	pub focus: InventoryFocusHint,
	pub section: InventorySection,
	pub drawer: InventoryDrawerMode,
	pub region: InventoryRegion,
	pub section_memory: [[InventorySectionMemory; 2]; 5],
}

impl InventoryMenuSession {
	pub fn active_memory(&self) -> &InventorySectionMemory {
		&self.section_memory[self.section.index()][self.region.index()]
	}

	pub fn active_memory_mut(&mut self) -> &mut InventorySectionMemory {
		&mut self.section_memory[self.section.index()][self.region.index()]
	}

	/// Rebase every remembered item key after a successful bag removal.
	/// Lubikan pseudo-indices shift too because they start after the bag length.
	pub fn remap_after_drop(&mut self, dropped_index: usize) {
		self.focus.0 = self.focus.0.and_then(|item| item.remap_after_drop(dropped_index));
		for section in &mut self.section_memory {
			for memory in section {
				memory.focused_item =
					memory.focused_item.and_then(|item| item.remap_after_drop(dropped_index));
			}
		}
		self.interaction = self.interaction.remap_after_drop(dropped_index);
	}
}

#[cfg(test)]
mod tests {
	use super::{InventoryInteraction, InventoryItemKey, InventoryMenuSession, InventoryRegion};

	#[test]
	fn drop_remapping_updates_each_representation_and_clears_removed_focus() {
		let mut session = InventoryMenuSession::default();
		let drawer_key =
			InventoryItemKey { index: 8, equipped: true, region: InventoryRegion::Drawer };
		let equipped_key =
			InventoryItemKey { index: 4, equipped: true, region: InventoryRegion::Equipped };
		session.focus.0 = Some(drawer_key);
		session.section_memory[0][0].focused_item = Some(drawer_key);
		session.section_memory[4][1].focused_item = Some(equipped_key);
		session.interaction = InventoryInteraction::Reorder { item: drawer_key };

		session.remap_after_drop(4);
		assert_eq!(session.focus.0, Some(InventoryItemKey { index: 7, ..drawer_key }));
		assert_eq!(session.section_memory[0][0].focused_item, session.focus.0);
		assert_eq!(session.section_memory[4][1].focused_item, None);
		assert_eq!(
			session.interaction,
			InventoryInteraction::Reorder { item: InventoryItemKey { index: 7, ..drawer_key } }
		);
	}

	#[test]
	fn drop_remapping_rebases_lubikan_pseudo_indices() {
		let key = InventoryItemKey { index: 13, equipped: true, region: InventoryRegion::Equipped };
		assert_eq!(key.remap_after_drop(5), Some(InventoryItemKey { index: 12, ..key }));
		assert_eq!(key.remap_after_drop(13), None);
	}
}
