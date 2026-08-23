//! Bottom-left text column: header, hoverable items, keyboard focus.

use bevy::prelude::*;

use crate::theme::{
	COLUMN_INSET, HEADER_FONT_SIZE, HEADER_MARGIN_BOTTOM, ITEM_FONT_SIZE, ITEM_ROW_GAP,
	TEXT_YELLOW, TEXT_YELLOW_HOVER,
};
use crate::MenuFonts;

/// When `true`, arrow keys and Enter stay with the command line (or other HUD).
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct TextMenuInputLock(pub bool);

/// Root of a vertical text menu. [`selected`] is the keyboard / hover index.
#[derive(Component, Debug, Clone, Copy)]
pub struct TextMenu {
	pub selected: usize,
	pub item_count: usize,
}

impl TextMenu {
	pub fn new(item_count: usize) -> Self {
		Self { selected: 0, item_count }
	}

	pub fn step(&mut self, delta: i32) {
		if self.item_count == 0 {
			return;
		}
		let n = self.item_count as i32;
		self.selected = (self.selected as i32 + delta).rem_euclid(n) as usize;
	}
}

/// Selectable row in a [`TextMenu`].
#[derive(Component, Debug, Clone, Copy)]
pub struct TextMenuItem {
	pub index: usize,
	pub idle: Color,
	pub active: Color,
}

impl TextMenuItem {
	pub fn yellow(index: usize) -> Self {
		Self { index, idle: TEXT_YELLOW, active: TEXT_YELLOW_HOVER }
	}
}

/// Payload fired on click or Enter. `E` is a [`Message`] registered by the screen.
#[derive(Component, Clone, Copy, Debug)]
pub struct TextMenuItemAction<E>(pub E);

/// Marker on the label [`Text`] child of a [`TextMenuItem`].
#[derive(Component, Debug, Clone, Copy)]
pub struct TextMenuItemLabel;

/// Column node pinned to the bottom-left of the window.
pub fn text_menu_column_node() -> Node {
	Node {
		position_type: PositionType::Absolute,
		left: Val::Px(COLUMN_INSET),
		bottom: Val::Px(COLUMN_INSET),
		flex_direction: FlexDirection::Column,
		align_items: AlignItems::FlexStart,
		row_gap: Val::Px(ITEM_ROW_GAP),
		..default()
	}
}

pub fn spawn_text_menu_header(
	parent: &mut ChildSpawnerCommands,
	fonts: &MenuFonts,
	label: impl Into<String>,
) {
	parent.spawn((
		Text::new(label.into()),
		TextFont {
			font: FontSource::from(fonts.header.clone()),
			font_size: FontSize::Px(HEADER_FONT_SIZE),
			..default()
		},
		TextColor(TEXT_YELLOW),
		Node { margin: UiRect::bottom(Val::Px(HEADER_MARGIN_BOTTOM)), ..default() },
		Pickable::IGNORE,
	));
}

pub fn spawn_text_menu_item<E: Copy + Send + Sync + 'static>(
	parent: &mut ChildSpawnerCommands,
	fonts: &MenuFonts,
	index: usize,
	label: impl Into<String>,
	action: E,
) {
	parent
		.spawn((
			Button,
			Node {
				padding: UiRect::axes(Val::Px(0.0), Val::Px(2.0)),
				justify_content: JustifyContent::FlexStart,
				align_items: AlignItems::FlexStart,
				..default()
			},
			BackgroundColor(Color::NONE),
			TextMenuItem::yellow(index),
			TextMenuItemAction(action),
		))
		.with_children(|button| {
			button.spawn((
				Text::new(label.into()),
				TextFont {
					font: FontSource::from(fonts.item.clone()),
					font_size: FontSize::Px(ITEM_FONT_SIZE),
					..default()
				},
				TextColor(TEXT_YELLOW),
				TextMenuItemLabel,
				Pickable::IGNORE,
			));
		});
}

#[allow(clippy::type_complexity)]
pub fn sync_hover_to_text_menu_selection(
	items: Query<(&Interaction, &TextMenuItem, &ChildOf), (Changed<Interaction>, With<Button>)>,
	mut menus: Query<&mut TextMenu>,
) {
	for (interaction, item, child_of) in &items {
		if *interaction != Interaction::Hovered {
			continue;
		}
		if let Ok(mut menu) = menus.get_mut(child_of.parent()) {
			if menu.item_count == 0 {
				continue;
			}
			menu.selected = item.index.min(menu.item_count - 1);
		}
	}
}

pub fn navigate_text_menus(
	keyboard: Res<ButtonInput<KeyCode>>,
	lock: Res<TextMenuInputLock>,
	mut menus: Query<&mut TextMenu>,
) {
	if lock.0 {
		return;
	}
	let delta = if keyboard.just_pressed(KeyCode::ArrowDown) {
		1
	} else if keyboard.just_pressed(KeyCode::ArrowUp) {
		-1
	} else {
		return;
	};
	for mut menu in &mut menus {
		menu.step(delta);
	}
}

pub fn sync_text_menu_item_colors(
	menus: Query<&TextMenu>,
	items: Query<(&TextMenuItem, &ChildOf, &Children)>,
	mut labels: Query<&mut TextColor, With<TextMenuItemLabel>>,
) {
	for (item, child_of, children) in &items {
		let Ok(menu) = menus.get(child_of.parent()) else {
			continue;
		};
		let color = if item.index == menu.selected { item.active } else { item.idle };
		for child in children {
			if let Ok(mut text_color) = labels.get_mut(*child) {
				text_color.0 = color;
			}
		}
	}
}

#[allow(clippy::type_complexity)]
pub fn activate_clicked_text_menu_items<E: Message + Copy>(
	items: Query<(&Interaction, &TextMenuItemAction<E>), (Changed<Interaction>, With<Button>)>,
	mut writer: MessageWriter<E>,
) {
	for (interaction, action) in &items {
		if *interaction == Interaction::Pressed {
			writer.write(action.0);
		}
	}
}

pub fn activate_selected_text_menu_items<E: Message + Copy>(
	keyboard: Res<ButtonInput<KeyCode>>,
	lock: Res<TextMenuInputLock>,
	menus: Query<(Entity, &TextMenu)>,
	items: Query<(&TextMenuItem, &TextMenuItemAction<E>, &ChildOf)>,
	mut writer: MessageWriter<E>,
) {
	if lock.0 || !keyboard.just_pressed(KeyCode::Enter) {
		return;
	}
	for (menu_entity, menu) in &menus {
		for (item, action, child_of) in &items {
			if child_of.parent() == menu_entity && item.index == menu.selected {
				writer.write(action.0);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::TextMenu;

	#[test]
	fn step_wraps() {
		let mut menu = TextMenu::new(5);
		menu.step(-1);
		assert_eq!(menu.selected, 4);
		menu.step(1);
		assert_eq!(menu.selected, 0);
	}
}
