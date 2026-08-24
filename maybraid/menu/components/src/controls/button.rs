//! Transparent text button used by steppers and compact controls.

use bevy::prelude::*;
use bevy::text::Justify;

use crate::theme::{PANEL_VALUE_FONT_SIZE, TEXT_YELLOW};

use super::display::menu_display_name;
use super::text::spawn_hud_text;
use super::HudFonts;

/// Pickable label with no chip background. `extra` is typically `MenuButton<E>`.
pub fn spawn_text_button(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	extra: impl Bundle,
) {
	parent
		.spawn((
			Button,
			extra,
			Node {
				min_width: Val::Px(22.0),
				padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			BackgroundColor(Color::NONE),
		))
		.with_children(|button| {
			spawn_hud_text(
				button,
				fonts.item(PANEL_VALUE_FONT_SIZE),
				&menu_display_name(label),
				TEXT_YELLOW,
				Justify::Center,
			);
		});
}
