//! Compact rounded HUD card for in-world labels and prompts.

use bevy::prelude::*;

use crate::theme::{
	MENU_CLEAR, OBJECTIVE_MARKER_BORDER, OBJECTIVE_MARKER_PAD_X, OBJECTIVE_MARKER_PAD_Y,
	OBJECTIVE_MARKER_RADIUS, TEXT_YELLOW,
};
use crate::HudFonts;

/// Face size for play HUD cards (smaller than menu [`crate::OBJECTIVE_MARKER_FONT_SIZE`]).
pub const HUD_TEXT_CARD_FACE_PX: f32 = 13.0;

/// Opaque menu-clear card with a yellow stroke. Wraps a label or an icon+label.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudTextCard;

impl HudTextCard {
	pub fn node() -> Node {
		Node {
			padding: UiRect::axes(Val::Px(OBJECTIVE_MARKER_PAD_X), Val::Px(OBJECTIVE_MARKER_PAD_Y)),
			border: UiRect::all(Val::Px(OBJECTIVE_MARKER_BORDER)),
			border_radius: BorderRadius::all(Val::Px(OBJECTIVE_MARKER_RADIUS)),
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			flex_direction: FlexDirection::Row,
			column_gap: Val::Px(4.0),
			flex_shrink: 0.0,
			..default()
		}
	}

	pub fn fill() -> BackgroundColor {
		BackgroundColor(MENU_CLEAR)
	}

	pub fn stroke() -> BorderColor {
		BorderColor::all(TEXT_YELLOW)
	}
}

/// Empty card; caller fills children (text, icon + text, …).
pub fn spawn_hud_text_card(
	parent: &mut ChildSpawnerCommands,
	extra: impl Bundle,
	children: impl FnOnce(&mut ChildSpawnerCommands),
) -> Entity {
	parent
		.spawn((
			HudTextCard,
			HudTextCard::node(),
			HudTextCard::fill(),
			HudTextCard::stroke(),
			Pickable::IGNORE,
			extra,
		))
		.with_children(children)
		.id()
}

/// Single yellow label inside a [`HudTextCard`].
pub fn spawn_hud_text_card_label(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: impl Into<String>,
	extra: impl Bundle,
) -> Entity {
	let label = label.into();
	spawn_hud_text_card(parent, extra, |card| {
		card.spawn((
			Text::new(label),
			fonts.item(HUD_TEXT_CARD_FACE_PX),
			TextColor(TEXT_YELLOW),
			Pickable::IGNORE,
		));
	})
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn card_is_opaque_menu_clear_with_a_yellow_stroke() {
		assert_eq!(HudTextCard::fill().0, MENU_CLEAR);
		assert_eq!(HudTextCard::stroke(), BorderColor::all(TEXT_YELLOW));
		assert_eq!(HUD_TEXT_CARD_FACE_PX, 13.0);
	}
}
