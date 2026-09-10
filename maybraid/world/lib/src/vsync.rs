//! Present-mode (vsync) toggle for Tracy / frame-time flights.
//!
//! Default stays [`PresentMode::AutoVsync`]. `F8`, `/stats vsync`, or
//! `MAYBRAID_VSYNC=off` switches to [`PresentMode::Immediate`] so the Frame
//! plot is not locked to 16.6 / 33.3 ms.

use bevy::prelude::*;
use bevy::window::{PresentMode, PrimaryWindow};
use game_commands::ui::GameCommandStatusText;

const ENV_VSYNC: &str = "MAYBRAID_VSYNC";

/// Toggle vsync (`/stats vsync` or [`VSYNC_TOGGLE_KEY`]).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestVsyncToggle;

pub const VSYNC_TOGGLE_KEY: KeyCode = KeyCode::F8;

pub fn default_window_present_mode() -> PresentMode {
	parse_vsync_env(std::env::var(ENV_VSYNC).ok().as_deref().unwrap_or(""))
		.unwrap_or(PresentMode::AutoVsync)
}

pub(crate) fn parse_vsync_env(raw: &str) -> Option<PresentMode> {
	let raw = raw.trim();
	if raw.is_empty() {
		return None;
	}
	match raw.to_ascii_lowercase().as_str() {
		"off" | "0" | "immediate" | "no" | "false" => Some(PresentMode::Immediate),
		"on" | "1" | "vsync" | "auto" | "true" => Some(PresentMode::AutoVsync),
		other => {
			eprintln!("[{ENV_VSYNC}] unknown {other:?} (use on|off)");
			None
		}
	}
}

fn present_mode_label(mode: PresentMode) -> &'static str {
	match mode {
		PresentMode::Immediate => "off (Immediate)",
		PresentMode::AutoVsync => "on (AutoVsync)",
		PresentMode::AutoNoVsync => "off (AutoNoVsync)",
		PresentMode::Fifo => "on (Fifo)",
		PresentMode::FifoRelaxed => "on (FifoRelaxed)",
		PresentMode::Mailbox => "off (Mailbox)",
	}
}

fn toggle_present_mode(mode: PresentMode) -> PresentMode {
	match mode {
		PresentMode::Immediate | PresentMode::AutoNoVsync | PresentMode::Mailbox => {
			PresentMode::AutoVsync
		}
		_ => PresentMode::Immediate,
	}
}

pub(crate) fn apply_startup_vsync(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
	let Ok(mut window) = windows.single_mut() else {
		return;
	};
	if let Some(mode) = parse_vsync_env(std::env::var(ENV_VSYNC).ok().as_deref().unwrap_or("")) {
		window.present_mode = mode;
		info!("{ENV_VSYNC}={}", present_mode_label(mode));
	}
}

pub(crate) fn toggle_vsync(
	mut commands: Commands,
	mut windows: Query<&mut Window, With<PrimaryWindow>>,
	keyboard: Res<ButtonInput<KeyCode>>,
	requests: Query<Entity, With<RequestVsyncToggle>>,
	mut status: Option<ResMut<GameCommandStatusText>>,
	focus: Option<Res<game_commands::command::TextEntryFocus>>,
) {
	let key = keyboard.just_pressed(VSYNC_TOGGLE_KEY) && !focus.is_some_and(|focus| focus.0);
	let requested: Vec<_> = requests.iter().collect();
	if !key && requested.is_empty() {
		return;
	}
	for entity in requested {
		commands.entity(entity).despawn();
	}
	let Ok(mut window) = windows.single_mut() else {
		return;
	};
	window.present_mode = toggle_present_mode(window.present_mode);
	let line = format!("vsync {}", present_mode_label(window.present_mode));
	if let Some(status) = status.as_mut() {
		status.0 = line.clone();
	}
	info!("{line}");
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn parse_off_and_on() {
		assert_eq!(parse_vsync_env("off"), Some(PresentMode::Immediate));
		assert_eq!(parse_vsync_env("ON"), Some(PresentMode::AutoVsync));
		assert_eq!(parse_vsync_env(""), None);
	}

	#[test]
	fn toggle_swaps_vsync_and_immediate() {
		assert_eq!(toggle_present_mode(PresentMode::AutoVsync), PresentMode::Immediate);
		assert_eq!(toggle_present_mode(PresentMode::Immediate), PresentMode::AutoVsync);
	}
}
