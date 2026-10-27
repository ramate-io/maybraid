//! Shared virtual pad for Maybraid: device producers, current-frame snapshot,
//! optional history, and cursor integration.

pub mod analog;
pub mod button;
pub mod config;
pub mod controller;
pub mod debug;
pub mod gate;
pub mod hid;
pub mod history;
pub mod overlay;
pub mod pad;
pub mod produce;
pub mod rumble;
pub mod surface;

pub use analog::{Cardinal, Deadzone};
pub use button::{ButtonPhase, ButtonStroke, PAD_BUTTON_COUNT, PadButton};
pub use config::VirtualPadConfig;
pub use controller::{
	PadDevices, PrimaryVirtualPad, VirtualPadController, VirtualPadEvent, VirtualPadLifecycle,
};
pub use gate::PadInputPolicy;
pub use hid::{PadHidPlugins, with_pad_hid};
pub use history::{PadEdge, PadHistory, PadSnapshot, Timed};
pub use overlay::PadOverlay;
pub use pad::VirtualPad;
pub use rumble::{PadRumble, PadRumbleSystems};
pub use surface::cursor::PadCursor;

use bevy::input::InputSystems;
use bevy::prelude::*;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VirtualPadSystems {
	Produce,
	Derive,
}

pub struct VirtualPadPlugin {
	pub config: VirtualPadConfig,
	/// Enable the default single-player device assignment host.
	pub auto_assign_devices: bool,
}

impl Default for VirtualPadPlugin {
	fn default() -> Self {
		Self { config: VirtualPadConfig::default(), auto_assign_devices: true }
	}
}

impl VirtualPadPlugin {
	pub fn new(config: VirtualPadConfig) -> Self {
		Self { config, ..Self::default() }
	}
}

impl Plugin for VirtualPadPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			PostUpdate,
			rumble::PadRumbleSystems::Play.after(rumble::PadRumbleSystems::FanOut),
		);
		#[cfg(not(any(target_os = "macos", target_os = "ios")))]
		{
			app.configure_sets(
				PostUpdate,
				bevy::gilrs::RumbleSystems.after(rumble::PadRumbleSystems::FanOut),
			);
		}
		hid::configure_backend(app);
		app.insert_resource(self.config.clone())
			.init_resource::<VirtualPad>()
			.init_resource::<PadHistory>()
			.init_resource::<PadInputPolicy>()
			.init_resource::<PadCursor>()
			.add_message::<rumble::PadRumble>()
			.init_resource::<rumble::DelayedPadRumble>()
			.configure_sets(
				PreUpdate,
				VirtualPadSystems::Produce.after(InputSystems).before(VirtualPadSystems::Derive),
			)
			.configure_sets(PreUpdate, VirtualPadSystems::Derive.after(VirtualPadSystems::Produce));
		let primary = app
			.world_mut()
			.spawn(
				VirtualPadController::new(PadDevices::keyboard_mouse())
					.with_config(self.config.clone()),
			)
			.id();
		app.insert_resource(PrimaryVirtualPad(primary))
			.init_resource::<controller::DefaultDeviceAssignments>()
			.add_message::<VirtualPadEvent>()
			.add_message::<VirtualPadLifecycle>();
		if self.auto_assign_devices {
			app.add_systems(
				PreUpdate,
				controller::discover_devices
					.in_set(VirtualPadSystems::Produce)
					.before(controller::produce_controllers),
			);
		}
		produce::configure_produce(app);
		debug::configure_debug(app);
		app.add_systems(
			PreUpdate,
			controller::publish_controllers.in_set(VirtualPadSystems::Derive),
		);
		app.add_systems(
			PostUpdate,
			rumble::fan_out_pad_rumble.in_set(rumble::PadRumbleSystems::FanOut),
		);
	}
}
