use bevy::prelude::*;
use character_ui_menu::ThumbnailCamera;

pub const BUTTON_HEIGHT: f32 = 22.0;
pub const ACTIVE: Color = Color::srgba(0.16, 0.34, 0.50, 0.95);
pub const INACTIVE: Color = Color::srgba(0.18, 0.20, 0.24, 0.92);
pub const MUTED: Color = Color::srgba(0.72, 0.78, 0.86, 1.0);

#[derive(Component, Clone, Copy, Debug)]
pub struct MenuButton<E: Copy + Send + Sync + 'static>(pub E);

#[derive(Component, Clone, Copy, Debug)]
pub struct ToggleSectionKey(pub &'static str);

#[derive(Component, Clone, Copy, Debug)]
pub struct AssetThumbnailHover {
	pub label: &'static str,
	pub path: &'static str,
	pub color: Color,
	pub camera: ThumbnailCamera,
}

pub fn render_button<E: Copy + Send + Sync + 'static>(
	parent: &mut ChildSpawnerCommands,
	label: &str,
	event: E,
	active: bool,
) {
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

pub fn render_asset_button<E: Copy + Send + Sync + 'static>(
	parent: &mut ChildSpawnerCommands,
	label: &str,
	event: E,
	active: bool,
	thumbnail: Option<Handle<Image>>,
) {
	parent
		.spawn((
			Button,
			Node {
				min_width: Val::Px(72.0),
				min_height: Val::Px(54.0),
				padding: UiRect::axes(Val::Px(5.0), Val::Px(4.0)),
				flex_direction: FlexDirection::Column,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				row_gap: Val::Px(3.0),
				..default()
			},
			BackgroundColor(if active { ACTIVE } else { INACTIVE }),
			MenuButton(event),
		))
		.with_children(|button| {
			if let Some(thumbnail) = thumbnail {
				button.spawn((
					ImageNode::new(thumbnail),
					Node { width: Val::Px(54.0), height: Val::Px(54.0), ..default() },
					Pickable::IGNORE,
				));
			}
			text(button, label, 9.0, Color::WHITE);
		});
}

pub fn text(parent: &mut ChildSpawnerCommands, value: &str, size: f32, color: Color) {
	parent.spawn((
		Text::new(value.to_string()),
		TextFont { font_size: size, ..default() },
		TextColor(color),
		Pickable::IGNORE,
	));
}

pub fn row_node() -> Node {
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

pub fn color_from_hex(hex: &str) -> Color {
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
