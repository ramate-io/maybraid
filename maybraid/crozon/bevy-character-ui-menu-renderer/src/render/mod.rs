use bevy::prelude::*;
use character_ui_menu::{AssetValue, CharacterField, MenuEvent, SectionId, SwatchValue};
use crozon_character_ui_menus::{AssetOption, LabelOption, ListValues, SwatchOption};

const BUTTON_HEIGHT: f32 = 22.0;
const ACTIVE: Color = Color::srgba(0.16, 0.34, 0.50, 0.95);
const INACTIVE: Color = Color::srgba(0.18, 0.20, 0.24, 0.92);
const MUTED: Color = Color::srgba(0.72, 0.78, 0.86, 1.0);

#[derive(Component, Clone, Copy, Debug)]
pub struct MenuButton(pub MenuEvent);

/// Renderer-owned thumbnail bridge. The playground can adapt this to its cache.
pub trait MenuThumbnailContext {
	fn image_for_asset(&mut self, asset_path: &'static str) -> Option<Handle<Image>>;
}

pub fn render_section(
	parent: &mut ChildSpawnerCommands,
	section: SectionId,
	open: bool,
	body: impl FnOnce(&mut ChildSpawnerCommands),
) {
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(4.0),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|section_parent| {
			render_button(
				section_parent,
				&format!("{} {}", if open { "v" } else { ">" }, section.label()),
				MenuEvent::ToggleSection(section),
				open,
			);
			if open {
				body(section_parent);
			}
		});
}

pub fn render_single_cycle(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	value_label: &'static str,
	field: CharacterField,
) {
	parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
		text(row, label, 11.0, Color::WHITE);
		render_button(row, "<", MenuEvent::Cycle(field, -1), false);
		text(row, value_label, 11.0, Color::srgb(0.85, 0.95, 1.0));
		render_button(row, ">", MenuEvent::Cycle(field, 1), false);
	});
}

pub fn render_slider(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	value: f32,
	step: f32,
	field: CharacterField,
) {
	parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
		text(row, label, 11.0, Color::WHITE);
		render_button(row, "-", MenuEvent::SliderDelta(field, -step), false);
		text(row, &format!("{value:.2}"), 11.0, Color::srgb(0.85, 0.95, 1.0));
		render_button(row, "+", MenuEvent::SliderDelta(field, step), false);
	});
}

pub fn render_swatch_select<T>(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	active: T,
	to_event: impl Fn(T) -> MenuEvent,
) where
	T: Copy + PartialEq + LabelOption + ListValues + SwatchOption,
{
	parent.spawn((row_node(), Pickable::IGNORE)).with_children(|row| {
		text(row, label, 11.0, Color::WHITE);
		for value in T::values() {
			let active = *value == active;
			row.spawn((
				Button,
				Node {
					width: Val::Px(22.0),
					height: Val::Px(18.0),
					border: UiRect::all(Val::Px(if active { 2.0 } else { 1.0 })),
					..default()
				},
				BorderColor::all(if active { Color::WHITE } else { MUTED }),
				BackgroundColor(color_from_hex(value.color_hex())),
				MenuButton(to_event(*value)),
			));
		}
	});
}

pub fn render_asset_select<T>(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	active: T,
	field: CharacterField,
	to_value: impl Fn(T) -> AssetValue,
	thumbnails: &mut impl MenuThumbnailContext,
) where
	T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
{
	text(parent, label, 12.0, Color::srgb(0.78, 0.84, 0.92));
	parent
		.spawn((
			Node {
				width: Val::Percent(100.0),
				flex_direction: FlexDirection::Row,
				flex_wrap: FlexWrap::Wrap,
				column_gap: Val::Px(6.0),
				row_gap: Val::Px(6.0),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|grid| {
			for value in T::values() {
				let asset = value.asset();
				let _thumbnail = thumbnails.image_for_asset(asset.path);
				render_button(
					grid,
					value.label(),
					MenuEvent::SetAsset(field, to_value(*value)),
					*value == active,
				);
			}
		});
}

pub fn render_multi_select<T>(
	parent: &mut ChildSpawnerCommands,
	label: &'static str,
	selected: &[T],
	to_event: impl Fn(T) -> MenuEvent,
	thumbnails: &mut impl MenuThumbnailContext,
) where
	T: Copy + PartialEq + LabelOption + ListValues + AssetOption,
{
	text(parent, label, 12.0, Color::srgb(0.78, 0.84, 0.92));
	for value in T::values() {
		let asset = value.asset();
		let _thumbnail = thumbnails.image_for_asset(asset.path);
		let active = selected.iter().any(|selected| *selected == *value);
		render_button(parent, value.label(), to_event(*value), active);
	}
}

fn render_button(parent: &mut ChildSpawnerCommands, label: &str, event: MenuEvent, active: bool) {
	parent
		.spawn((
			Button,
			Node {
				min_width: Val::Px(28.0),
				height: Val::Px(BUTTON_HEIGHT),
				padding: UiRect::axes(Val::Px(7.0), Val::Px(2.0)),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			BackgroundColor(if active { ACTIVE } else { INACTIVE }),
			MenuButton(event),
		))
		.with_children(|button| text(button, label, 10.0, Color::WHITE));
}

fn text(parent: &mut ChildSpawnerCommands, value: &str, size: f32, color: Color) {
	parent.spawn((
		Text::new(value.to_string()),
		TextFont { font_size: size, ..default() },
		TextColor(color),
		Pickable::IGNORE,
	));
}

fn row_node() -> Node {
	Node {
		width: Val::Percent(100.0),
		min_height: Val::Px(24.0),
		flex_direction: FlexDirection::Row,
		column_gap: Val::Px(5.0),
		align_items: AlignItems::Center,
		justify_content: JustifyContent::SpaceBetween,
		..default()
	}
}

fn color_from_hex(hex: &str) -> Color {
	let hex = hex.strip_prefix('#').unwrap_or(hex);
	if hex.len() != 6 {
		return INACTIVE;
	}
	let Ok(red) = u8::from_str_radix(&hex[0..2], 16) else {
		return INACTIVE;
	};
	let Ok(green) = u8::from_str_radix(&hex[2..4], 16) else {
		return INACTIVE;
	};
	let Ok(blue) = u8::from_str_radix(&hex[4..6], 16) else {
		return INACTIVE;
	};
	Color::srgb(red as f32 / 255.0, green as f32 / 255.0, blue as f32 / 255.0)
}

#[allow(dead_code)]
fn swatch_event(field: CharacterField, value: SwatchValue) -> MenuEvent {
	MenuEvent::SetSwatch(field, value)
}
