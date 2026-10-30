//! Optional analog / digital overlay applied after HID producers.
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

/// Extra stick / trigger / button contribution for this frame.
#[derive(Clone, Debug, PartialEq)]
pub struct PadOverlay {
	pub move_stick: Vec2,
	pub look_stick: Vec2,
	pub trigger_focus: f32,
	pub trigger_fire: f32,
	pub buttons: [bool; PAD_BUTTON_COUNT],
}

impl Default for PadOverlay {
	fn default() -> Self {
		Self {
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
		*self = Self::default();
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
