use bevy::prelude::*;

/// Last visible state remembered for a combat subject.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CombatContact {
	pub subject: Entity,
	pub position: Vec3,
	pub movement_vector: Vec3,
	/// A point that was visibly reachable when this contact was observed.
	pub visible_point: Vec3,
	/// A visible head point, when perception could establish one.
	pub visible_head: Option<Vec3>,
	/// [`Time::elapsed_secs`] when this contact was observed.
	pub last_spotted_at: f32,
}

impl CombatContact {
	/// Chooses the visible head when requested and available, otherwise the
	/// general visible point.
	pub fn aim_point(self, prefer_head: bool) -> Vec3 {
		if prefer_head {
			self.visible_head.unwrap_or(self.visible_point)
		} else {
			self.visible_point
		}
	}

	/// Returns linearly normalized freshness in `0.0..=1.0`.
	pub fn freshness(self, now: f32, memory_secs: f32) -> f32 {
		let age = (now - self.last_spotted_at).max(0.0);
		let window = memory_secs.max(0.0);
		if window == 0.0 {
			return if age == 0.0 { 1.0 } else { 0.0 };
		}
		(1.0 - age / window).clamp(0.0, 1.0)
	}

	pub fn is_fresh(self, now: f32, memory_secs: f32) -> bool {
		now - self.last_spotted_at <= memory_secs.max(0.0)
	}
}
