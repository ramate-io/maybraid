//! Description copy in the remainder of the screen, to the right of the menu.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::text::{FontSourceTemplate, LineBreak};

use crate::theme::{
	BARLOW_REGULAR, COLUMN_INSET, DESCRIPTION_BAND_HEIGHT, DESCRIPTION_BOTTOM,
	DESCRIPTION_FONT_SIZE, DESCRIPTION_PANE_LEFT_PERCENT, TEXT_YELLOW_FAINT,
};

/// Marker on the description [`Text`]. Screens write the string.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextMenuDescription;

impl TextMenuDescription {
	pub fn scene(initial: impl Into<String>) -> impl Scene + 'static {
		let initial = initial.into();
		bsn! {
			Node {
				position_type: PositionType::Absolute,
				left: percent(DESCRIPTION_PANE_LEFT_PERCENT),
				right: px(COLUMN_INSET),
				top: px(COLUMN_INSET),
				bottom: px(DESCRIPTION_BOTTOM),
				flex_direction: FlexDirection::Column,
				align_items: AlignItems::Center,
				justify_content: JustifyContent::FlexEnd,
			}
			Pickable::IGNORE
			Children [(
				Node {
					width: percent(80),
					height: px(DESCRIPTION_BAND_HEIGHT),
					justify_content: JustifyContent::Center,
					align_items: AlignItems::Center,
				}
				Pickable::IGNORE
				Children [(
					TextMenuDescription
					template_value(Text::new(initial))
					TextFont {
						font: FontSourceTemplate::Handle(BARLOW_REGULAR),
						font_size: px(DESCRIPTION_FONT_SIZE),
					}
					TextColor(TEXT_YELLOW_FAINT)
					TextLayout::new(Justify::Center, LineBreak::WordBoundary)
					Pickable::IGNORE
				)]
			)]
		}
	}
}

/// Write `value` onto the [`TextMenuDescription`] under `root` (the screen).
///
/// [`crate::single_select::MenuFocus`] bubbles, so `entity` on a screen observer
/// is the screen, not the menu.
pub fn set_description_for_menu(
	root: Entity,
	value: impl Into<String>,
	children: &Query<&Children>,
	lines: &mut Query<&mut Text, With<TextMenuDescription>>,
) {
	let value = value.into();
	set_description_under(root, &value, children, lines);
}

fn set_description_under(
	entity: Entity,
	value: &str,
	children: &Query<&Children>,
	lines: &mut Query<&mut Text, With<TextMenuDescription>>,
) -> bool {
	if let Ok(mut text) = lines.get_mut(entity) {
		text.0 = value.to_string();
		return true;
	}
	let Ok(kids) = children.get(entity) else {
		return false;
	};
	for child in kids {
		if set_description_under(*child, value, children, lines) {
			return true;
		}
	}
	false
}
