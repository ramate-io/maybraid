//! Scope component and exclusive focus pick.

use bevy::prelude::*;
use menu_components::{HudMenu, HudOverlayMenu, TextMenu};

/// Input scope. Nav from [`crate::dispatch::dispatch_menu_nav`] is delivered to
/// one focused descendant, not every child.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MenuController {
	pub focus: Option<Entity>,
}

impl MenuController {
	/// Overlay, then any HUD list, then a text column.
	pub fn pick_focus(
		overlay: Option<Entity>,
		hud: Option<Entity>,
		text: Option<Entity>,
	) -> Option<Entity> {
		overlay.or(hud).or(text)
	}

	pub fn resolve(
		root: Entity,
		children: &Query<&Children>,
		overlays: &Query<Entity, With<HudOverlayMenu>>,
		hud_menus: &Query<Entity, (With<HudMenu>, Without<HudOverlayMenu>)>,
		text_menus: &Query<Entity, With<TextMenu>>,
	) -> Option<Entity> {
		let mut overlay = None;
		let mut hud = None;
		let mut text = None;
		for entity in std::iter::once(root).chain(children.iter_descendants(root)) {
			if overlays.contains(entity) {
				overlay = Some(entity);
			} else if hud_menus.contains(entity) && hud.is_none() {
				hud = Some(entity);
			} else if text_menus.contains(entity) && text.is_none() {
				text = Some(entity);
			}
		}
		Self::pick_focus(overlay, hud, text)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn overlay_beats_hud_and_text() -> anyhow::Result<()> {
		let overlay = Entity::from_bits(1);
		let hud = Entity::from_bits(2);
		let text = Entity::from_bits(3);
		assert_eq!(MenuController::pick_focus(Some(overlay), Some(hud), Some(text)), Some(overlay));
		assert_eq!(MenuController::pick_focus(None, Some(hud), Some(text)), Some(hud));
		assert_eq!(MenuController::pick_focus(None, None, Some(text)), Some(text));
		assert_eq!(MenuController::pick_focus(None, None, None), None);
		Ok(())
	}
}
