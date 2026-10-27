//! Ring of recent pad snapshots and digital edges.

use std::collections::VecDeque;
use std::time::Duration;

use bevy::prelude::*;

use crate::button::{ButtonStroke, PadButton};
use crate::config::VirtualPadConfig;
use crate::pad::VirtualPad;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PadSnapshot {
	pub move_stick: Vec2,
	pub look_stick: Vec2,
	pub trigger_focus: f32,
	pub trigger_fire: f32,
	pub dpad: Vec2,
	pub buttons_down: u16,
}

impl PadSnapshot {
	pub fn button_down(&self, button: PadButton) -> bool {
		self.buttons_down & (1 << button.index()) != 0
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PadEdge {
	Button(ButtonStroke<PadButton>),
	Key(ButtonStroke<KeyCode>),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Timed<T> {
	pub elapsed_secs: f32,
	pub value: T,
}

#[derive(Resource, Clone, Debug, Default)]
pub struct PadHistory {
	pub frames: VecDeque<Timed<PadSnapshot>>,
	pub edges: VecDeque<Timed<PadEdge>>,
}

impl PadHistory {
	pub fn push_frame(&mut self, elapsed_secs: f32, snapshot: PadSnapshot) {
		self.frames.push_back(Timed { elapsed_secs, value: snapshot });
	}

	pub fn push_edge(&mut self, elapsed_secs: f32, edge: PadEdge) {
		self.edges.push_back(Timed { elapsed_secs, value: edge });
	}

	pub fn evict(&mut self, now: f32, window: Duration, max_frames: usize) {
		let keep_after = now - window.as_secs_f32();
		while self.frames.front().is_some_and(|frame| frame.elapsed_secs < keep_after) {
			self.frames.pop_front();
		}
		while self.edges.front().is_some_and(|edge| edge.elapsed_secs < keep_after) {
			self.edges.pop_front();
		}
		while self.frames.len() > max_frames {
			self.frames.pop_front();
		}
	}

	pub fn recent_button(&self, button: PadButton, phase: crate::button::ButtonPhase) -> bool {
		self.edges.iter().any(|edge| {
			matches!(
				edge.value,
				PadEdge::Button(stroke) if stroke.button == button && stroke.phase == phase
			)
		})
	}
}

pub fn push_history(
	pad: Res<VirtualPad>,
	config: Res<VirtualPadConfig>,
	time: Res<Time>,
	mut history: ResMut<PadHistory>,
) {
	let now = time.elapsed_secs();
	history.push_frame(now, pad.snapshot());
	for stroke in &pad.button_events {
		history.push_edge(now, PadEdge::Button(*stroke));
	}
	for stroke in &pad.key_events {
		history.push_edge(now, PadEdge::Key(*stroke));
	}
	history.evict(now, config.history_window, config.history_max_frames);
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::button::ButtonPhase;

	#[test]
	fn evict_drops_old_frames() -> anyhow::Result<()> {
		let mut history = PadHistory::default();
		history.push_frame(
			0.0,
			PadSnapshot {
				move_stick: Vec2::ZERO,
				look_stick: Vec2::ZERO,
				trigger_focus: 0.0,
				trigger_fire: 0.0,
				dpad: Vec2::ZERO,
				buttons_down: 0,
			},
		);
		history.push_frame(
			1.0,
			PadSnapshot {
				move_stick: Vec2::X,
				look_stick: Vec2::ZERO,
				trigger_focus: 0.0,
				trigger_fire: 0.0,
				dpad: Vec2::ZERO,
				buttons_down: 0,
			},
		);
		history.evict(1.0, Duration::from_millis(400), 64);
		assert_eq!(history.frames.len(), 1);
		assert_eq!(history.frames[0].value.move_stick, Vec2::X);
		Ok(())
	}

	#[test]
	fn recent_button_reads_edges() -> anyhow::Result<()> {
		let mut history = PadHistory::default();
		history.push_edge(0.1, PadEdge::Button(ButtonStroke::pressed(PadButton::A)));
		assert!(history.recent_button(PadButton::A, ButtonPhase::Pressed));
		assert!(!history.recent_button(PadButton::B, ButtonPhase::Pressed));
		Ok(())
	}
}
