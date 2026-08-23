//! Bottom-left text column, authored as BSN [`Scene`]s.
//!
//! Pickable rows stamp a screen-specific choice component `E`. Selection
//! triggers [`MenuFocus<E>`] on the [`TextMenu`]; click / Enter trigger
//! [`MenuActivate<E>`]. Both bubble to the screen. [`republish_menu_activate`]
//! copies activate onto [`Message<E>`] for listeners outside the screen.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::text::FontSourceTemplate;

use crate::theme::{
	BARLOW_BLACK, BARLOW_SEMIBOLD, COLUMN_BOTTOM, COLUMN_INSET, HEADER_FONT_SIZE,
	HEADER_MARGIN_BOTTOM, ITEM_FONT_SIZE, ITEM_ROW_GAP, TEXT_YELLOW, TEXT_YELLOW_HOVER,
};

/// When `true`, arrow keys, Enter, and pick activations stay with the command line.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct TextMenuInputLock(pub bool);

/// Root of a vertical text menu. [`selected`] is the keyboard / hover index.
#[derive(Component, Debug, Default, Clone, Copy)]
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

/// Chrome on a selectable row. The screen’s choice type is a separate component.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextMenuItem {
	pub index: usize,
	pub idle: Color,
	pub active: Color,
}

impl TextMenuItem {
	pub fn yellow(index: usize) -> Self {
		Self { index, idle: TEXT_YELLOW, active: TEXT_YELLOW_HOVER }
	}

	/// Pickable row that stamps `action` for [`MenuFocus`] / [`MenuActivate`].
	pub fn scene<E>(self, label: impl Into<String>, action: E) -> impl Scene + 'static
	where
		E: Component + Copy + Default + Unpin + Send + Sync + 'static,
	{
		let label = label.into();
		bsn! {
			Button
			template_value(self)
			template_value(action)
			Node {
				padding: UiRect::axes(px(0.0), px(2.0)),
				justify_content: JustifyContent::FlexStart,
				align_items: AlignItems::FlexStart,
			}
			BackgroundColor(Color::NONE)
			Children [(
				template_value(Text::new(label))
				TextFont {
					font: FontSourceTemplate::Handle(BARLOW_SEMIBOLD),
					font_size: px(ITEM_FONT_SIZE),
				}
				TextColor(TEXT_YELLOW)
				TextMenuItemLabel
				Pickable::IGNORE
			)]
		}
	}
}

/// Marker on the label [`Text`] child of a [`TextMenuItem`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextMenuItemLabel;

/// Non-interactive title above the options.
pub struct TextMenuHeader {
	pub label: String,
}

impl TextMenuHeader {
	pub fn new(label: impl Into<String>) -> Self {
		Self { label: label.into() }
	}

	pub fn scene(self) -> impl Scene + 'static {
		let label = self.label;
		bsn! {
			template_value(Text::new(label))
			TextFont {
				font: FontSourceTemplate::Handle(BARLOW_BLACK),
				font_size: px(HEADER_FONT_SIZE),
			}
			TextColor(TEXT_YELLOW)
			Node {
				margin: UiRect::bottom(px(HEADER_MARGIN_BOTTOM)),
			}
			Pickable::IGNORE
		}
	}
}

/// Header plus labeled actions, pinned to the bottom-left.
pub struct TextMenuColumn<E> {
	pub header: String,
	pub items: Vec<(String, E)>,
}

impl<E: Component + Copy + Default + Unpin + Send + Sync + 'static> TextMenuColumn<E> {
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
			children.push(Box::new(TextMenuItem::yellow(index).scene(label, action)));
		}
		bsn! {
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

pub fn select_text_menu_item_on_over(
	over: On<Pointer<Over>>,
	items: Query<(&TextMenuItem, &ChildOf)>,
	mut menus: Query<&mut TextMenu>,
) {
	let Ok((item, child_of)) = items.get(over.entity) else {
		return;
	};
	let Ok(mut menu) = menus.get_mut(child_of.parent()) else {
		return;
	};
	if menu.item_count == 0 {
		return;
	}
	menu.selected = item.index.min(menu.item_count - 1);
}

/// In-screen focus: triggered on the [`TextMenu`], bubbles to the screen root.
#[derive(EntityEvent, Clone, Copy, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct MenuFocus<E> {
	pub entity: Entity,
	pub choice: E,
}

/// In-screen activate: triggered on the [`TextMenu`], bubbles to the screen root.
#[derive(EntityEvent, Clone, Copy, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct MenuActivate<E> {
	pub entity: Entity,
	pub choice: E,
}

fn selected_choice<E: Component + Copy>(
	menu_entity: Entity,
	menu: &TextMenu,
	items: &Query<(&TextMenuItem, &E, &ChildOf)>,
) -> Option<E> {
	items.iter().find_map(|(item, choice, child_of)| {
		(child_of.parent() == menu_entity && item.index == menu.selected).then_some(*choice)
	})
}

/// Trigger [`MenuFocus<E>`] on a menu when its [`TextMenu::selected`] changes.
pub fn emit_menu_focus<E: Component + Copy + Send + Sync + 'static>(
	menus: Query<(Entity, &TextMenu), Changed<TextMenu>>,
	items: Query<(&TextMenuItem, &E, &ChildOf)>,
	mut commands: Commands,
) {
	for (menu_entity, menu) in &menus {
		if let Some(choice) = selected_choice(menu_entity, menu, &items) {
			commands.trigger(MenuFocus { entity: menu_entity, choice });
		}
	}
}

/// Trigger [`MenuActivate<E>`] on the row’s parent menu.
pub fn emit_menu_activate_on_click<E: Component + Copy>(
	click: On<Pointer<Click>>,
	lock: Res<TextMenuInputLock>,
	items: Query<(&E, &ChildOf), With<TextMenuItem>>,
	mut commands: Commands,
) {
	if lock.0 {
		return;
	}
	let Ok((choice, child_of)) = items.get(click.entity) else {
		return;
	};
	commands.trigger(MenuActivate { entity: child_of.parent(), choice: *choice });
}

/// Trigger [`MenuActivate<E>`] for the keyboard-selected row.
pub fn emit_menu_activate_on_enter<E: Component + Copy + Send + Sync + 'static>(
	keyboard: Res<ButtonInput<KeyCode>>,
	lock: Res<TextMenuInputLock>,
	menus: Query<(Entity, &TextMenu)>,
	items: Query<(&TextMenuItem, &E, &ChildOf)>,
	mut commands: Commands,
) {
	if lock.0 || !keyboard.just_pressed(KeyCode::Enter) {
		return;
	}
	for (menu_entity, menu) in &menus {
		if let Some(choice) = selected_choice(menu_entity, menu, &items) {
			commands.trigger(MenuActivate { entity: menu_entity, choice });
		}
	}
}

/// Screen-boundary adapter: [`MenuActivate<E>`] → [`Message<E>`].
pub fn republish_menu_activate<E: Message + Copy>(
	activate: On<MenuActivate<E>>,
	mut writer: MessageWriter<E>,
) {
	writer.write(activate.event().choice);
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
