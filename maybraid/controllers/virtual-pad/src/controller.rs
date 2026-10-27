//! Source-owned pad state and explicit physical-device assignments.

use std::time::Duration;

use bevy::input::keyboard::KeyboardInput;
use bevy::input::mouse::MouseMotion;
use bevy::prelude::*;

use crate::{
	MenuNavPad, PadCursor, PadEdge, PadHistory, PadInputPolicy, PadOverlay, VirtualPad,
	VirtualPadConfig,
};

/// Physical devices intentionally combined into one controller.
///
/// Separate controller entities never implicitly union their device state.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PadDevices {
	pub keyboard: bool,
	pub mouse: bool,
	pub gamepads: Vec<Entity>,
}

impl PadDevices {
	pub fn keyboard_mouse() -> Self {
		Self { keyboard: true, mouse: true, gamepads: Vec::new() }
	}

	pub fn gamepad(entity: Entity) -> Self {
		Self::default().with_gamepad(entity)
	}

	pub fn with_gamepad(mut self, entity: Entity) -> Self {
		if !self.gamepads.contains(&entity) {
			self.gamepads.push(entity);
		}
		self
	}
}

/// One virtual controller owns its rich state, history and device assignment.
#[derive(Component, Clone, Debug)]
pub struct VirtualPadController {
	pub devices: PadDevices,
	pub config: VirtualPadConfig,
	pub pad: VirtualPad,
	pub policy: PadInputPolicy,
	pub history: PadHistory,
	pub cursor: PadCursor,
	/// Temporary compatibility surface; routed consumers interpret the rich pad.
	pub legacy_menu_nav: MenuNavPad,
	sequence: u64,
	previous_devices: Option<PadDevices>,
	connected: bool,
}

impl VirtualPadController {
	pub fn new(devices: PadDevices) -> Self {
		Self {
			devices,
			config: VirtualPadConfig::default(),
			pad: VirtualPad::default(),
			policy: PadInputPolicy::default(),
			history: PadHistory::default(),
			cursor: PadCursor::default(),
			legacy_menu_nav: MenuNavPad::default(),
			sequence: 0,
			previous_devices: None,
			connected: true,
		}
	}

	pub fn with_config(mut self, config: VirtualPadConfig) -> Self {
		self.config = config;
		self
	}

	pub fn connected(&self) -> bool {
		self.connected
	}

	pub fn sequence(&self) -> u64 {
		self.sequence
	}

	fn finish_frame(&mut self, config: &VirtualPadConfig, delta: Duration, now: f32) {
		if self.policy.suppress_gameplay() {
			self.pad.suppress_gameplay();
		}
		self.pad.apply_trigger_digital(config.trigger_press_threshold);
		self.pad.finish_digital();
		self.pad.tick_holds(delta.as_secs_f32());
		self.pad.move_stick = self.pad.move_stick.clamp_length_max(1.0);
		self.pad.look_throw = self.pad.look_throw.clamp_length_max(1.0);
		self.pad.dpad = self.pad.dpad.clamp_length_max(1.0);
		self.history.push_frame(now, self.pad.snapshot());
		for stroke in &self.pad.button_events {
			self.history.push_edge(now, PadEdge::Button(*stroke));
		}
		for stroke in &self.pad.key_events {
			self.history.push_edge(now, PadEdge::Key(*stroke));
		}
		self.history.evict(now, config.history_window, config.history_max_frames);
		self.cursor.integrate(&self.pad, config.cursor_speed, delta.as_secs_f32());
		self.legacy_menu_nav.derive(&self.pad, config, delta.as_secs_f32());
		self.sequence = self.sequence.wrapping_add(1);
	}
}

/// Rich, immutable source snapshot. Menu interpretation belongs to its receiver.
#[derive(Message, Clone, Debug)]
pub struct VirtualPadEvent {
	pub source: Entity,
	pub sequence: u64,
	pub delta: Duration,
	pub pad: VirtualPad,
}

/// Source-device changes retire in-flight delivery in the routing owner.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct VirtualPadLifecycle {
	pub source: Entity,
	pub connected: bool,
}

/// Explicit single-player compatibility controller, also used by agent injection.
#[derive(Resource, Clone, Copy, Debug)]
pub struct PrimaryVirtualPad(pub Entity);

#[derive(Resource, Default)]
pub(crate) struct DefaultDeviceAssignments {
	seen: Vec<Entity>,
}

/// Default single-player host: keyboard/mouse and the first pad share one source.
/// Additional gamepads are separate sources and await explicit actor assignment.
pub(crate) fn discover_devices(
	mut commands: Commands,
	primary: Res<PrimaryVirtualPad>,
	gamepads: Query<Entity, With<Gamepad>>,
	mut controllers: Query<&mut VirtualPadController>,
	mut assignments: ResMut<DefaultDeviceAssignments>,
) {
	let assigned: Vec<_> = controllers
		.iter()
		.flat_map(|controller| controller.devices.gamepads.iter().copied())
		.collect();
	let mut fresh: Vec<_> =
		gamepads.iter().filter(|entity| !assignments.seen.contains(entity)).collect();
	fresh.sort_by_key(|entity| entity.to_bits());
	for entity in fresh {
		assignments.seen.push(entity);
		if assigned.contains(&entity) {
			continue;
		}
		if let Ok(mut controller) = controllers.get_mut(primary.0) {
			if controller.devices.gamepads.iter().all(|device| !gamepads.contains(*device)) {
				controller.devices.gamepads.clear();
				controller.devices.gamepads.push(entity);
				continue;
			}
		}
		let mut vacant: Vec<_> = controllers
			.iter()
			.enumerate()
			.filter_map(|(index, controller)| {
				(!controller.devices.keyboard
					&& !controller.devices.mouse
					&& !controller.connected
					&& controller.devices.gamepads.iter().all(|device| !gamepads.contains(*device)))
				.then_some(index)
			})
			.collect();
		if let Some(index) = vacant.pop() {
			if let Some(mut controller) = controllers.iter_mut().nth(index) {
				controller.devices = PadDevices::gamepad(entity);
				continue;
			}
		}
		let config = controllers
			.get(primary.0)
			.map(|controller| controller.config.clone())
			.unwrap_or_default();
		commands.spawn(VirtualPadController::new(PadDevices::gamepad(entity)).with_config(config));
	}
}

pub(crate) fn produce_controllers(
	time: Res<Time>,
	keyboard: Res<ButtonInput<KeyCode>>,
	mouse: Res<ButtonInput<MouseButton>>,
	mut key_events: MessageReader<KeyboardInput>,
	mut mouse_events: MessageReader<MouseMotion>,
	gamepads: Query<&Gamepad>,
	primary: Res<PrimaryVirtualPad>,
	legacy_policy: Res<PadInputPolicy>,
	legacy_overlay: Option<Res<PadOverlay>>,
	mut controllers: Query<(Entity, &mut VirtualPadController, Option<&PadOverlay>)>,
	mut lifecycle: MessageWriter<VirtualPadLifecycle>,
) {
	let keys: Vec<_> = key_events
		.read()
		.map(|event| crate::ButtonStroke {
			button: event.key_code,
			phase: crate::ButtonPhase::from_button_state(event.state),
			repeat: event.repeat,
		})
		.collect();
	let mouse_delta: Vec2 = mouse_events.read().map(|event| event.delta).sum();
	let command_held = keyboard.any_pressed([KeyCode::SuperLeft, KeyCode::SuperRight]);
	for (entity, mut controller, overlay) in &mut controllers {
		let devices = controller.devices.clone();
		let config = controller.config.clone();
		let changed = controller
			.previous_devices
			.as_ref()
			.is_some_and(|previous| previous != &devices);
		let was_connected = controller.connected;
		let connected = devices.keyboard
			|| devices.mouse
			|| devices.gamepads.iter().any(|gamepad| gamepads.contains(*gamepad))
			|| overlay.is_some();
		if changed || connected != controller.connected {
			lifecycle.write(VirtualPadLifecycle { source: entity, connected });
			controller.history = PadHistory::default();
			controller.legacy_menu_nav = MenuNavPad::default();
		}
		controller.connected = connected;
		controller.previous_devices = Some(devices.clone());
		if entity == primary.0 {
			controller.policy = *legacy_policy;
		}
		controller.pad.begin_frame();
		if devices.keyboard {
			if controller.policy.keyboard_menu() {
				crate::produce::keyboard::produce_menu_keys(&keyboard, &mut controller.pad);
			} else {
				crate::produce::keyboard::produce_gameplay_keys(&keyboard, &mut controller.pad);
			}
			controller.pad.keys = keyboard.clone();
			controller.pad.key_events = keys.clone();
		} else {
			controller.pad.keys = ButtonInput::default();
		}
		if devices.mouse && !command_held {
			controller.pad.add_look(mouse_delta);
			if mouse.pressed(MouseButton::Right) {
				controller.pad.max_triggers(1.0, 0.0);
			}
			if mouse.pressed(MouseButton::Left) {
				controller.pad.max_triggers(0.0, 1.0);
			}
		}
		for gamepad in &devices.gamepads {
			if let Ok(gamepad) = gamepads.get(*gamepad) {
				crate::produce::gamepad::GamepadAxes::apply(
					gamepad,
					&config,
					time.delta_secs(),
					&mut controller.pad,
				);
			}
		}
		if let Some(overlay) = overlay {
			overlay.apply(&mut controller.pad);
		}
		if entity == primary.0 {
			if let Some(overlay) = &legacy_overlay {
				overlay.apply(&mut controller.pad);
			}
		}
		if changed || (connected && !was_connected) {
			// Device transfer cancels holds; a still-held device must release first.
			controller.pad.suppress_gameplay();
		}
		controller.finish_frame(&config, time.delta(), time.elapsed_secs());
	}
}

/// Publish entity-owned snapshots and mirror only the explicitly designated pad.
pub(crate) fn publish_controllers(
	primary: Res<PrimaryVirtualPad>,
	time: Res<Time>,
	controllers: Query<(Entity, &VirtualPadController)>,
	mut events: MessageWriter<VirtualPadEvent>,
	mut legacy_pad: ResMut<VirtualPad>,
	mut legacy_history: ResMut<PadHistory>,
	mut legacy_cursor: ResMut<PadCursor>,
	mut legacy_nav: ResMut<MenuNavPad>,
) {
	for (entity, controller) in &controllers {
		events.write(VirtualPadEvent {
			source: entity,
			sequence: controller.sequence,
			delta: time.delta(),
			pad: controller.pad.clone(),
		});
		if entity == primary.0 {
			*legacy_pad = controller.pad.clone();
			*legacy_history = controller.history.clone();
			*legacy_cursor = controller.cursor.clone();
			*legacy_nav = controller.legacy_menu_nav.clone();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::PadButton;

	fn app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			bevy::input::InputPlugin,
			crate::VirtualPadPlugin { auto_assign_devices: false, ..default() },
		));
		app
	}

	#[test]
	fn two_gamepads_keep_buttons_edges_and_history_independent() -> anyhow::Result<()> {
		let mut app = app();
		let mut left = Gamepad::default();
		left.digital_mut().press(GamepadButton::South);
		let left_device = app.world_mut().spawn(left).id();
		let right_device = app.world_mut().spawn(Gamepad::default()).id();
		let left = app
			.world_mut()
			.spawn(VirtualPadController::new(PadDevices::gamepad(left_device)))
			.id();
		let right = app
			.world_mut()
			.spawn(VirtualPadController::new(PadDevices::gamepad(right_device)))
			.id();
		app.update();
		let left_pad = app
			.world()
			.get::<VirtualPadController>(left)
			.ok_or_else(|| anyhow::anyhow!("left controller"))?;
		let right_pad = app
			.world()
			.get::<VirtualPadController>(right)
			.ok_or_else(|| anyhow::anyhow!("right controller"))?;
		assert!(left_pad.pad.just_pressed(PadButton::A));
		assert!(!right_pad.pad.pressed(PadButton::A));
		assert_eq!(left_pad.history.edges.len(), 1);
		assert!(right_pad.history.edges.is_empty());
		app.update();
		assert!(!app
			.world()
			.get::<VirtualPadController>(left)
			.ok_or_else(|| anyhow::anyhow!("left controller"))?
			.pad
			.just_pressed(PadButton::A));
		app.world_mut().despawn(left_device);
		app.update();
		let left_pad = app
			.world()
			.get::<VirtualPadController>(left)
			.ok_or_else(|| anyhow::anyhow!("left controller"))?;
		assert!(!left_pad.connected());
		assert!(left_pad.pad.just_released(PadButton::A));
		assert!(app
			.world()
			.get::<VirtualPadController>(right)
			.is_some_and(VirtualPadController::connected));
		Ok(())
	}

	#[test]
	fn keyboard_and_mouse_combine_only_when_assigned() -> anyhow::Result<()> {
		let mut app = app();
		let primary = app.world().resource::<PrimaryVirtualPad>().0;
		let isolated = app.world_mut().spawn(VirtualPadController::new(PadDevices::default())).id();
		app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyW);
		app.world_mut()
			.resource_mut::<ButtonInput<MouseButton>>()
			.press(MouseButton::Left);
		app.update();
		let combined = app
			.world()
			.get::<VirtualPadController>(primary)
			.ok_or_else(|| anyhow::anyhow!("combined controller"))?;
		assert_eq!(combined.pad.move_stick, Vec2::Y);
		assert!(combined.pad.pressed(PadButton::TriggerFire));
		let isolated = app
			.world()
			.get::<VirtualPadController>(isolated)
			.ok_or_else(|| anyhow::anyhow!("isolated controller"))?;
		assert_eq!(isolated.pad.move_stick, Vec2::ZERO);
		assert!(!isolated.pad.pressed(PadButton::TriggerFire));
		Ok(())
	}

	#[test]
	fn each_controller_uses_its_own_trigger_threshold() -> anyhow::Result<()> {
		let mut app = app();
		let mut gamepad = Gamepad::default();
		gamepad.analog_mut().set(GamepadButton::RightTrigger2, 0.4);
		let device = app.world_mut().spawn(gamepad).id();
		let config = VirtualPadConfig { trigger_press_threshold: 0.3, ..default() };
		let source = app
			.world_mut()
			.spawn(VirtualPadController::new(PadDevices::gamepad(device)).with_config(config))
			.id();
		app.update();
		let controller = app
			.world()
			.get::<VirtualPadController>(source)
			.ok_or_else(|| anyhow::anyhow!("configured controller"))?;
		assert!(controller.pad.pressed(PadButton::TriggerFire));
		assert_eq!(controller.config.trigger_press_threshold, 0.3);
		Ok(())
	}

	#[test]
	fn default_host_reuses_primary_and_secondary_sources_without_fresh_held_edges(
	) -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			bevy::input::InputPlugin,
			crate::VirtualPadPlugin::default(),
		));
		let first_device = app.world_mut().spawn(Gamepad::default()).id();
		let second_device = app.world_mut().spawn(Gamepad::default()).id();
		app.update();
		let primary = app.world().resource::<PrimaryVirtualPad>().0;
		let secondary = app
			.world_mut()
			.query::<(Entity, &VirtualPadController)>()
			.iter(app.world())
			.find_map(|(entity, controller)| {
				controller.devices.gamepads.contains(&second_device).then_some(entity)
			})
			.ok_or_else(|| anyhow::anyhow!("secondary source"))?;
		app.world_mut().despawn(first_device);
		app.world_mut().despawn(second_device);
		app.update();
		let mut held = Gamepad::default();
		held.digital_mut().press(GamepadButton::South);
		let first_replacement = app.world_mut().spawn(held).id();
		let mut held = Gamepad::default();
		held.digital_mut().press(GamepadButton::South);
		let second_replacement = app.world_mut().spawn(held).id();
		app.update();
		for source in [primary, secondary] {
			let controller = app
				.world()
				.get::<VirtualPadController>(source)
				.ok_or_else(|| anyhow::anyhow!("reused source"))?;
			assert!(controller.devices.gamepads.iter().any(|device| [
				first_replacement,
				second_replacement
			]
			.contains(device)));
			assert!(controller.connected());
			assert!(!controller.pad.just_pressed(PadButton::A));
			assert!(!controller.pad.pressed(PadButton::A));
		}
		Ok(())
	}
}
