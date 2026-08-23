//! Text menu whose active row shows an animated mark in a reserved gutter.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::text::FontSourceTemplate;

use crate::icons::maybraid::AnimatedIcon;

use super::text_menu::{TextMenu, TextMenuHeader, TextMenuItem, TextMenuItemLabel};
use crate::theme::{
	BARLOW_SEMIBOLD, COLUMN_BOTTOM, COLUMN_INSET, CURSOR_ICON_GAP, CURSOR_ICON_SIZE,
	ITEM_FONT_SIZE, ITEM_ROW_GAP, TEXT_YELLOW,
};

/// Marker on a text-cursor column. Shares [`TextMenu`] selection with the plain column.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextCursorMenu;

/// Reserved gutter on a row; the animated mark is a child.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextCursorSlot;

/// Header plus labeled actions, with an animated mark beside the active row.
pub struct TextCursorColumn<E> {
	pub header: String,
	pub items: Vec<(String, E)>,
}

impl<E: Component + Copy + Default + Unpin + Send + Sync + 'static> TextCursorColumn<E> {
	pub fn new(
		header: impl Into<String>,
		items: impl IntoIterator<Item = (impl Into<String>, E)>,
	) -> Self {
		Self {
			header: header.into(),
			items: items.into_iter().map(|(label, action)| (label.into(), action)).collect(),
		}
	}

	pub fn scene(self) -> impl Scene + 'static {
		let item_count = self.items.len();
		let mut children: Vec<Box<dyn Scene>> = Vec::with_capacity(item_count + 1);
		children.push(Box::new(TextMenuHeader::new(self.header).scene()));
		for (index, (label, action)) in self.items.into_iter().enumerate() {
			children.push(Box::new(cursor_item_scene(TextMenuItem::yellow(index), label, action)));
		}
		bsn! {
			TextCursorMenu
			template_value(TextMenu::new(item_count))
			Node {
				position_type: PositionType::Absolute,
				left: px(COLUMN_INSET),
				bottom: px(COLUMN_BOTTOM),
				flex_direction: FlexDirection::Column,
				align_items: AlignItems::FlexStart,
				row_gap: px(ITEM_ROW_GAP),
			}
			Children [ {children} ]
		}
	}
}

fn cursor_item_scene<E>(item: TextMenuItem, label: String, action: E) -> impl Scene + 'static
where
	E: Component + Copy + Default + Unpin + Send + Sync + 'static,
{
	let visibility = if item.index == 0 { Visibility::Inherited } else { Visibility::Hidden };
	let children: Vec<Box<dyn Scene>> =
		vec![Box::new(cursor_slot_scene(visibility)), Box::new(cursor_label_scene(label))];
	bsn! {
		Button
		template_value(item)
		template_value(action)
		Node {
			padding: UiRect::axes(px(0.0), px(2.0)),
			flex_direction: FlexDirection::Row,
			justify_content: JustifyContent::FlexStart,
			align_items: AlignItems::Center,
			column_gap: px(CURSOR_ICON_GAP),
		}
		BackgroundColor(Color::NONE)
		Children [ {children} ]
	}
}

fn cursor_slot_scene(visibility: Visibility) -> impl Scene {
	let children: Vec<Box<dyn Scene>> = vec![Box::new(
		AnimatedIcon::maybraid_scene_with_visibility(CURSOR_ICON_SIZE, TEXT_YELLOW, visibility),
	)];
	bsn! {
		TextCursorSlot
		Node {
			width: px(CURSOR_ICON_SIZE),
			height: px(CURSOR_ICON_SIZE),
			flex_shrink: 0.0,
		}
		Pickable::IGNORE
		Children [ {children} ]
	}
}

fn cursor_label_scene(label: String) -> impl Scene {
	bsn! {
		template_value(Text::new(label))
		TextFont {
			font: FontSourceTemplate::Handle(BARLOW_SEMIBOLD),
			font_size: px(ITEM_FONT_SIZE),
		}
		TextColor(TEXT_YELLOW)
		TextMenuItemLabel
		Pickable::IGNORE
	}
}

/// Show the animated mark only in the selected row’s gutter.
pub fn sync_text_cursor_icons(
	menus: Query<&TextMenu, With<TextCursorMenu>>,
	items: Query<(Entity, &TextMenuItem, &ChildOf)>,
	children: Query<&Children>,
	slots: Query<(), With<TextCursorSlot>>,
	mut icons: Query<&mut Visibility, With<AnimatedIcon>>,
) {
	for (item_entity, item, child_of) in &items {
		let Ok(menu) = menus.get(child_of.parent()) else {
			continue;
		};
		let show = item.index == menu.selected;
		let Ok(item_children) = children.get(item_entity) else {
			continue;
		};
		for child in item_children {
			if slots.get(*child).is_err() {
				continue;
			}
			let Ok(slot_children) = children.get(*child) else {
				continue;
			};
			for icon_entity in slot_children {
				if let Ok(mut visibility) = icons.get_mut(*icon_entity) {
					*visibility = if show { Visibility::Inherited } else { Visibility::Hidden };
				}
			}
		}
	}
}
