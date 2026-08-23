//! Description copy in the remainder of the screen, to the right of the menu.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::text::{FontSourceTemplate, LineBreak};

use crate::theme::{
	BARLOW_REGULAR, COLUMN_INSET, DESCRIPTION_BOTTOM, DESCRIPTION_FONT_SIZE,
	DESCRIPTION_PANE_LEFT_PERCENT, TEXT_YELLOW_FAINT,
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
				TextMenuDescription
				template_value(Text::new(initial))
				TextFont {
					font: FontSourceTemplate::Handle(BARLOW_REGULAR),
					font_size: px(DESCRIPTION_FONT_SIZE),
				}
				TextColor(TEXT_YELLOW_FAINT)
				TextLayout::new(Justify::Center, LineBreak::WordBoundary)
				Node {
					max_width: percent(80),
				}
				Pickable::IGNORE
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
	if let Ok(mut text) = lines.get_mut(root) {
		text.0 = value.into();
		return;
	}
	let Ok(root_children) = children.get(root) else {
		return;
	};
	let value = value.into();
	for child in root_children {
		if let Ok(mut text) = lines.get_mut(*child) {
			text.0 = value;
			return;
		}
		let Ok(nested) = children.get(*child) else {
			continue;
		};
		for nested_child in nested {
			if let Ok(mut text) = lines.get_mut(*nested_child) {
				text.0 = value;
				return;
			}
		}
	}
}
