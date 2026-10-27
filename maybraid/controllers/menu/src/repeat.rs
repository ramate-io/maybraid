//! Receiver-owned directional repeat.
use bevy::prelude::*;
use maybraid_virtual_pad_controller::{Cardinal, PadButton, VirtualPad};
use menu_components::MenuNav;
/// Menu-owned interpretation settings. Physical pad production does not interpret repeat.
#[derive(Clone, Copy, Debug)]
pub struct MenuRepeatConfig {
	pub stick_threshold: f32,
	pub repeat_delay_secs: f32,
	pub repeat_rate_secs: f32,
}
impl Default for MenuRepeatConfig {
	fn default() -> Self {
		Self { stick_threshold: 0.2, repeat_delay_secs: 0.35, repeat_rate_secs: 0.08 }
	}
}

#[derive(Clone, Debug, Default)]
pub struct MenuRepeat {
	pub events: Vec<MenuNav>,
	held_dir: Option<MenuNav>,
	repeat_clock: f32,
	repeating: bool,
}

impl MenuRepeat {
	pub fn just_pressed(&self, nav: MenuNav) -> bool {
		self.events.contains(&nav)
	}

	pub fn derive(&mut self, pad: &VirtualPad, config: &MenuRepeatConfig, dt: f32) {
		self.events.clear();
		if pad.just_pressed(PadButton::A) || pad.just_pressed(PadButton::Start) {
			self.events.push(MenuNav::Select);
		}
		if pad.just_pressed(PadButton::B) {
			self.events.push(MenuNav::Back);
		}

		let edge_dir = if pad.just_pressed(PadButton::DpadUp) {
			Some(MenuNav::Up)
		} else if pad.just_pressed(PadButton::DpadDown) {
			Some(MenuNav::Down)
		} else if pad.just_pressed(PadButton::DpadLeft) {
			Some(MenuNav::Left)
		} else if pad.just_pressed(PadButton::DpadRight) {
			Some(MenuNav::Right)
		} else {
			None
		};

		let held = Cardinal::from_stick(pad.dpad, config.stick_threshold)
			.or_else(|| Cardinal::from_stick(pad.move_stick, config.stick_threshold))
			.map(MenuNav::from_cardinal);

		if let Some(dir) = edge_dir {
			self.events.push(dir);
			self.held_dir = Some(dir);
			self.repeat_clock = 0.0;
			self.repeating = false;
			return;
		}

		match (held, self.held_dir) {
			(Some(dir), Some(prev)) if dir == prev => {
				self.repeat_clock += dt;
				let interval =
					if self.repeating { config.repeat_rate_secs } else { config.repeat_delay_secs };
				if self.repeat_clock >= interval {
					self.events.push(dir);
					self.repeat_clock = 0.0;
					self.repeating = true;
				}
			}
			(Some(dir), _) => {
				self.events.push(dir);
				self.held_dir = Some(dir);
				self.repeat_clock = 0.0;
				self.repeating = false;
			}
			(None, _) => {
				self.held_dir = None;
				self.repeat_clock = 0.0;
				self.repeating = false;
			}
		}
	}
}
