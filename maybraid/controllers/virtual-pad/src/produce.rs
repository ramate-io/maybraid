//! Device producers. Run after Bevy [`InputSystems`].

use bevy::input::InputSystems;
use bevy::prelude::*;

pub mod gamepad;
pub mod keyboard;
pub mod mouse;

/// Shared schedule: Bevy HID → produce → derive.
pub fn configure_produce(app: &mut App) {
	app.add_systems(
		PreUpdate,
		(crate::overlay::apply_overlay_requests, crate::controller::produce_controllers)
			.chain()
			.in_set(crate::VirtualPadSystems::Produce)
			.after(InputSystems),
	);
}
