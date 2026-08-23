//! Hint strip: animated mark plus faint copy.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::text::{FontSourceTemplate, LineBreak};

use crate::icons::maybraid::AnimatedIcon;
use crate::single_select::text_menu::TextMenu;
use crate::theme::{
	BARLOW_REGULAR, COLUMN_INSET, HINT_BOTTOM, HINT_FONT_SIZE, HINT_ICON_GAP, HINT_ICON_SIZE,
	TEXT_YELLOW, TEXT_YELLOW_FAINT,
};

/// Row that holds the blinking mark and the hint [`Text`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextMenuHint;

/// Marker on the hint copy [`Text`]. Screens write the string.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextMenuHintLabel;

impl TextMenuHint {
	pub fn scene(initial: impl Into<String>) -> impl Scene + 'static {
		let initial = initial.into();
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(AnimatedIcon::maybraid_scene(HINT_ICON_SIZE, TEXT_YELLOW)),
			Box::new(hint_label_scene(initial)),
		];
		bsn! {
			TextMenuHint
			Node {
				position_type: PositionType::Absolute,
				left: px(COLUMN_INSET),
				right: px(COLUMN_INSET),
				bottom: px(HINT_BOTTOM),
				flex_direction: FlexDirection::Row,
				align_items: AlignItems::Center,
				column_gap: px(HINT_ICON_GAP),
			}
			Pickable::IGNORE
			Children [ {children} ]
		}
	}
}

fn hint_label_scene(initial: String) -> impl Scene {
	bsn! {
		TextMenuHintLabel
		template_value(Text::new(initial))
		TextFont {
			font: FontSourceTemplate::Handle(BARLOW_REGULAR),
			font_size: px(HINT_FONT_SIZE),
		}
		TextColor(TEXT_YELLOW_FAINT)
		TextLayout::new(Justify::Left, LineBreak::WordBoundary)
		Pickable::IGNORE
	}
}

/// Write `value` onto the [`TextMenuHintLabel`] that shares a screen with `menu`.
pub fn set_hint_for_menu(
	menu: Entity,
	value: impl Into<String>,
	menus: &Query<&ChildOf, With<TextMenu>>,
	children: &Query<&Children>,
	lines: &mut Query<&mut Text, With<TextMenuHintLabel>>,
) {
	let Ok(child_of) = menus.get(menu) else {
		return;
	};
	let Ok(screen_children) = children.get(child_of.parent()) else {
		return;
	};
	let value = value.into();
	for child in screen_children {
		let Ok(hint_children) = children.get(*child) else {
			continue;
		};
		for hint_child in hint_children {
			if let Ok(mut text) = lines.get_mut(*hint_child) {
				text.0 = value;
				return;
			}
		}
	}
}
