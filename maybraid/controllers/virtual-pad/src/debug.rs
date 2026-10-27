//! On-screen HID → pad dump. Enable with [`crate::VirtualPadConfig::debug_overlay`].

use bevy::prelude::*;

use crate::produce::gamepad::GamepadAxes;
use crate::VirtualPadController;

#[derive(Component)]
struct VirtualPadDebugText;

pub fn configure_debug(app: &mut App) {
	app.add_systems(Startup, spawn_debug_overlay)
		.add_systems(Update, update_debug_overlay);
}

fn spawn_debug_overlay(controllers: Query<&VirtualPadController>, mut commands: Commands) {
	if !controllers.iter().any(|controller| controller.config.debug_overlay) {
		return;
	}
	commands.spawn((
		VirtualPadDebugText,
		Text::new("pad debug"),
		TextFont { font_size: FontSize::Px(13.0), ..default() },
		TextColor(Color::srgb(0.95, 0.9, 0.45)),
		Node {
			position_type: PositionType::Absolute,
			top: Val::Px(8.0),
			left: Val::Px(8.0),
			width: Val::Px(720.0),
			..default()
		},
		GlobalZIndex(i32::MAX),
		Pickable::IGNORE,
	));
}

fn update_debug_overlay(
	controllers: Query<(Entity, &VirtualPadController)>,
	gamepads: Query<(Entity, &Gamepad)>,
	mut lines: Query<&mut Text, With<VirtualPadDebugText>>,
) {
	let Ok(mut text) = lines.single_mut() else {
		return;
	};
	let mut out = String::new();
	for (entity, controller) in &controllers {
		if !controller.config.debug_overlay {
			continue;
		}
		let pad = &controller.pad;
		out.push_str(&format!(
			"{entity} enabled={} move=({:.2},{:.2}) dpad=({:.2},{:.2}) A={} B={} Start={} Select={}\n",
			!controller.policy.suppress_gameplay(), pad.move_stick.x, pad.move_stick.y, pad.dpad.x, pad.dpad.y,
			pad.pressed(crate::PadButton::A), pad.pressed(crate::PadButton::B), pad.pressed(crate::PadButton::Start), pad.pressed(crate::PadButton::Select),
		));
	}

	if gamepads.is_empty() {
		out.push_str("no Gamepad components\n");
	}
	for (entity, gamepad) in &gamepads {
		let left = gamepad.left_stick();
		let right = gamepad.right_stick();
		let dpad = gamepad.dpad();
		let pressed: Vec<_> = gamepad.get_pressed().copied().collect();
		out.push_str(&format!(
			"{entity} left=({:.2},{:.2}) right=({:.2},{:.2}) dpad=({:.2},{:.2}) pressed={pressed:?}\n",
			left.x, left.y, right.x, right.y, dpad.x, dpad.y
		));
		let analog = GamepadAxes::analog_dump(gamepad);
		if !analog.is_empty() {
			out.push_str(&format!("  analog {analog}\n"));
		}
	}
	text.0 = out;
}
