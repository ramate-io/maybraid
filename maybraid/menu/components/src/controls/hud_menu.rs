//! Shared HUD focus list.
//!
//! Pickables stamp [`HudMenuItem`]. The host inserts [`HudMenu`] on the list
//! root after painting. Keyboard and hover update `selected`; activate/focus
//! observers live with the payload type (`MenuActivate` / `MenuFocus`).

use bevy::prelude::*;

use crate::single_select::TextMenuInputLock;

/// Focus index for a HUD list (panel headers or overlay leaves).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudMenu {
	pub selected: usize,
	pub item_count: usize,
}

impl HudMenu {
	pub fn new(item_count: usize) -> Self {
		Self { selected: 0, item_count }
	}

	/// Keep the previous index when a list is rebuilt.
	pub fn retain(item_count: usize, previous: Option<Self>) -> Self {
		let selected = previous
			.filter(|menu| menu.item_count > 0 && item_count > 0)
			.map(|menu| menu.selected.min(item_count - 1))
			.unwrap_or(0);
		Self { selected, item_count }
	}

	pub fn step(&mut self, delta: i32) {
		if self.item_count == 0 {
			return;
		}
		let n = self.item_count as i32;
		self.selected = (self.selected as i32 + delta).rem_euclid(n) as usize;
	}
}

/// One pickable in a [`HudMenu`]. `menu` is the list root, not necessarily
/// the item's parent (tiles nest).
#[derive(Component, Debug, Clone, Copy)]
pub struct HudMenuItem {
	pub index: usize,
	pub menu: Entity,
}

/// Overlay lists steal arrows / Enter from panel lists while they exist.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct HudOverlayMenu;

pub fn select_hud_item_on_over(
	over: On<Pointer<Over>>,
	items: Query<&HudMenuItem>,
	mut menus: Query<&mut HudMenu>,
) {
	let Ok(item) = items.get(over.entity) else {
		return;
	};
	let Ok(mut menu) = menus.get_mut(item.menu) else {
		return;
	};
	if menu.item_count == 0 {
		return;
	}
	menu.selected = item.index.min(menu.item_count - 1);
}

pub fn navigate_hud_menus(
	keyboard: Res<ButtonInput<KeyCode>>,
	lock: Res<TextMenuInputLock>,
	overlay_menus: Query<Entity, With<HudOverlayMenu>>,
	mut menus: Query<(Entity, &mut HudMenu)>,
) {
	if lock.0 {
		return;
	}
	let delta = if keyboard.just_pressed(KeyCode::ArrowDown)
		|| keyboard.just_pressed(KeyCode::ArrowRight)
	{
		1
	} else if keyboard.just_pressed(KeyCode::ArrowUp) || keyboard.just_pressed(KeyCode::ArrowLeft) {
		-1
	} else {
		return;
	};
	let overlay_open = !overlay_menus.is_empty();
	for (entity, mut menu) in &mut menus {
		if overlay_open != overlay_menus.contains(entity) {
			continue;
		}
		menu.step(delta);
	}
}

#[cfg(test)]
mod tests {
	use super::HudMenu;

	#[test]
	fn step_wraps() {
		let mut menu = HudMenu::new(3);
		menu.step(-1);
		assert_eq!(menu.selected, 2);
		menu.step(1);
		assert_eq!(menu.selected, 0);
	}

	#[test]
	fn retain_clamps() {
		let previous = HudMenu { selected: 4, item_count: 5 };
		assert_eq!(HudMenu::retain(2, Some(previous)).selected, 1);
		assert_eq!(HudMenu::retain(0, Some(previous)).selected, 0);
	}
}
