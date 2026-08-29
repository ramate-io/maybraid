//! Tunables for produce and derived surfaces.

use std::time::Duration;

use bevy::prelude::*;

use crate::analog::Deadzone;

#[derive(Resource, Clone, Debug)]
pub struct VirtualPadConfig {
	pub stick_deadzone: Deadzone,
	pub trigger_press_threshold: f32,
	/// Gamepad look is converted into mouse-delta units (`pixels/sec * dt`).
	pub gamepad_look_pixels_per_sec: f32,
	pub menu_stick_threshold: f32,
	pub menu_repeat_delay_secs: f32,
	pub menu_repeat_rate_secs: f32,
	pub history_window: Duration,
	pub history_max_frames: usize,
	pub cursor_speed: f32,
}

impl Default for VirtualPadConfig {
	fn default() -> Self {
		Self {
			stick_deadzone: Deadzone(0.15),
			trigger_press_threshold: 0.5,
			gamepad_look_pixels_per_sec: 1_200.0,
			menu_stick_threshold: 0.5,
			menu_repeat_delay_secs: 0.35,
			menu_repeat_rate_secs: 0.08,
			history_window: Duration::from_millis(500),
			history_max_frames: 64,
			cursor_speed: 800.0,
		}
	}
}
