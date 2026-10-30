//! Platform HID backends that fill Bevy `Gamepad`.
//!
//! Linux / Windows keep gilrs (via `DefaultPlugins`). Apple replaces gilrs with
//! GameController.framework because IOKit enumerates Xbox pads but does not
//! deliver reports.

use bevy::app::PluginGroupBuilder;
use bevy::prelude::*;

mod gamecontroller_lifecycle;

#[cfg(any(target_os = "macos", target_os = "ios"))]
mod gamecontroller;

/// Select who initializes the controller backend when the virtual pad is installed.
///
/// Insert this resource before adding [`crate::VirtualPadPlugin`] or any plugin
/// that installs it. This is a setup policy, not a runtime switch.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PadHidPolicy {
	/// Install the platform backend. Apple GameController requires the process main thread.
	#[default]
	Platform,
	/// Omit this crate's hardware initialization while the host supplies Bevy input
	/// state or messages, for example in headless tests or an external input driver.
	/// Bevy input processing and virtual-pad sampling/discovery remain active.
	/// This does not disable hardware plugins separately installed by the host.
	External,
}

/// Chain on `DefaultPlugins.set(…)` so Apple builds do not start gilrs.
///
/// ```ignore
/// App::new().add_plugins(DefaultPlugins.set(WindowPlugin { .. }).with_pad_hid())
/// ```
pub trait PadHidPlugins {
	fn with_pad_hid(self) -> PluginGroupBuilder;
}

impl PadHidPlugins for PluginGroupBuilder {
	fn with_pad_hid(self) -> PluginGroupBuilder {
		with_pad_hid(self)
	}
}

/// Disable [`bevy::gilrs::GilrsPlugin`] on Apple. No-op elsewhere.
pub fn with_pad_hid(group: PluginGroupBuilder) -> PluginGroupBuilder {
	#[cfg(any(target_os = "macos", target_os = "ios"))]
	{
		group.disable::<bevy::gilrs::GilrsPlugin>()
	}
	#[cfg(not(any(target_os = "macos", target_os = "ios")))]
	{
		group
	}
}

pub(crate) fn configure_backend(app: &mut App) {
	app.init_resource::<PadHidPolicy>();
	if *app.world().resource::<PadHidPolicy>() == PadHidPolicy::External {
		return;
	}
	#[cfg(any(target_os = "macos", target_os = "ios"))]
	{
		if app.is_plugin_added::<bevy::gilrs::GilrsPlugin>() {
			warn!(
				"GilrsPlugin is still enabled on Apple; Xbox pads may connect without events. \
				 Chain `.with_pad_hid()` on DefaultPlugins."
			);
		}
		app.add_plugins(gamecontroller::GameControllerPlugin);
	}
	#[cfg(not(any(target_os = "macos", target_os = "ios")))]
	{
		let _ = app;
	}
}

#[cfg(test)]
mod policy_tests {
	use super::PadHidPolicy;
	use crate::{PadButton, PrimaryVirtualPad, VirtualPadController, VirtualPadPlugin};
	use bevy::prelude::*;

	#[test]
	fn external_input_keeps_sampling_and_device_discovery() -> anyhow::Result<()> {
		let mut app = App::new();
		app.insert_resource(PadHidPolicy::External).add_plugins((
			MinimalPlugins,
			bevy::input::InputPlugin,
			VirtualPadPlugin::default(),
		));
		#[cfg(any(target_os = "macos", target_os = "ios"))]
		assert!(!app.is_plugin_added::<super::gamecontroller::GameControllerPlugin>());
		let device = app.world_mut().spawn(Gamepad::default()).id();
		app.update();
		app.world_mut()
			.get_mut::<Gamepad>(device)
			.ok_or_else(|| anyhow::anyhow!("injected gamepad"))?
			.digital_mut()
			.press(GamepadButton::South);
		app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::KeyW);
		app.update();
		let primary = app.world().resource::<PrimaryVirtualPad>().0;
		let controller = app
			.world()
			.get::<VirtualPadController>(primary)
			.ok_or_else(|| anyhow::anyhow!("primary controller"))?;
		assert!(controller.devices.gamepads.contains(&device));
		assert_eq!(controller.pad.move_stick, Vec2::Y);
		assert!(controller.pad.just_pressed(PadButton::A));
		Ok(())
	}

	#[test]
	fn default_policy_registers_the_platform_backend() {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, bevy::input::InputPlugin, VirtualPadPlugin::default()));
		assert_eq!(*app.world().resource::<PadHidPolicy>(), PadHidPolicy::Platform);
		#[cfg(any(target_os = "macos", target_os = "ios"))]
		assert!(app.is_plugin_added::<super::gamecontroller::GameControllerPlugin>());
	}
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
#[cfg(test)]
pub(crate) fn value_changed(previous: Option<f32>, value: f32) -> bool {
	const EPS: f32 = 1e-4;
	previous.is_none_or(|old| (old - value).abs() > EPS)
}

#[cfg(any(target_os = "macos", target_os = "ios"))]
#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn first_sample_always_emits() -> anyhow::Result<()> {
		assert!(value_changed(None, 0.0));
		Ok(())
	}

	#[test]
	fn tiny_noise_is_ignored() -> anyhow::Result<()> {
		assert!(!value_changed(Some(0.5), 0.5 + 1e-5));
		Ok(())
	}

	#[test]
	fn stick_deflection_emits() -> anyhow::Result<()> {
		assert!(value_changed(Some(0.0), 0.2));
		Ok(())
	}
}
