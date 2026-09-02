//! Text menu whose active row shows an animated mark in a reserved gutter.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::text::{FontSourceTemplate, Justify};

use crate::icons::maybraid::AnimatedIcon;

use super::text_menu::{
	TextColumnAlign, TextColumnAnchor, TextMenu, TextMenuHeader, TextMenuItem, TextMenuItemLabel,
};
use crate::controls::section::CursorRow;
use crate::info::description::TextMenuDescription;
use crate::theme::{
	BARLOW_SEMIBOLD, CORNER_BOTTOM, CORNER_INSET, CURSOR_ICON_GAP, CURSOR_ICON_SIZE,
	DESCRIPTION_FONT_SIZE, ITEM_FONT_SIZE, TEXT_YELLOW, TEXT_YELLOW_FAINT,
};
use maybraid_input::{MenuNav, MenuNavPad};

/// Marker on a text-cursor column. Shares [`TextMenu`] selection with the plain column.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextCursorMenu;

/// Reserved gutter on a row; the animated mark is a child.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextCursorSlot;

/// One labeled action, optionally with a caption under the title.
pub struct TextCursorRow<E> {
	pub label: String,
	pub subtext: Option<String>,
	pub action: E,
}

impl<E> TextCursorRow<E> {
	pub fn new(label: impl Into<String>, action: E) -> Self {
		Self { label: label.into(), subtext: None, action }
	}

	pub fn with_subtext(mut self, subtext: impl Into<String>) -> Self {
		let subtext = subtext.into();
		self.subtext = (!subtext.is_empty()).then_some(subtext);
		self
	}
}

/// Header plus labeled actions, with an animated mark beside the active row.
pub struct TextCursorColumn<E> {
	pub header: Option<String>,
	pub items: Vec<TextCursorRow<E>>,
	pub anchor: TextColumnAnchor,
	pub align: TextColumnAlign,
	pub selected: usize,
	pub description: Option<String>,
}

impl<E: Component + Copy + Default + Unpin + Send + Sync + 'static> TextCursorColumn<E> {
	pub fn new(
		header: impl Into<String>,
		items: impl IntoIterator<Item = (impl Into<String>, E)>,
	) -> Self {
		Self {
			header: Some(header.into()),
			items: items
				.into_iter()
				.map(|(label, action)| TextCursorRow::new(label, action))
				.collect(),
			anchor: TextColumnAnchor::TopLeft,
			align: TextColumnAlign::Start,
			selected: 0,
			description: None,
		}
	}

	/// Column of actions with no title above the first row.
	pub fn untitled(items: impl IntoIterator<Item = (impl Into<String>, E)>) -> Self {
		Self {
			header: None,
			items: items
				.into_iter()
				.map(|(label, action)| TextCursorRow::new(label, action))
				.collect(),
			anchor: TextColumnAnchor::TopLeft,
			align: TextColumnAlign::Start,
			selected: 0,
			description: None,
		}
	}

	pub fn rows(
		header: impl Into<String>,
		items: impl IntoIterator<Item = TextCursorRow<E>>,
	) -> Self {
		Self {
			header: Some(header.into()),
			items: items.into_iter().collect(),
			anchor: TextColumnAnchor::TopLeft,
			align: TextColumnAlign::Start,
			selected: 0,
			description: None,
		}
	}

	pub fn anchored(mut self, anchor: TextColumnAnchor) -> Self {
		self.anchor = anchor;
		self
	}

	pub fn aligned(mut self, align: TextColumnAlign) -> Self {
		self.align = align;
		self
	}

	pub fn with_selected(mut self, selected: usize) -> Self {
		self.selected = selected;
		self
	}

	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}

	pub fn scene(self) -> impl Scene + 'static {
		let item_count = self.items.len();
		let selected = if item_count == 0 { 0 } else { self.selected.min(item_count - 1) };
		let mut children: Vec<Box<dyn Scene>> = Vec::with_capacity(
			item_count
				+ usize::from(self.header.is_some())
				+ usize::from(self.description.is_some()),
		);
		if let Some(header) = self.header {
			children.push(Box::new(TextMenuHeader::new(header).scene()));
		}
		for (index, row) in self.items.into_iter().enumerate() {
			children.push(Box::new(cursor_row_scene(
				TextMenuItem::yellow(index),
				row,
				self.align,
				selected,
			)));
		}
		if let Some(description) = self.description {
			children.push(Box::new(TextMenuDescription::under_column(description)));
		}
		let node = self.anchor.node(self.align);
		bsn! {
			TextCursorMenu
			template_value(TextMenu::with_selected(item_count, selected))
			template_value(node)
			Children [ {children} ]
		}
	}
}

/// Cursor-marked action plus a separate caption under it (e.g. Next + progress).
///
/// The mark lives on the label row only, same as [`TextCursorColumn`]. Subtext is
/// its own node, indented to line up with the title.
pub struct ButtonWithSubtext<E> {
	pub label: String,
	pub subtext: String,
	pub action: E,
	pub anchor: TextColumnAnchor,
}

impl<E: Component + Copy + Default + Unpin + Send + Sync + 'static> ButtonWithSubtext<E> {
	pub fn new(label: impl Into<String>, subtext: impl Into<String>, action: E) -> Self {
		Self {
			label: label.into(),
			subtext: subtext.into(),
			action,
			anchor: TextColumnAnchor::BottomRight,
		}
	}

	pub fn anchored(mut self, anchor: TextColumnAnchor) -> Self {
		self.anchor = anchor;
		self
	}

	pub fn scene(self) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(cursor_item_scene(
				TextMenuItem::yellow(0),
				self.label,
				self.action,
				TextColumnAlign::Start,
				0,
			)),
			Box::new(subtext_caption_scene(self.subtext)),
		];
		let node = self.anchor.node(TextColumnAlign::Start);
		bsn! {
			TextCursorMenu
			template_value(TextMenu::new(1))
			template_value(node)
			Children [ {children} ]
		}
	}
}

/// Lower-left screen chrome. Click (and host `B` / Escape) leave the screen.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ScreenBack;

/// One-shot leave request from [`ScreenBack`].
#[derive(Message, Debug, Default, Clone, Copy)]
pub struct ScreenBackPressed;

/// Bottom-left Back control. Not part of the screen's [`TextMenu`] so Enter
/// on the main list cannot fire it.
pub fn screen_back_scene() -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = vec![
		Box::new(cursor_slot_scene(Visibility::Hidden, TextColumnAlign::Start)),
		Box::new(cursor_label_scene(String::from("Back"), TextColumnAlign::Start)),
	];
	bsn! {
		Button
		ScreenBack
		CursorRow
		Node {
			position_type: PositionType::Absolute,
			left: px(CORNER_INSET),
			bottom: px(CORNER_BOTTOM),
			flex_direction: FlexDirection::Row,
			align_items: AlignItems::Center,
			column_gap: px(CURSOR_ICON_GAP),
			padding: UiRect::axes(px(0.0), px(2.0)),
		}
		BackgroundColor(Color::NONE)
		Children [ {children} ]
	}
}

pub fn emit_screen_back_on_click(
	click: On<Pointer<Click>>,
	backs: Query<(), With<ScreenBack>>,
	mut pressed: MessageWriter<ScreenBackPressed>,
) {
	if backs.contains(click.entity) {
		pressed.write(ScreenBackPressed);
	}
}

/// Click on [`ScreenBack`], or pad/keyboard B, while no overlay is open.
pub fn consume_screen_back(
	nav: &MenuNavPad,
	overlay_open: bool,
	backs: &mut MessageReader<ScreenBackPressed>,
) -> bool {
	let clicked = backs.read().next().is_some();
	if overlay_open {
		return false;
	}
	clicked || nav.just_pressed(MenuNav::Back)
}

/// Lower-right screen chrome. Click opens the active character in the editor.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ScreenEdit;

#[derive(Message, Debug, Default, Clone, Copy)]
pub struct ScreenEditPressed;

pub fn screen_edit_scene() -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = vec![
		Box::new(cursor_slot_scene(Visibility::Hidden, TextColumnAlign::Start)),
		Box::new(cursor_label_scene(String::from("Edit"), TextColumnAlign::Start)),
	];
	bsn! {
		Button
		ScreenEdit
		CursorRow
		Node {
			position_type: PositionType::Absolute,
			right: px(CORNER_INSET),
			bottom: px(CORNER_BOTTOM),
			flex_direction: FlexDirection::Row,
			align_items: AlignItems::Center,
			column_gap: px(CURSOR_ICON_GAP),
			padding: UiRect::axes(px(0.0), px(2.0)),
		}
		BackgroundColor(Color::NONE)
		Children [ {children} ]
	}
}

pub fn emit_screen_edit_on_click(
	click: On<Pointer<Click>>,
	edits: Query<(), With<ScreenEdit>>,
	mut pressed: MessageWriter<ScreenEditPressed>,
) {
	if edits.contains(click.entity) {
		pressed.write(ScreenEditPressed);
	}
}

fn cursor_row_scene<E>(
	item: TextMenuItem,
	row: TextCursorRow<E>,
	align: TextColumnAlign,
	selected: usize,
) -> impl Scene + 'static
where
	E: Component + Copy + Default + Unpin + Send + Sync + 'static,
{
	let mut children: Vec<Box<dyn Scene>> =
		vec![Box::new(cursor_item_scene(item, row.label, row.action, align, selected))];
	if let Some(subtext) = row.subtext {
		children.push(Box::new(subtext_caption_scene(subtext)));
	}
	bsn! {
		Node {
			flex_direction: FlexDirection::Column,
			align_items: AlignItems::Start,
		}
		Pickable::IGNORE
		Children [ {children} ]
	}
}

fn subtext_caption_scene(subtext: String) -> impl Scene + 'static {
	let margin = UiRect { left: Val::Px(CURSOR_ICON_SIZE + CURSOR_ICON_GAP), ..default() };
	bsn! {
		Node {
			margin: margin,
		}
		Pickable::IGNORE
		Children [(
			template_value(Text::new(subtext))
			TextFont {
				font: FontSourceTemplate::Handle(BARLOW_SEMIBOLD),
				font_size: px(DESCRIPTION_FONT_SIZE),
			}
			TextColor(TEXT_YELLOW_FAINT)
			TextLayout::new(Justify::Left, bevy::text::LineBreak::NoWrap)
			Pickable::IGNORE
		)]
	}
}

fn cursor_item_scene<E>(
	item: TextMenuItem,
	label: String,
	action: E,
	align: TextColumnAlign,
	selected: usize,
) -> impl Scene + 'static
where
	E: Component + Copy + Default + Unpin + Send + Sync + 'static,
{
	let visibility =
		if item.index == selected { Visibility::Inherited } else { Visibility::Hidden };
	let children: Vec<Box<dyn Scene>> = vec![
		Box::new(cursor_slot_scene(visibility, align)),
		Box::new(cursor_label_scene(label, align)),
	];
	let column_gap = match align {
		TextColumnAlign::Start => Val::Px(CURSOR_ICON_GAP),
		TextColumnAlign::Center => Val::Px(0.0),
	};
	let justify_content = align.justify_content();
	bsn! {
		Button
		template_value(item)
		template_value(action)
		Node {
			padding: UiRect::axes(px(0.0), px(2.0)),
			flex_direction: FlexDirection::Row,
			justify_content: justify_content,
			align_items: AlignItems::Center,
			column_gap: column_gap,
		}
		BackgroundColor(Color::NONE)
		Children [ {children} ]
	}
}

fn cursor_slot_scene(visibility: Visibility, align: TextColumnAlign) -> impl Scene {
	let children: Vec<Box<dyn Scene>> = vec![Box::new(
		AnimatedIcon::maybraid_scene_with_visibility(CURSOR_ICON_SIZE, TEXT_YELLOW, visibility),
	)];
	let node = match align {
		TextColumnAlign::Start => Node {
			width: Val::Px(CURSOR_ICON_SIZE),
			height: Val::Px(CURSOR_ICON_SIZE),
			flex_shrink: 0.0,
			..default()
		},
		TextColumnAlign::Center => Node {
			position_type: PositionType::Absolute,
			left: Val::Px(-(CURSOR_ICON_SIZE + CURSOR_ICON_GAP)),
			top: Val::Px(0.0),
			bottom: Val::Px(0.0),
			width: Val::Px(CURSOR_ICON_SIZE),
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			flex_shrink: 0.0,
			..default()
		},
	};
	bsn! {
		TextCursorSlot
		template_value(node)
		Pickable::IGNORE
		Children [ {children} ]
	}
}

fn cursor_label_scene(label: String, align: TextColumnAlign) -> impl Scene {
	let text_justify = align.text_justify();
	bsn! {
		template_value(Text::new(label))
		TextFont {
			font: FontSourceTemplate::Handle(BARLOW_SEMIBOLD),
			font_size: px(ITEM_FONT_SIZE),
		}
		TextColor(TEXT_YELLOW)
		TextLayout::new(text_justify, bevy::text::LineBreak::NoWrap)
		TextMenuItemLabel
		Pickable::IGNORE
	}
}

/// Show the animated mark only in the selected row’s gutter.
pub fn sync_text_cursor_icons(
	menus: Query<&TextMenu, With<TextCursorMenu>>,
	items: Query<(Entity, &TextMenuItem)>,
	child_of: Query<&ChildOf>,
	children: Query<&Children>,
	slots: Query<(), With<TextCursorSlot>>,
	mut icons: Query<&mut Visibility, With<AnimatedIcon>>,
) {
	for (item_entity, item) in &items {
		let Some(menu) = text_cursor_menu(item_entity, &child_of, &menus) else {
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

fn text_cursor_menu<'a>(
	start: Entity,
	child_of: &Query<&ChildOf>,
	menus: &'a Query<&TextMenu, With<TextCursorMenu>>,
) -> Option<&'a TextMenu> {
	let mut entity = start;
	loop {
		if let Ok(menu) = menus.get(entity) {
			return Some(menu);
		}
		entity = child_of.get(entity).ok()?.parent();
	}
}
