//! Poll Apple `GCController` into Bevy raw gamepad messages.

use std::collections::{HashMap, HashSet};
use std::ptr::from_ref;

use bevy::input::gamepad::{
	GamepadAxis, GamepadButton, GamepadConnection, GamepadConnectionEvent,
	RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use bevy::input::InputSystems;
use bevy::prelude::*;
use objc2_game_controller::{
	GCController, GCControllerButtonInput, GCControllerDirectionPad, GCDevice, GCExtendedGamepad,
};

use super::value_changed;

pub struct GameControllerPlugin;

impl Plugin for GameControllerPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<GameControllerPads>()
			.add_systems(PreStartup, (enable_background_events, poll_game_controllers).chain())
			.add_systems(PreUpdate, poll_game_controllers.before(InputSystems));
	}
}

#[derive(Default, Resource)]
struct GameControllerPads {
	pads: HashMap<usize, PadState>,
}

struct PadState {
	entity: Entity,
	axes: HashMap<GamepadAxis, f32>,
	buttons: HashMap<GamepadButton, f32>,
}

fn controller_id(controller: &GCController) -> usize {
	from_ref(controller) as usize
}

fn enable_background_events() {
	// SAFETY: class method; Bevy runs this on the main thread that pumps CFRunLoop.
	unsafe {
		GCController::setShouldMonitorBackgroundEvents(true);
	}
}

fn poll_game_controllers(
	mut commands: Commands,
	mut pads: ResMut<GameControllerPads>,
	mut connection_events: MessageWriter<GamepadConnectionEvent>,
	mut raw_events: MessageWriter<RawGamepadEvent>,
) {
	// SAFETY: `controllers()` is the GameController inventory for this process.
	let controllers = unsafe { GCController::controllers() };
	let mut seen = HashSet::new();
	for index in 0..controllers.count() {
		let controller = controllers.objectAtIndex(index);
		let id = controller_id(&controller);
		seen.insert(id);
		sync_controller(
			&controller,
			id,
			&mut commands,
			&mut pads,
			&mut connection_events,
			&mut raw_events,
		);
	}

	let gone: Vec<usize> = pads.pads.keys().copied().filter(|id| !seen.contains(id)).collect();
	for id in gone {
		let Some(state) = pads.pads.remove(&id) else {
			continue;
		};
		let event = GamepadConnectionEvent::new(state.entity, GamepadConnection::Disconnected);
		connection_events.write(event.clone());
		raw_events.write(event.into());
	}
}

fn sync_controller(
	controller: &GCController,
	id: usize,
	commands: &mut Commands,
	pads: &mut GameControllerPads,
	connection_events: &mut MessageWriter<GamepadConnectionEvent>,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
) {
	// SAFETY: live `GCController` from `controllers()`.
	let Some(extended) = (unsafe { controller.extendedGamepad() }) else {
		return;
	};
	let state = pads.pads.entry(id).or_insert_with(|| {
		let entity = commands.spawn_empty().id();
		let event = GamepadConnectionEvent::new(
			entity,
			GamepadConnection::Connected {
				name: controller_name(controller),
				vendor_id: None,
				product_id: None,
			},
		);
		connection_events.write(event.clone());
		raw_events.write(event.into());
		PadState { entity, axes: HashMap::new(), buttons: HashMap::new() }
	});
	emit_extended(state, &extended, raw_events);
}

fn controller_name(controller: &GCController) -> String {
	// SAFETY: `vendorName` / `productCategory` are read-only NSString copies.
	unsafe {
		if let Some(vendor) = controller.vendorName() {
			let name = vendor.to_string();
			if !name.is_empty() {
				return name;
			}
		}
		controller.productCategory().to_string()
	}
}

fn emit_extended(
	state: &mut PadState,
	pad: &GCExtendedGamepad,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
) {
	// SAFETY: `pad` is the extended profile of a connected controller.
	unsafe {
		emit_stick(
			state,
			raw_events,
			GamepadAxis::LeftStickX,
			GamepadAxis::LeftStickY,
			&pad.leftThumbstick(),
		);
		emit_stick(
			state,
			raw_events,
			GamepadAxis::RightStickX,
			GamepadAxis::RightStickY,
			&pad.rightThumbstick(),
		);
		emit_button(state, raw_events, GamepadButton::South, &pad.buttonA());
		emit_button(state, raw_events, GamepadButton::East, &pad.buttonB());
		emit_button(state, raw_events, GamepadButton::West, &pad.buttonX());
		emit_button(state, raw_events, GamepadButton::North, &pad.buttonY());
		emit_button(state, raw_events, GamepadButton::LeftTrigger, &pad.leftShoulder());
		emit_button(state, raw_events, GamepadButton::RightTrigger, &pad.rightShoulder());
		emit_button(state, raw_events, GamepadButton::LeftTrigger2, &pad.leftTrigger());
		emit_button(state, raw_events, GamepadButton::RightTrigger2, &pad.rightTrigger());
		emit_button(state, raw_events, GamepadButton::Start, &pad.buttonMenu());
		if let Some(options) = pad.buttonOptions() {
			emit_button(state, raw_events, GamepadButton::Select, &options);
		}
		if let Some(home) = pad.buttonHome() {
			emit_button(state, raw_events, GamepadButton::Mode, &home);
		}
		if let Some(click) = pad.leftThumbstickButton() {
			emit_button(state, raw_events, GamepadButton::LeftThumb, &click);
		}
		if let Some(click) = pad.rightThumbstickButton() {
			emit_button(state, raw_events, GamepadButton::RightThumb, &click);
		}
		let dpad = pad.dpad();
		emit_button(state, raw_events, GamepadButton::DPadUp, &dpad.up());
		emit_button(state, raw_events, GamepadButton::DPadDown, &dpad.down());
		emit_button(state, raw_events, GamepadButton::DPadLeft, &dpad.left());
		emit_button(state, raw_events, GamepadButton::DPadRight, &dpad.right());
	}
}

/// # Safety
/// `stick` must be a live direction pad from a connected `GCExtendedGamepad`.
unsafe fn emit_stick(
	state: &mut PadState,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
	axis_x: GamepadAxis,
	axis_y: GamepadAxis,
	stick: &GCControllerDirectionPad,
) {
	emit_axis(state, raw_events, axis_x, unsafe { stick.xAxis().value() });
	emit_axis(state, raw_events, axis_y, unsafe { stick.yAxis().value() });
}

fn emit_axis(
	state: &mut PadState,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
	axis: GamepadAxis,
	value: f32,
) {
	if !value_changed(state.axes.get(&axis).copied(), value) {
		return;
	}
	state.axes.insert(axis, value);
	raw_events.write(RawGamepadAxisChangedEvent::new(state.entity, axis, value).into());
}

/// # Safety
/// `input` must be a live button from a connected `GCExtendedGamepad`.
unsafe fn emit_button(
	state: &mut PadState,
	raw_events: &mut MessageWriter<RawGamepadEvent>,
	button: GamepadButton,
	input: &GCControllerButtonInput,
) {
	let value = unsafe { input.value() };
	if !value_changed(state.buttons.get(&button).copied(), value) {
		return;
	}
	state.buttons.insert(button, value);
	raw_events.write(RawGamepadButtonChangedEvent::new(state.entity, button, value).into());
}
