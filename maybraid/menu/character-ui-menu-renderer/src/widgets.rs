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

/// Navigable category header in the editable inventory's equipped list.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryFolderHeader(pub InventorySection);

/// Stable selection in the left list; menu indices also include category headers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InventoryLeftFocus {
	Header(InventorySection),
	Slot { section: InventorySection, ordinal: usize },
}

impl Default for InventoryLeftFocus {
	fn default() -> Self {
		Self::Header(InventorySection::Clothing)
	}
}

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
	/// Last selected row when the region contains empty equipped slots.
	pub focused_ordinal: usize,
	pub gallery_scroll: usize,
}

/// AIOP row that opened the selection drawer. `item` is absent for an empty slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryPickerSource {
	pub ordinal: usize,
	pub item: Option<InventoryItemKey>,
	/// Candidate whose host action is awaiting an accepted model/UI refresh.
	pub attempted_item: Option<usize>,
}

/// Host conversion from a catalog action to assignment at the selected AIOP slot.
/// Kept on the viewport so independent hosts retain their own event vocabulary.
#[derive(Component, Clone, Copy)]
pub struct InventorySlotEvent<E: Copy + Send + Sync + 'static>(
	pub fn(E, InventorySection, usize) -> Option<E>,
);

/// State for one inventory viewport. Primary editor hosts may mirror their legacy resources.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryMenuSession {
	pub interaction: InventoryInteraction,
	pub focus: InventoryFocusHint,
	pub section: InventorySection,
	/// Editable inventory uses folders; direct read-only catalogs retain page navigation.
	pub folder_navigation: bool,
	pub expanded_section: Option<InventorySection>,
	pub left_focus: InventoryLeftFocus,
	pub drawer: InventoryDrawerMode,
	pub region: InventoryRegion,
	pub picker_source: Option<InventoryPickerSource>,
	pub section_memory: [[InventorySectionMemory; 2]; 5],
}

impl Default for InventoryMenuSession {
	fn default() -> Self {
		Self {
			interaction: InventoryInteraction::default(),
			focus: InventoryFocusHint::default(),
			section: InventorySection::default(),
			folder_navigation: true,
			expanded_section: Some(InventorySection::Clothing),
			left_focus: InventoryLeftFocus::default(),
			drawer: InventoryDrawerMode::default(),
			region: InventoryRegion::Equipped,
			picker_source: None,
			section_memory: [[InventorySectionMemory::default(); 2]; 5],
		}
	}
}

impl InventoryMenuSession {
	/// Header focus never overwrites the remembered slot within its category.
	pub fn focus_header(&mut self, section: InventorySection) {
		self.left_focus = InventoryLeftFocus::Header(section);
		self.focus.0 = None;
	}

	pub fn focus_slot(
		&mut self,
		section: InventorySection,
		ordinal: usize,
		item: Option<InventoryItemKey>,
	) {
		self.section = section;
		self.left_focus = InventoryLeftFocus::Slot { section, ordinal };
		let memory = &mut self.section_memory[section.index()][InventoryRegion::Equipped.index()];
		memory.focused_ordinal = ordinal;
		memory.focused_item = item;
		self.focus.0 = item;
	}

	pub fn toggle_folder(&mut self, section: InventorySection) {
		if self.picker_source.is_some() || self.interaction.reordering().is_some() {
			return;
		}
		self.region = InventoryRegion::Equipped;
		self.section = section;
		self.focus_header(section);
		self.expanded_section =
			if self.expanded_section == Some(section) { None } else { Some(section) };
	}

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
		if let Some(source) = &mut self.picker_source {
			source.item = source.item.and_then(|item| item.remap_after_drop(dropped_index));
			source.attempted_item = source.attempted_item.and_then(|index| {
				(index != dropped_index).then_some(if index > dropped_index {
					index - 1
				} else {
					index
				})
			});
		}
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
