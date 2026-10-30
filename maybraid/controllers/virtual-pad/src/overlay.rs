//! Optional analog / digital overlay, additive or replacing physical input.
//!
//! [`PadOverlayRequest`] installs or removes injection on one controller entity.

use bevy::prelude::*;

use crate::button::{PadButton, PAD_BUTTON_COUNT};
use crate::pad::VirtualPad;

/// Explicit injection for one controller entity. `None` removes its overlay.
#[derive(Message, Clone, Debug)]
pub struct PadOverlayRequest {
	pub source: Entity,
	pub overlay: Option<PadOverlay>,
}

#[derive(Component, Clone, Debug)]
pub(crate) struct PadOverlayState(pub PadOverlay);

pub(crate) fn apply_overlay_requests(
	mut requests: MessageReader<PadOverlayRequest>,
	controllers: Query<(), With<crate::VirtualPadController>>,
	mut commands: Commands,
) {
	for request in requests.read() {
		if !controllers.contains(request.source) {
			continue;
		}
		if let Some(overlay) = &request.overlay {
			commands.entity(request.source).try_insert(PadOverlayState(overlay.clone()));
		} else {
			commands.entity(request.source).try_remove::<PadOverlayState>();
		}
	}
}

/// How one controller combines injected and physical input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PadOverlayMode {
	/// Combine physical input with the overlay.
	#[default]
	Additive,
	/// Sample only the overlay; ignore this controller's physical input.
	Exclusive,
}

/// Stick / trigger / button injection for this frame.
#[derive(Clone, Debug, PartialEq)]
pub struct PadOverlay {
	pub mode: PadOverlayMode,
	pub move_stick: Vec2,
	pub look_stick: Vec2,
	pub trigger_focus: f32,
	pub trigger_fire: f32,
	pub buttons: [bool; PAD_BUTTON_COUNT],
}

impl Default for PadOverlay {
	fn default() -> Self {
		Self {
			mode: PadOverlayMode::Additive,
			move_stick: Vec2::ZERO,
			look_stick: Vec2::ZERO,
			trigger_focus: 0.0,
			trigger_fire: 0.0,
			buttons: [false; PAD_BUTTON_COUNT],
		}
	}
}

impl PadOverlay {
	pub fn clear(&mut self) {
		*self = Self { mode: self.mode, ..Self::default() };
	}

	pub fn hold(&mut self, button: PadButton) {
		self.buttons[button.index()] = true;
	}

	pub fn apply(&self, pad: &mut VirtualPad) {
		pad.add_move(self.move_stick);
		pad.add_look(self.look_stick);
		pad.max_triggers(self.trigger_focus, self.trigger_fire);
		for button in PadButton::ALL {
			if self.buttons[button.index()] {
				pad.hold_digital(button);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn overlay_unions_with_existing_stick() -> anyhow::Result<()> {
		let mut pad = VirtualPad::default();
		pad.begin_frame();
		pad.add_move(Vec2::new(0.2, 0.0));
		let mut overlay = PadOverlay::default();
		overlay.move_stick = Vec2::new(0.0, 1.0);
		overlay.hold(PadButton::A);
		overlay.apply(&mut pad);
		pad.finish_digital();
		assert!((pad.move_stick - Vec2::new(0.2, 1.0)).length() < 1e-5);
		assert!(pad.just_pressed(PadButton::A));
		Ok(())
	}

	#[test]
	fn clear_preserves_exclusive_mode() {
		let mut overlay =
			PadOverlay { mode: PadOverlayMode::Exclusive, move_stick: Vec2::Y, ..default() };
		overlay.hold(PadButton::A);
		overlay.clear();
		assert_eq!(overlay, PadOverlay { mode: PadOverlayMode::Exclusive, ..default() });
	}

	#[test]
	fn exclusive_overlay_is_addressed_and_removal_restores_physical_input() -> anyhow::Result<()> {
		use crate::{
			ButtonPhase, PadDevices, PrimaryVirtualPad, VirtualPadController, VirtualPadPlugin,
		};
		use bevy::input::mouse::MouseMotion;
		use bevy::time::TimeUpdateStrategy;

		let mut app = App::new();
		app.insert_resource(crate::PadHidPolicy::External)
			.insert_resource(TimeUpdateStrategy::ManualDuration(
				std::time::Duration::from_secs_f32(1.0 / 60.0),
			))
			.add_plugins((
				MinimalPlugins,
				bevy::input::InputPlugin,
				VirtualPadPlugin { auto_assign_devices: false, ..default() },
			));
		let device = app.world_mut().spawn(Gamepad::default()).id();
		let devices = PadDevices::keyboard_mouse().with_gamepad(device);
		let primary = app.world().resource::<PrimaryVirtualPad>().0;
		app.world_mut()
			.get_mut::<VirtualPadController>(primary)
			.ok_or_else(|| anyhow::anyhow!("primary controller"))?
			.devices = devices.clone();
		let other = app.world_mut().spawn(VirtualPadController::new(devices.clone())).id();
		app.update();
		{
			let mut gamepad = app
				.world_mut()
				.get_mut::<Gamepad>(device)
				.ok_or_else(|| anyhow::anyhow!("physical gamepad"))?;
			gamepad.analog_mut().set(GamepadAxis::LeftStickX, 1.0);
			gamepad.analog_mut().set(GamepadAxis::RightStickX, 1.0);
			gamepad.analog_mut().set(GamepadButton::RightTrigger2, 1.0);
			gamepad.digital_mut().press(GamepadButton::East);
		}
		app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyW);
		app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Space);
		app.world_mut()
			.resource_mut::<ButtonInput<MouseButton>>()
			.press(MouseButton::Left);
		app.world_mut()
			.resource_mut::<ButtonInput<MouseButton>>()
			.press(MouseButton::Right);
		app.world_mut().write_message(MouseMotion { delta: Vec2::new(9.0, -5.0) });
		let mut overlay = PadOverlay {
			mode: PadOverlayMode::Exclusive,
			move_stick: Vec2::new(-0.4, -0.2),
			look_stick: Vec2::new(2.0, 3.0),
			trigger_focus: 0.2,
			trigger_fire: 0.3,
			..default()
		};
		overlay.hold(PadButton::X);
		app.world_mut()
			.write_message(PadOverlayRequest { source: primary, overlay: Some(overlay.clone()) });
		app.update();
		{
			let controller = app
				.world()
				.get::<VirtualPadController>(primary)
				.ok_or_else(|| anyhow::anyhow!("exclusive controller"))?;
			assert_eq!(controller.devices, devices);
			assert_eq!(controller.pad.move_stick, overlay.move_stick);
			assert_eq!(controller.pad.look_stick, overlay.look_stick);
			assert_eq!(controller.pad.look_throw, Vec2::ZERO);
			assert_eq!(controller.pad.trigger_focus, 0.2);
			assert_eq!(controller.pad.trigger_fire, 0.3);
			assert!(!controller.pad.keys.pressed(KeyCode::KeyW));
			assert!(!controller.pad.pressed(PadButton::A));
			assert!(!controller.pad.pressed(PadButton::B));
			assert!(controller.pad.just_pressed(PadButton::X));
			assert!(controller.history.recent_button(PadButton::X, ButtonPhase::Pressed));
			let physical = app
				.world()
				.get::<VirtualPadController>(other)
				.ok_or_else(|| anyhow::anyhow!("other controller"))?;
			assert!(physical.pad.move_stick.x > 0.0 && physical.pad.move_stick.y > 0.0);
			assert!(physical.pad.look_stick.x > 9.0);
			assert_eq!(physical.pad.trigger_focus, 1.0);
			assert_eq!(physical.pad.trigger_fire, 1.0);
			assert!(physical.pad.keys.pressed(KeyCode::KeyW));
			assert!(physical.pad.pressed(PadButton::A));
			assert!(physical.pad.pressed(PadButton::B));
			assert!(!physical.pad.pressed(PadButton::X));
		}
		app.update();
		let held = app
			.world()
			.get::<VirtualPadController>(primary)
			.ok_or_else(|| anyhow::anyhow!("held controller"))?;
		assert!(held.pad.pressed(PadButton::X));
		assert!(!held.pad.just_pressed(PadButton::X));
		let sequence = held.sequence();
		app.world_mut()
			.write_message(PadOverlayRequest { source: primary, overlay: None });
		app.update();
		let restored = app
			.world()
			.get::<VirtualPadController>(primary)
			.ok_or_else(|| anyhow::anyhow!("restored controller"))?;
		assert_eq!(restored.sequence(), sequence + 1);
		assert!(restored.pad.move_stick.x > 0.0 && restored.pad.move_stick.y > 0.0);
		assert!(restored.pad.look_throw.x > 0.0);
		assert!(restored.pad.pressed(PadButton::A));
		assert!(restored.pad.pressed(PadButton::B));
		assert!(restored.pad.just_released(PadButton::X));
		assert!(restored.history.recent_button(PadButton::X, ButtonPhase::Released));
		overlay.mode = PadOverlayMode::Additive;
		app.world_mut()
			.write_message(PadOverlayRequest { source: primary, overlay: Some(overlay) });
		app.update();
		let additive = app
			.world()
			.get::<VirtualPadController>(primary)
			.ok_or_else(|| anyhow::anyhow!("additive controller"))?;
		assert!(additive.pad.pressed(PadButton::A));
		assert!(additive.pad.pressed(PadButton::B));
		assert!(additive.pad.just_pressed(PadButton::X));
		assert_eq!(additive.pad.trigger_fire, 1.0);
		Ok(())
	}

	#[test]
	fn missing_overlay_is_a_noop() -> anyhow::Result<()> {
		use crate::{PrimaryVirtualPad, VirtualPadController, VirtualPadPlugin};
		let mut app = App::new();
		app.insert_resource(crate::PadHidPolicy::External);
		app.add_plugins((MinimalPlugins, bevy::input::InputPlugin, VirtualPadPlugin::default()));
		let primary = app.world().resource::<PrimaryVirtualPad>().0;
		app.update();
		assert!(app.world().get::<PadOverlayState>(primary).is_none());
		let controller = app
			.world()
			.get::<VirtualPadController>(primary)
			.ok_or_else(|| anyhow::anyhow!("primary controller"))?;
		assert_eq!(controller.pad.trigger_fire, 0.0);
		assert!(controller.pad.button_events.is_empty());
		Ok(())
	}

	#[test]
	fn device_transfer_does_not_swallow_exclusive_script_buttons() -> anyhow::Result<()> {
		use crate::{PadDevices, PrimaryVirtualPad, VirtualPadController, VirtualPadPlugin};

		let mut app = App::new();
		app.insert_resource(crate::PadHidPolicy::External).add_plugins((
			MinimalPlugins,
			bevy::input::InputPlugin,
			VirtualPadPlugin { auto_assign_devices: false, ..default() },
		));
		let primary = app.world().resource::<PrimaryVirtualPad>().0;
		let mut overlay = PadOverlay { mode: PadOverlayMode::Exclusive, ..default() };
		overlay.hold(PadButton::X);
		app.world_mut()
			.write_message(PadOverlayRequest { source: primary, overlay: Some(overlay.clone()) });
		app.update();
		app.world_mut()
			.get_mut::<VirtualPadController>(primary)
			.ok_or_else(|| anyhow::anyhow!("primary controller"))?
			.devices = PadDevices::default();
		overlay.hold(PadButton::Start);
		app.world_mut()
			.write_message(PadOverlayRequest { source: primary, overlay: Some(overlay) });
		app.update();
		let controller = app
			.world()
			.get::<VirtualPadController>(primary)
			.ok_or_else(|| anyhow::anyhow!("exclusive controller"))?;
		assert!(controller.pad.pressed(PadButton::X));
		assert!(!controller.pad.just_pressed(PadButton::X));
		assert!(controller.pad.just_pressed(PadButton::Start));
		Ok(())
	}

	#[test]
	fn addressed_requests_inject_and_remove_only_the_named_controller() -> anyhow::Result<()> {
		use crate::{PadDevices, PrimaryVirtualPad, VirtualPadController, VirtualPadPlugin};
		let mut app = App::new();
		app.insert_resource(crate::PadHidPolicy::External);
		app.add_plugins((
			MinimalPlugins,
			bevy::input::InputPlugin,
			VirtualPadPlugin { auto_assign_devices: false, ..default() },
		));
		let primary = app.world().resource::<PrimaryVirtualPad>().0;
		let other = app.world_mut().spawn(VirtualPadController::new(PadDevices::default())).id();
		let primary_overlay = PadOverlay { trigger_fire: 0.9, ..default() };
		let mut other_overlay = PadOverlay::default();
		other_overlay.hold(PadButton::A);
		app.world_mut()
			.write_message(PadOverlayRequest { source: primary, overlay: Some(primary_overlay) });
		app.world_mut()
			.write_message(PadOverlayRequest { source: other, overlay: Some(other_overlay) });
		app.update();
		let primary_pad = app
			.world()
			.get::<VirtualPadController>(primary)
			.ok_or_else(|| anyhow::anyhow!("primary controller"))?;
		let other_pad = app
			.world()
			.get::<VirtualPadController>(other)
			.ok_or_else(|| anyhow::anyhow!("other controller"))?;
		assert!(!primary_pad.pad.pressed(PadButton::A));
		assert_eq!(primary_pad.pad.trigger_fire, 0.9);
		assert!(other_pad.pad.pressed(PadButton::A));
		assert_eq!(other_pad.pad.trigger_fire, 0.0);
		app.world_mut()
			.write_message(PadOverlayRequest { source: other, overlay: None });
		app.update();
		assert!(app.world().get::<PadOverlayState>(other).is_none());
		assert_eq!(
			app.world()
				.get::<VirtualPadController>(primary)
				.ok_or_else(|| anyhow::anyhow!("primary controller"))?
				.pad
				.trigger_fire,
			0.9
		);
		Ok(())
	}
}
