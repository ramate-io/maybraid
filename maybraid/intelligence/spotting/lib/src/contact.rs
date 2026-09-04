use bevy::prelude::*;

/// Last successful observation and retry state for one subject.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpottedContact {
	pub subject: Entity,
	/// Exact subject transform translation at the latest successful probe.
	pub position: Vec3,
	/// Exact subject velocity at the latest successful probe.
	pub velocity: Vec3,
	/// A point whose line of sight was clear at the latest successful probe.
	pub visible_point: Vec3,
	/// A clear head sample, when the latest probe found one.
	pub visible_head: Option<Vec3>,
	pub last_success_at: f32,
	pub last_attempt_at: f32,
	pub consecutive_failures: u32,
	pub next_respot_at: f32,
}

impl SpottedContact {
	#[allow(clippy::too_many_arguments)]
	pub fn new(
		subject: Entity,
		position: Vec3,
		velocity: Vec3,
		visible_point: Vec3,
		visible_head: Option<Vec3>,
		now: f32,
		respot_interval_secs: f32,
	) -> Self {
		Self {
			subject,
			position,
			velocity,
			visible_point,
			visible_head,
			last_success_at: now,
			last_attempt_at: now,
			consecutive_failures: 0,
			next_respot_at: now + respot_interval_secs.max(0.0),
		}
	}

	pub fn is_fresh(self, now: f32, freshness_secs: f32) -> bool {
		now - self.last_success_at <= freshness_secs.max(0.0)
	}

	pub fn should_forget(self, now: f32, memory_secs: f32) -> bool {
		now - self.last_success_at > memory_secs.max(0.0)
	}

	pub fn is_due(self, now: f32) -> bool {
		now >= self.next_respot_at
	}

	pub fn aim_point(self, prefer_head: bool) -> Vec3 {
		if prefer_head {
			self.visible_head.unwrap_or(self.visible_point)
		} else {
			self.visible_point
		}
	}

	/// Translate the last clear point using constant-velocity extrapolation.
	pub fn predicted_aim_point(self, now: f32, prefer_head: bool) -> Vec3 {
		let elapsed = (now - self.last_success_at).max(0.0);
		self.aim_point(prefer_head) + self.velocity * elapsed
	}

	#[allow(clippy::too_many_arguments)]
	pub fn note_success(
		&mut self,
		position: Vec3,
		velocity: Vec3,
		visible_point: Vec3,
		visible_head: Option<Vec3>,
		now: f32,
		respot_interval_secs: f32,
	) {
		self.position = position;
		self.velocity = velocity;
		self.visible_point = visible_point;
		self.visible_head = visible_head;
		self.last_success_at = now;
		self.last_attempt_at = now;
		self.consecutive_failures = 0;
		self.next_respot_at = now + respot_interval_secs.max(0.0);
	}

	pub fn note_failure(&mut self, now: f32, respot_interval_secs: f32) {
		self.last_attempt_at = now;
		self.consecutive_failures = self.consecutive_failures.saturating_add(1);
		self.next_respot_at = now + respot_interval_secs.max(0.0);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn contact() -> SpottedContact {
		SpottedContact::new(Entity::from_bits(1), Vec3::ZERO, Vec3::X, Vec3::Y, None, 2.0, 0.25)
	}

	#[test]
	fn freshness_and_forgetting_use_last_success() -> anyhow::Result<()> {
		let contact = contact();
		assert!(contact.is_fresh(2.5, 0.5));
		assert!(!contact.is_fresh(2.51, 0.5));
		assert!(!contact.should_forget(5.0, 3.0));
		assert!(contact.should_forget(5.01, 3.0));
		Ok(())
	}

	#[test]
	fn predicted_aim_uses_latest_velocity() -> anyhow::Result<()> {
		assert_eq!(contact().predicted_aim_point(3.5, false), Vec3::new(1.5, 1.0, 0.0));
		Ok(())
	}
}
