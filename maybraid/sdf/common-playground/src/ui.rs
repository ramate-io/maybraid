use bevy::ecs::event::EntityEvent;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;

use crate::input::CommandConsoleOutput;
use crate::input::TextEntryFocus;
use crate::input::TypedCommandLine;
use crate::preview::PreviewConfig;

/// Fixed total height of the bottom HUD (status + console + padding).
const HUD_ROOT_HEIGHT_PX: f32 = 276.0;
/// Fixed height of the scrollable command / help area.
const HUD_CONSOLE_VIEWPORT_PX: f32 = 200.0;
const SCROLL_LINE_PX: f32 = 14.0;

#[derive(Component)]
pub struct DebugHudRoot;

#[derive(Component)]
pub(crate) struct HudStatusLine;

#[derive(Component)]
pub(crate) struct HudConsoleBlock;

#[derive(Component)]
pub(crate) struct HudConsoleViewport;

/// Wheel / trackpad scrolling for [`HudConsoleViewport`] (mirrors Bevy `examples/ui/scroll.rs`).
#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct ConsoleUiScroll {
	pub entity: Entity,
	pub delta: Vec2,
}

pub fn setup_debug_ui(mut commands: Commands) {
	let status_size = 12.0;
	let console_size = 11.0;

	commands
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				bottom: Val::Px(6.0),
				left: Val::Px(8.0),
				right: Val::Px(8.0),
				height: Val::Px(HUD_ROOT_HEIGHT_PX),
				min_height: Val::Px(HUD_ROOT_HEIGHT_PX),
				max_height: Val::Px(HUD_ROOT_HEIGHT_PX),
				padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
				flex_direction: FlexDirection::Column,
				row_gap: Val::Px(4.0),
				align_items: AlignItems::Stretch,
				flex_shrink: 0.0,
				..default()
			},
			BackgroundColor(Color::hsla(201.0, 0.69, 0.62, 0.82)),
			DebugHudRoot,
		))
		.with_children(|parent| {
			parent.spawn((
				Text::new("SDF playground · Tab/1/2/3 · +/- res · / cmd · WASD · ↑↓ history · ⇧↑↓ scroll"),
				TextFont { font_size: status_size, ..default() },
				TextColor(Color::WHITE),
				HudStatusLine,
			));
			parent
				.spawn((
					Node {
						width: Val::Percent(100.0),
						height: Val::Px(HUD_CONSOLE_VIEWPORT_PX),
						min_height: Val::Px(HUD_CONSOLE_VIEWPORT_PX),
						max_height: Val::Px(HUD_CONSOLE_VIEWPORT_PX),
						flex_shrink: 0.0,
						overflow: Overflow::scroll_y(),
						..default()
					},
					ScrollPosition::default(),
					BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.12)),
					Pickable::default(),
					HudConsoleViewport,
				))
				.with_children(|viewport| {
					viewport
						.spawn((
							Node {
								width: Val::Percent(100.0),
								flex_direction: FlexDirection::Column,
								..default()
							},
							Pickable::IGNORE,
						))
						.with_children(|col| {
							col.spawn((
								Text::new(""),
								TextFont { font_size: console_size, ..default() },
								TextColor(Color::srgba(0.95, 0.98, 1.0, 1.0)),
								Pickable::IGNORE,
								HudConsoleBlock,
							));
						});
				});
		});
}

pub fn send_console_ui_scroll_events(
	mut mouse_wheel_reader: MessageReader<MouseWheel>,
	hover_map: Res<HoverMap>,
	mut commands: Commands,
) {
	for mouse_wheel in mouse_wheel_reader.read() {
		let mut delta = -Vec2::new(mouse_wheel.x, mouse_wheel.y);
		if mouse_wheel.unit == MouseScrollUnit::Line {
			delta *= SCROLL_LINE_PX;
		}
		for pointer_map in hover_map.values() {
			for entity in pointer_map.keys().copied() {
				commands.trigger(ConsoleUiScroll { entity, delta });
			}
		}
	}
}

pub fn on_console_viewport_scroll(
	mut scroll: On<ConsoleUiScroll>,
	mut query: Query<(&mut ScrollPosition, &Node, &ComputedNode), With<HudConsoleViewport>>,
) {
	let Ok((mut scroll_position, node, computed)) = query.get_mut(scroll.entity) else {
		return;
	};

	let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();
	let delta = &mut scroll.delta;

	if node.overflow.x == OverflowAxis::Scroll && delta.x != 0. {
		let max = if delta.x > 0. {
			scroll_position.x >= max_offset.x
		} else {
			scroll_position.x <= 0.
		};
		if !max {
			scroll_position.x += delta.x;
			delta.x = 0.;
		}
	}

	if node.overflow.y == OverflowAxis::Scroll && delta.y != 0. {
		let max = if delta.y > 0. {
			scroll_position.y >= max_offset.y
		} else {
			scroll_position.y <= 0.
		};
		if !max {
			scroll_position.y += delta.y;
			delta.y = 0.;
		}
	}

	if *delta == Vec2::ZERO {
		scroll.propagate(false);
	}
}

/// PageUp / PageDown to scroll the console (wheel also works over the console panel).
pub fn scroll_console_viewport_keyboard(
	keyboard: Res<ButtonInput<KeyCode>>,
	mut q: Query<(&mut ScrollPosition, &Node, &ComputedNode), With<HudConsoleViewport>>,
) {
	let Ok((mut scroll_position, node, computed)) = q.single_mut() else {
		return;
	};
	if node.overflow.y != OverflowAxis::Scroll {
		return;
	}
	let max_offset = (computed.content_size() - computed.size()) * computed.inverse_scale_factor();
	let max_y = max_offset.y.max(0.);
	let step = SCROLL_LINE_PX * 3.0;
	let shift = keyboard.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);
	if keyboard.just_pressed(KeyCode::PageUp) {
		scroll_position.y = (scroll_position.y - step * 4.0).max(0.);
	}
	if keyboard.just_pressed(KeyCode::PageDown) {
		scroll_position.y = (scroll_position.y + step * 4.0).min(max_y);
	}
	if shift && keyboard.just_pressed(KeyCode::ArrowUp) {
		scroll_position.y = (scroll_position.y - step).max(0.);
	}
	if shift && keyboard.just_pressed(KeyCode::ArrowDown) {
		scroll_position.y = (scroll_position.y + step).min(max_y);
	}
}

fn panel_status(config: &PreviewConfig, line: &TypedCommandLine, text_focus: &TextEntryFocus) -> String {
	format!(
		"{} ({})  res_2={}  |  [/] {}  |  buf: {}",
		config.primitive,
		config.primitive.variant_key(),
		config.res_2,
		if text_focus.0 { "cmd ON" } else { "cmd off" },
		if line.0.is_empty() {
			"_".into()
		} else {
			line.0.clone()
		},
	)
}

pub(crate) fn update_debug_ui(
	camera_query: Query<&Transform, With<Camera3d>>,
	mut hud_text: ParamSet<(
		Query<&mut Text, With<HudStatusLine>>,
		Query<&mut Text, With<HudConsoleBlock>>,
	)>,
	mut console_scroll: Query<&mut ScrollPosition, With<HudConsoleViewport>>,
	config: Res<PreviewConfig>,
	typed: Res<TypedCommandLine>,
	text_focus: Res<TextEntryFocus>,
	console: Res<CommandConsoleOutput>,
) {
	let Ok(transform) = camera_query.single() else {
		return;
	};
	let pos = transform.translation;

	if console.is_changed() {
		for mut sp in &mut console_scroll {
			sp.0 = Vec2::ZERO;
		}
	}

	if let Ok(mut status) = hud_text.p0().single_mut() {
		status.0 = format!(
			"{}\nCam {:.1}, {:.1}, {:.1}   ·   help · Enter · ↑↓ hist · PgUp/PgDn · ⇧↑↓",
			panel_status(&config, &typed, &text_focus),
			pos.x,
			pos.y,
			pos.z
		);
	}
	if let Ok(mut block) = hud_text.p1().single_mut() {
		block.0 = if console.0.is_empty() {
			"Console: (errors & `help` output) — wheel or PgUp/PgDn".into()
		} else {
			console.0.clone()
		};
	}
}
