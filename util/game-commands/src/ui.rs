use bevy::ecs::event::EntityEvent;
use bevy::input::gamepad::GamepadButton;
use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::picking::hover::HoverMap;
use bevy::prelude::*;

use crate::command::{CommandConsoleOutput, TextEntryFocus, TypedCommandLine};

const HUD_ROOT_HEIGHT_PX: f32 = 276.0;
const HUD_CONSOLE_VIEWPORT_PX: f32 = 200.0;
const SCROLL_LINE_PX: f32 = 14.0;

#[derive(Resource, Clone)]
pub struct GameCommandUiConfig {
	pub title: String,
	pub empty_console_text: String,
	pub root_background: Color,
	pub controls_hint: String,
}

impl Default for GameCommandUiConfig {
	fn default() -> Self {
		Self {
			title: "Game commands - / cmd - F1 drawer - WASD - up/down history - PgUp/PgDn scroll"
				.into(),
			empty_console_text: "Console: (errors & `help` output) - wheel or PgUp/PgDn".into(),
			root_background: Color::srgba(0.1, 0.2, 0.24, 0.82),
			controls_hint: "help - Enter - up/down history - PgUp/PgDn - Shift+up/down scroll"
				.into(),
		}
	}
}

/// Visibility and toggle bindings for the bottom command drawer.
#[derive(Clone, Debug)]
pub struct GameCommandDrawerConfig {
	pub open_at_start: bool,
	pub toggle_keys: Vec<KeyCode>,
	pub toggle_gamepad_buttons: Vec<GamepadButton>,
}

impl Default for GameCommandDrawerConfig {
	fn default() -> Self {
		Self {
			open_at_start: true,
			toggle_keys: vec![KeyCode::F1],
			toggle_gamepad_buttons: vec![GamepadButton::Start],
		}
	}
}

#[derive(Resource, Clone, Copy, Debug)]
pub struct GameCommandDrawerVisible(pub bool);

#[derive(Resource, Clone, Default)]
pub struct GameCommandStatusText(pub String);

#[derive(Component)]
pub struct DebugHudRoot;

#[derive(Component)]
pub struct HudStatusLine;

#[derive(Component)]
pub struct HudConsoleBlock;

#[derive(Component)]
pub struct HudConsoleViewport;

#[derive(EntityEvent, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct ConsoleUiScroll {
	pub entity: Entity,
	pub delta: Vec2,
}

pub struct GameCommandUiPlugin {
	pub config: GameCommandUiConfig,
	pub drawer: GameCommandDrawerConfig,
}

impl Default for GameCommandUiPlugin {
	fn default() -> Self {
		Self { config: GameCommandUiConfig::default(), drawer: GameCommandDrawerConfig::default() }
	}
}

impl GameCommandUiPlugin {
	pub fn new(config: GameCommandUiConfig, drawer: GameCommandDrawerConfig) -> Self {
		Self { config, drawer }
	}
}

impl Plugin for GameCommandUiPlugin {
	fn build(&self, app: &mut App) {
		app.insert_resource(self.config.clone())
			.insert_resource(GameCommandDrawerVisible(self.drawer.open_at_start))
			.insert_resource(GameCommandDrawerConfigResource(self.drawer.clone()))
			.init_resource::<GameCommandStatusText>()
			.add_observer(on_console_viewport_scroll)
			.add_systems(Startup, setup_debug_ui)
			.add_systems(
				Update,
				(
					toggle_game_command_drawer,
					sync_game_command_drawer_visibility,
					update_debug_ui,
					send_console_ui_scroll_events,
					scroll_console_viewport_keyboard,
				),
			);
	}
}

#[derive(Resource, Clone)]
pub struct GameCommandDrawerConfigResource(GameCommandDrawerConfig);

pub fn setup_debug_ui(
	mut commands: Commands,
	config: Res<GameCommandUiConfig>,
	visible: Res<GameCommandDrawerVisible>,
) {
	let status_size = 12.0;
	let console_size = 11.0;
	let visibility = drawer_visibility(*visible);

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
			BackgroundColor(config.root_background),
			visibility,
			DebugHudRoot,
		))
		.with_children(|parent| {
			parent.spawn((
				Text::new(config.title.clone()),
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

pub fn toggle_game_command_drawer(
	keyboard: Res<ButtonInput<KeyCode>>,
	gamepads: Query<&Gamepad>,
	drawer: Res<GameCommandDrawerConfigResource>,
	mut visible: ResMut<GameCommandDrawerVisible>,
) {
	if toggle_binding_pressed(
		&keyboard,
		&gamepads,
		&drawer.0.toggle_keys,
		&drawer.0.toggle_gamepad_buttons,
	) {
		visible.0 = !visible.0;
	}
}

pub fn sync_game_command_drawer_visibility(
	visible: Res<GameCommandDrawerVisible>,
	mut roots: Query<&mut Visibility, With<DebugHudRoot>>,
	mut text_focus: ResMut<TextEntryFocus>,
) {
	if !visible.is_changed() {
		return;
	}
	let visibility = drawer_visibility(*visible);
	for mut root in &mut roots {
		*root = visibility;
	}
	if !visible.0 {
		text_focus.0 = false;
	}
}

pub fn update_debug_ui(
	camera_query: Query<&Transform, With<Camera3d>>,
	mut hud_text: ParamSet<(
		Query<&mut Text, With<HudStatusLine>>,
		Query<&mut Text, With<HudConsoleBlock>>,
	)>,
	mut console_scroll: Query<&mut ScrollPosition, With<HudConsoleViewport>>,
	config: Res<GameCommandUiConfig>,
	status_text: Res<GameCommandStatusText>,
	typed: Res<TypedCommandLine>,
	text_focus: Res<TextEntryFocus>,
	console: Res<CommandConsoleOutput>,
	drawer_visible: Res<GameCommandDrawerVisible>,
) {
	if !drawer_visible.0 {
		return;
	}
	if console.is_changed() {
		for mut sp in &mut console_scroll {
			sp.0 = Vec2::ZERO;
		}
	}

	if let Ok(mut status) = hud_text.p0().single_mut() {
		let command_status = format!(
			"[/] {}  |  buf: {}",
			if text_focus.0 { "cmd ON" } else { "cmd off" },
			if typed.0.is_empty() { "_".into() } else { typed.0.clone() },
		);
		let playground_status =
			if status_text.0.is_empty() { config.title.as_str() } else { status_text.0.as_str() };

		status.0 = if let Ok(transform) = camera_query.single() {
			let pos = transform.translation;
			format!(
				"{playground_status}  |  {command_status}\nCam {:.1}, {:.1}, {:.1}   -   {}",
				pos.x, pos.y, pos.z, config.controls_hint
			)
		} else {
			format!("{playground_status}  |  {command_status}\n{}", config.controls_hint)
		};
	}

	if let Ok(mut block) = hud_text.p1().single_mut() {
		block.0 = if console.0.is_empty() {
			config.empty_console_text.clone()
		} else {
			console.0.clone()
		};
	}
}

pub fn send_console_ui_scroll_events(
	drawer_visible: Res<GameCommandDrawerVisible>,
	mut mouse_wheel_reader: MessageReader<MouseWheel>,
	hover_map: Res<HoverMap>,
	mut commands: Commands,
) {
	if !drawer_visible.0 {
		mouse_wheel_reader.clear();
		return;
	}
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
		let max =
			if delta.x > 0. { scroll_position.x >= max_offset.x } else { scroll_position.x <= 0. };
		if !max {
			scroll_position.x += delta.x;
			delta.x = 0.;
		}
	}

	if node.overflow.y == OverflowAxis::Scroll && delta.y != 0. {
		let max =
			if delta.y > 0. { scroll_position.y >= max_offset.y } else { scroll_position.y <= 0. };
		if !max {
			scroll_position.y += delta.y;
			delta.y = 0.;
		}
	}

	if *delta == Vec2::ZERO {
		scroll.propagate(false);
	}
}

pub fn scroll_console_viewport_keyboard(
	drawer_visible: Res<GameCommandDrawerVisible>,
	keyboard: Res<ButtonInput<KeyCode>>,
	mut q: Query<(&mut ScrollPosition, &Node, &ComputedNode), With<HudConsoleViewport>>,
) {
	if !drawer_visible.0 {
		return;
	}
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

fn drawer_visibility(visible: GameCommandDrawerVisible) -> Visibility {
	if visible.0 { Visibility::Visible } else { Visibility::Hidden }
}

fn toggle_binding_pressed(
	keyboard: &ButtonInput<KeyCode>,
	gamepads: &Query<&Gamepad>,
	keys: &[KeyCode],
	buttons: &[GamepadButton],
) -> bool {
	keys.iter().any(|key| keyboard.just_pressed(*key))
		|| gamepads
			.iter()
			.any(|gamepad| buttons.iter().any(|button| gamepad.just_pressed(*button)))
}
