use bevy::prelude::*;

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

/// Bag index stamped on inventory tiles so reorder/focus can follow identity.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InventoryItemKey {
	pub index: usize,
	pub equipped: bool,
}

/// Transient inventory overlay mode. Never persisted with the bag.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum InventoryInteraction {
	#[default]
	Browse,
	Reorder {
		item_index: usize,
	},
}

impl InventoryInteraction {
	pub fn reordering(self) -> Option<usize> {
		match self {
			Self::Reorder { item_index } => Some(item_index),
			Self::Browse => None,
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

/// State for one inventory viewport. Primary editor hosts may mirror their legacy resources.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InventoryMenuSession {
	pub interaction: InventoryInteraction,
	pub focus: InventoryFocusHint,
}
