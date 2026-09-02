//! Short-text button plus a modal line and in-game keypad.
//!
//! Toggle opens the modal. Devices without a system OSK get an A–Z / 0–9
//! keypad. iOS and Android hide that keypad and enable IME so the system
//! keyboard types into the same visible line. Submit emits [`ShortTextChange`].

use bevy::ecs::event::EntityEvent;
use bevy::input::ButtonState;
use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;
use bevy::text::{Justify, LineBreak, LineHeight, TextSpan};
use bevy::window::{Ime, PrimaryWindow};

use crate::icons::AnimatedIcon;
use crate::single_select::{KeyboardMenuNav, TextCursorSlot, TextMenuInputLock};
use crate::theme::{
	HEADER_FONT_SIZE, PANEL_CHIP_GAP, PANEL_CURSOR_ICON_GAP, PANEL_HEADER_CURSOR_ICON_SIZE,
	PANEL_HEADER_FONT_SIZE, PANEL_ITEM_FONT_SIZE, PANEL_ROW_GAP, PANEL_VALUE_FONT_SIZE,
	TEXT_YELLOW, TEXT_YELLOW_FAINT,
};
use maybraid_input::{MenuNav, MenuNavImpulse};

use super::HudFonts;
use super::button::spawn_text_button;
use super::display::menu_display_name;
use super::hud_menu::{HudMenu, HudMenuItem};
use super::text::{spawn_cursor_slot_sized, spawn_hud_text};

/// IR / host key for a short-text field.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortTextKey(pub &'static str);

/// Committed value on the HUD row.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct ShortTextField {
	pub value: String,
	pub max_len: usize,
	pub editing: bool,
}

/// Marker on the HUD row's value `TextSpan`.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ShortTextValue;

/// Which field's modal is open.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ActiveShortText(pub Option<&'static str>);

/// Open edit session. The modal is the only text owner while this is `Some`.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct ShortTextModal {
	pub session: Option<ShortTextSession>,
}

impl ShortTextModal {
	pub fn is_open(&self) -> bool {
		self.session.is_some()
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortTextSession {
	pub key: &'static str,
	pub source: Entity,
	pub value: String,
	pub original: String,
	pub max_len: usize,
	pub shift: bool,
}

/// Root of the fullscreen text modal.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ShortTextModalRoot;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ShortTextModalValue;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ShortTextSubmit;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ShortTextCancel;

/// In-game keypad root; absent on devices that already have a system OSK.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ShortTextPad;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortTextPadKey {
	Letter(char),
	Digit(char),
	Space,
	Backspace,
	Shift,
}

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct ShortTextPadLetter(char);

/// True on platforms where winit can show a system soft keyboard.
pub fn system_osk_available() -> bool {
	cfg!(target_os = "ios") || cfg!(target_os = "android")
}

/// Start or stop editing. The button does not interpret the text.
#[derive(EntityEvent, Debug, Clone)]
#[entity_event(propagate, auto_propagate)]
pub struct ShortTextToggle {
	pub entity: Entity,
	pub key: &'static str,
	pub editing: bool,
}

/// Submitted value after confirm.
#[derive(EntityEvent, Debug, Clone)]
#[entity_event(propagate, auto_propagate)]
pub struct ShortTextChange {
	pub entity: Entity,
	pub key: &'static str,
	pub value: String,
}

/// Pickable name row. `extra` is typically [`ShortTextKey`] + [`ShortTextField`] + [`HudMenuItem`].
pub fn spawn_short_text_button(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &'static str,
	value: &str,
	editing: bool,
	justify: JustifyContent,
	extra: impl Bundle,
) {
	parent
		.spawn((
			Button,
			extra,
			Node {
				width: Val::Percent(100.0),
				padding: UiRect::axes(Val::Px(0.0), Val::Px(4.0)),
				flex_direction: FlexDirection::Row,
				justify_content: justify,
				align_items: AlignItems::Center,
				column_gap: Val::Px(PANEL_CURSOR_ICON_GAP),
				..default()
			},
			BackgroundColor(Color::NONE),
		))
		.with_children(|row| {
			spawn_cursor_slot_sized(row, fonts, editing, PANEL_HEADER_CURSOR_ICON_SIZE);
			spawn_short_text_line(row, fonts, label, value);
		});
}

fn spawn_short_text_line(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	value: &str,
) {
	parent
		.spawn((
			Text::new(menu_display_name(label)),
			fonts.header(PANEL_HEADER_FONT_SIZE),
			TextColor(TEXT_YELLOW),
			TextLayout::new(Justify::Left, LineBreak::NoWrap),
			LineHeight::RelativeToFont(1.0),
			Pickable::IGNORE,
		))
		.with_children(|text| {
			text.spawn((
				ShortTextValue,
				TextSpan::new(row_value_display(value)),
				fonts.body(PANEL_HEADER_FONT_SIZE),
				TextColor(TEXT_YELLOW_FAINT),
				LineHeight::RelativeToFont(1.0),
			));
		});
}

fn row_value_display(value: &str) -> String {
	if value.is_empty() { String::from("  —") } else { format!("  {value}") }
}

fn modal_value_display(value: &str) -> String {
	format!("{value}_")
}

pub fn open_short_text_modal(
	entity: Entity,
	key: &'static str,
	field: &mut ShortTextField,
	active: &mut ActiveShortText,
	modal: &mut ShortTextModal,
	commands: &mut Commands,
) {
	field.editing = true;
	active.0 = Some(key);
	modal.session = Some(ShortTextSession {
		key,
		source: entity,
		value: field.value.clone(),
		original: field.value.clone(),
		max_len: field.max_len,
		shift: false,
	});
	commands.trigger(ShortTextToggle { entity, key, editing: true });
}

pub fn cancel_short_text_modal(
	active: &mut ActiveShortText,
	modal: &mut ShortTextModal,
	fields: &mut Query<&mut ShortTextField>,
	commands: &mut Commands,
) {
	let Some(session) = modal.session.take() else {
		return;
	};
	active.0 = None;
	if let Ok(mut field) = fields.get_mut(session.source) {
		field.editing = false;
	}
	commands.trigger(ShortTextToggle { entity: session.source, key: session.key, editing: false });
}

pub fn submit_short_text_modal(
	active: &mut ActiveShortText,
	modal: &mut ShortTextModal,
	fields: &mut Query<&mut ShortTextField>,
	commands: &mut Commands,
) {
	let Some(session) = modal.session.take() else {
		return;
	};
	active.0 = None;
	if let Ok(mut field) = fields.get_mut(session.source) {
		field.value.clone_from(&session.value);
		field.editing = false;
	}
	commands.trigger(ShortTextChange {
		entity: session.source,
		key: session.key,
		value: session.value,
	});
	commands.trigger(ShortTextToggle { entity: session.source, key: session.key, editing: false });
}

pub fn emit_short_text_toggle_on_click(
	click: On<Pointer<Click>>,
	lock: Res<TextMenuInputLock>,
	keys: Query<&ShortTextKey>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut commands: Commands,
) {
	if modal.is_open() {
		if fields.get(click.entity).is_ok_and(|field| field.editing) {
			cancel_short_text_modal(&mut active, &mut modal, &mut fields, &mut commands);
		}
		return;
	}
	if lock.0 {
		return;
	}
	let Ok(key) = keys.get(click.entity) else {
		return;
	};
	let Ok(mut field) = fields.get_mut(click.entity) else {
		return;
	};
	open_short_text_modal(click.entity, key.0, &mut field, &mut active, &mut modal, &mut commands);
}

#[allow(clippy::too_many_arguments)]
pub fn emit_short_text_toggle_on_enter(
	keyboard: Res<ButtonInput<KeyCode>>,
	keyboard_nav: Res<KeyboardMenuNav>,
	lock: Res<TextMenuInputLock>,
	overlay_menus: Query<Entity, With<super::hud_menu::HudOverlayMenu>>,
	menus: Query<&HudMenu>,
	items: Query<(Entity, &HudMenuItem, &ShortTextKey)>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut commands: Commands,
) {
	if !keyboard_nav.is_enabled()
		|| modal.is_open()
		|| !keyboard.just_pressed(KeyCode::Enter)
		|| lock.0
		|| !overlay_menus.is_empty()
	{
		return;
	}
	open_selected_short_text(&menus, &items, &mut fields, &mut active, &mut modal, &mut commands);
}

#[allow(clippy::too_many_arguments)]
pub fn emit_short_text_toggle_on_nav(
	impulse: On<MenuNavImpulse>,
	lock: Res<TextMenuInputLock>,
	overlay_menus: Query<(), With<super::hud_menu::HudOverlayMenu>>,
	menus: Query<&HudMenu>,
	items: Query<(Entity, &HudMenuItem, &ShortTextKey)>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut commands: Commands,
) {
	if lock.0
		|| modal.is_open()
		|| impulse.event().nav != MenuNav::Select
		|| overlay_menus.contains(impulse.entity)
	{
		return;
	}
	open_selected_short_text(&menus, &items, &mut fields, &mut active, &mut modal, &mut commands);
}

fn open_selected_short_text(
	menus: &Query<&HudMenu>,
	items: &Query<(Entity, &HudMenuItem, &ShortTextKey)>,
	fields: &mut Query<&mut ShortTextField>,
	active: &mut ActiveShortText,
	modal: &mut ShortTextModal,
	commands: &mut Commands,
) {
	for (entity, item, key) in items.iter() {
		let Ok(menu) = menus.get(item.menu) else {
			continue;
		};
		if item.index != menu.selected {
			continue;
		}
		let Ok(mut field) = fields.get_mut(entity) else {
			continue;
		};
		open_short_text_modal(entity, key.0, &mut field, active, modal, commands);
		return;
	}
}

pub fn emit_short_text_submit_on_click(
	click: On<Pointer<Click>>,
	submits: Query<(), With<ShortTextSubmit>>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut commands: Commands,
) {
	if submits.get(click.entity).is_err() {
		return;
	}
	submit_short_text_modal(&mut active, &mut modal, &mut fields, &mut commands);
}

pub fn emit_short_text_pad_on_click(
	click: On<Pointer<Click>>,
	keys: Query<&ShortTextPadKey>,
	mut modal: ResMut<ShortTextModal>,
) {
	let Ok(key) = keys.get(click.entity) else {
		return;
	};
	let Some(session) = modal.session.as_mut() else {
		return;
	};
	match *key {
		ShortTextPadKey::Shift => session.shift = !session.shift,
		ShortTextPadKey::Backspace => {
			session.value.pop();
		}
		ShortTextPadKey::Space => {
			push_short_text_char(&mut session.value, session.max_len, ' ');
		}
		ShortTextPadKey::Digit(ch) => {
			push_short_text_char(&mut session.value, session.max_len, ch);
		}
		ShortTextPadKey::Letter(ch) => {
			let ch = if session.shift { ch.to_ascii_uppercase() } else { ch.to_ascii_lowercase() };
			push_short_text_char(&mut session.value, session.max_len, ch);
		}
	}
}

pub fn emit_short_text_cancel_on_click(
	click: On<Pointer<Click>>,
	cancels: Query<(), With<ShortTextCancel>>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut commands: Commands,
) {
	if cancels.get(click.entity).is_err() {
		return;
	}
	cancel_short_text_modal(&mut active, &mut modal, &mut fields, &mut commands);
}

pub fn capture_short_text_input(
	mut reader: MessageReader<KeyboardInput>,
	mut ime: MessageReader<Ime>,
	keyboard: Res<ButtonInput<KeyCode>>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut commands: Commands,
) {
	if !modal.is_open() {
		reader.clear();
		ime.clear();
		return;
	}

	if keyboard.just_pressed(KeyCode::Escape) {
		cancel_short_text_modal(&mut active, &mut modal, &mut fields, &mut commands);
		reader.clear();
		ime.clear();
		return;
	}
	if keyboard.just_pressed(KeyCode::Enter) {
		submit_short_text_modal(&mut active, &mut modal, &mut fields, &mut commands);
		reader.clear();
		ime.clear();
		return;
	}

	let Some(session) = modal.session.as_mut() else {
		return;
	};
	if keyboard.just_pressed(KeyCode::Backspace) {
		session.value.pop();
	}
	for ev in reader.read() {
		if ev.state != ButtonState::Pressed || ev.repeat {
			continue;
		}
		let Some(text) = ev.text.as_ref() else {
			continue;
		};
		for ch in text.chars() {
			if ch == '\r' || ch == '\n' {
				continue;
			}
			push_short_text_char(&mut session.value, session.max_len, ch);
		}
	}
	for ev in ime.read() {
		if let Ime::Commit { value, .. } = ev {
			for ch in value.chars() {
				if ch == '\r' || ch == '\n' {
					continue;
				}
				push_short_text_char(&mut session.value, session.max_len, ch);
			}
		}
	}
}

pub fn sync_short_text_modal(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	modal: Res<ShortTextModal>,
	roots: Query<Entity, With<ShortTextModalRoot>>,
	mut values: Query<&mut Text, With<ShortTextModalValue>>,
) {
	if !modal.is_open() {
		for entity in &roots {
			commands.entity(entity).despawn();
		}
		return;
	}
	let session = modal.session.as_ref().expect("open");
	if roots.is_empty() {
		spawn_short_text_modal(&mut commands, &HudFonts::load(asset_server.as_ref()), session);
		return;
	}
	let display = modal_value_display(&session.value);
	for mut text in &mut values {
		if text.0 != display {
			text.0 = display.clone();
		}
	}
}

pub(crate) fn sync_short_text_pad_shift(
	modal: Res<ShortTextModal>,
	mut letters: Query<(&ShortTextPadLetter, &mut Text)>,
) {
	let shift = modal.session.as_ref().is_some_and(|session| session.shift);
	for (letter, mut text) in &mut letters {
		let display = pad_letter_label(letter.0, shift);
		if text.0 != display {
			text.0 = display;
		}
	}
}

fn spawn_short_text_modal(commands: &mut Commands, fonts: &HudFonts, session: &ShortTextSession) {
	let show_pad = !system_osk_available();
	commands
		.spawn((
			ShortTextModalRoot,
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				padding: UiRect::bottom(Val::Percent(if show_pad { 8.0 } else { 22.0 })),
				..default()
			},
			GlobalZIndex(100),
			Pickable::IGNORE,
		))
		.with_children(|root| {
			root.spawn((
				Button,
				ShortTextCancel,
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(0.0),
					top: Val::Px(0.0),
					width: Val::Percent(100.0),
					height: Val::Percent(100.0),
					..default()
				},
				BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.72)),
			));
			root.spawn((
				Node {
					width: Val::Px(720.0),
					max_width: Val::Percent(92.0),
					flex_direction: FlexDirection::Column,
					align_items: AlignItems::FlexStart,
					row_gap: Val::Px(PANEL_ROW_GAP),
					..default()
				},
				Pickable::default(),
			))
			.with_children(|card| {
				spawn_hud_text(
					card,
					fonts.header(HEADER_FONT_SIZE * 0.42),
					session.key,
					TEXT_YELLOW,
					Justify::Left,
				);
				card.spawn((
					ShortTextModalValue,
					Text::new(modal_value_display(&session.value)),
					fonts.header(HEADER_FONT_SIZE * 0.55),
					TextColor(TEXT_YELLOW),
					TextLayout::new(Justify::Left, LineBreak::NoWrap),
					LineHeight::RelativeToFont(1.0),
					Pickable::IGNORE,
				));
				if show_pad {
					spawn_short_text_pad(card, fonts, session.shift);
				} else {
					spawn_text_button(card, fonts, "submit", ShortTextSubmit);
					spawn_hud_text(
						card,
						fonts.body(PANEL_ITEM_FONT_SIZE),
						"type to enter",
						TEXT_YELLOW_FAINT,
						Justify::Left,
					);
				}
			});
		});
}

const PAD_LETTERS: [&str; 3] = ["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"];
const PAD_DIGITS: &str = "1234567890";
const PAD_KEY_SIZE: f32 = 40.0;
const PAD_WIDE_KEY: f32 = 88.0;

fn spawn_short_text_pad(parent: &mut ChildSpawnerCommands, fonts: &HudFonts, shift: bool) {
	parent
		.spawn((
			ShortTextPad,
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				align_items: AlignItems::FlexStart,
				row_gap: Val::Px(PANEL_CHIP_GAP),
				margin: UiRect::top(Val::Px(PANEL_ROW_GAP)),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|pad| {
			for row in PAD_LETTERS {
				spawn_pad_letter_row(pad, fonts, row, shift);
			}
			spawn_pad_digit_row(pad, fonts);
			spawn_pad_action_row(pad, fonts);
		});
}

fn spawn_pad_letter_row(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	letters: &str,
	shift: bool,
) {
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Row,
				column_gap: Val::Px(PANEL_CHIP_GAP),
				flex_wrap: FlexWrap::Wrap,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			for ch in letters.chars() {
				spawn_pad_key(
					row,
					fonts,
					&pad_letter_label(ch, shift),
					PAD_KEY_SIZE,
					ShortTextPadKey::Letter(ch),
					Some(ch),
				);
			}
		});
}

fn spawn_pad_digit_row(parent: &mut ChildSpawnerCommands, fonts: &HudFonts) {
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Row,
				column_gap: Val::Px(PANEL_CHIP_GAP),
				flex_wrap: FlexWrap::Wrap,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			for ch in PAD_DIGITS.chars() {
				spawn_pad_key(
					row,
					fonts,
					&ch.to_string(),
					PAD_KEY_SIZE,
					ShortTextPadKey::Digit(ch),
					None,
				);
			}
		});
}

fn spawn_pad_action_row(parent: &mut ChildSpawnerCommands, fonts: &HudFonts) {
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Row,
				column_gap: Val::Px(PANEL_CHIP_GAP),
				flex_wrap: FlexWrap::Wrap,
				align_items: AlignItems::Center,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			spawn_pad_key(row, fonts, "shift", PAD_WIDE_KEY, ShortTextPadKey::Shift, None);
			spawn_pad_key(row, fonts, "space", PAD_WIDE_KEY * 2.0, ShortTextPadKey::Space, None);
			spawn_pad_key(row, fonts, "back", PAD_WIDE_KEY, ShortTextPadKey::Backspace, None);
			spawn_pad_key(row, fonts, "submit", PAD_WIDE_KEY, ShortTextSubmit, None);
		});
}

fn spawn_pad_key(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	min_width: f32,
	extra: impl Bundle,
	letter: Option<char>,
) {
	parent
		.spawn((
			Button,
			extra,
			Node {
				min_width: Val::Px(min_width),
				min_height: Val::Px(PAD_KEY_SIZE),
				padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			BackgroundColor(Color::NONE),
		))
		.with_children(|button| {
			let mut text = button.spawn((
				Text::new(label),
				fonts.item(PANEL_VALUE_FONT_SIZE),
				TextColor(TEXT_YELLOW),
				TextLayout::new(Justify::Center, LineBreak::NoWrap),
				LineHeight::RelativeToFont(1.0),
				Pickable::IGNORE,
			));
			if let Some(ch) = letter {
				text.insert(ShortTextPadLetter(ch));
			}
		});
}

fn pad_letter_label(letter: char, shift: bool) -> String {
	if shift { letter.to_ascii_uppercase() } else { letter.to_ascii_lowercase() }.to_string()
}

pub fn sync_short_text_display(
	fields: Query<(Entity, &ShortTextField), Changed<ShortTextField>>,
	children: Query<&Children>,
	mut spans: Query<&mut TextSpan, With<ShortTextValue>>,
) {
	for (entity, field) in &fields {
		refresh_row_span(entity, &field.value, &children, &mut spans);
	}
}

fn push_short_text_char(value: &mut String, max_len: usize, ch: char) -> bool {
	if !is_short_text_char(ch) {
		return false;
	}
	if value.chars().count() >= max_len {
		return false;
	}
	value.push(ch);
	true
}

fn is_short_text_char(ch: char) -> bool {
	ch.is_ascii_alphabetic() || ch.is_ascii_digit() || ch == ' '
}

fn refresh_row_span(
	entity: Entity,
	value: &str,
	children: &Query<&Children>,
	spans: &mut Query<&mut TextSpan, With<ShortTextValue>>,
) {
	let Ok(row_children) = children.get(entity) else {
		return;
	};
	for child in row_children {
		if let Ok(mut span) = spans.get_mut(*child) {
			span.0 = row_value_display(value);
			return;
		}
		let Ok(nested) = children.get(*child) else {
			continue;
		};
		for grandchild in nested {
			if let Ok(mut span) = spans.get_mut(*grandchild) {
				span.0 = row_value_display(value);
				return;
			}
		}
	}
}

pub fn restore_short_text_editing(
	active: Res<ActiveShortText>,
	mut fields: Query<(&ShortTextKey, &mut ShortTextField)>,
) {
	for (key, mut field) in &mut fields {
		let editing = active.0 == Some(key.0);
		if field.editing != editing {
			field.editing = editing;
		}
	}
}

pub fn sync_short_text_cursors(
	fields: Query<(&ShortTextField, Option<&HudMenuItem>, &Children)>,
	menus: Query<&HudMenu>,
	slots: Query<(), With<TextCursorSlot>>,
	children: Query<&Children>,
	mut icons: Query<&mut Visibility, With<AnimatedIcon>>,
) {
	for (field, item, row_children) in &fields {
		let focused = item
			.is_some_and(|item| menus.get(item.menu).is_ok_and(|menu| menu.selected == item.index));
		let show = focused || field.editing;
		for child in row_children {
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

pub fn sync_short_text_ime(
	modal: Res<ShortTextModal>,
	mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
	let editing = modal.is_open() && system_osk_available();
	let Ok(mut window) = windows.single_mut() else {
		return;
	};
	if window.ime_enabled != editing {
		window.ime_enabled = editing;
	}
}

#[cfg(test)]
mod tests {
	use super::{is_short_text_char, pad_letter_label, push_short_text_char};

	#[test]
	fn pad_letters_follow_shift() {
		assert_eq!(pad_letter_label('A', false), "a");
		assert_eq!(pad_letter_label('A', true), "A");
	}

	#[test]
	fn allows_letters_digits_space_and_caps_max_len() {
		let mut value = String::new();
		assert!(push_short_text_char(&mut value, 4, 'A'));
		assert!(push_short_text_char(&mut value, 4, 'b'));
		assert!(push_short_text_char(&mut value, 4, '3'));
		assert!(!push_short_text_char(&mut value, 4, '-'));
		assert!(!push_short_text_char(&mut value, 4, '\''));
		assert!(push_short_text_char(&mut value, 4, ' '));
		assert!(!push_short_text_char(&mut value, 4, 'x'));
		assert_eq!(value, "Ab3 ");
		assert!(is_short_text_char('Z'));
		assert!(is_short_text_char('0'));
		assert!(!is_short_text_char('!'));
	}
}
