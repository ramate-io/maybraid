use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::{SpotDirective, SpottedContact};

/// Per-user caps and contact retention time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpottingSettings {
	pub candidate_budget: usize,
	pub vision_samples: usize,
	pub memory_secs: f32,
}

impl SpottingSettings {
	pub fn new(candidate_budget: usize, vision_samples: usize, memory_secs: f32) -> Self {
		Self { candidate_budget, vision_samples, memory_secs: memory_secs.max(0.0) }
	}
}

impl Default for SpottingSettings {
	fn default() -> Self {
		Self { candidate_budget: 32, vision_samples: 48, memory_secs: 5.0 }
	}
}

/// Observation policy and remembered contacts for one entity.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct SpottingUser {
	/// Eye position relative to the user's local transform.
	pub eye_offset: Vec3,
	pub directives: Vec<SpotDirective>,
	pub contacts: BTreeMap<Entity, SpottedContact>,
	pub settings: SpottingSettings,
	/// Earliest elapsed time at which broadphase discovery may run again.
	pub next_discovery_at: f32,
	sample_cursor: usize,
}

impl SpottingUser {
	pub fn new(eye_offset: Vec3, directives: impl IntoIterator<Item = SpotDirective>) -> Self {
		Self { eye_offset, directives: directives.into_iter().collect(), ..Self::default() }
	}

	pub fn with_settings(mut self, settings: SpottingSettings) -> Self {
		self.settings = settings;
		self
	}

	pub fn forget_stale(&mut self, now: f32) {
		let memory_secs = self.settings.memory_secs;
		self.contacts.retain(|_, contact| !contact.should_forget(now, memory_secs));
	}

	/// Advance the feature offset used by the next discovery pass.
	pub fn advance_sample_cursor(&mut self) -> usize {
		let current = self.sample_cursor;
		self.sample_cursor = self.sample_cursor.wrapping_add(1);
		current
	}
}

impl Default for SpottingUser {
	fn default() -> Self {
		Self {
			eye_offset: Vec3::Y * 1.6,
			directives: Vec::new(),
			contacts: BTreeMap::new(),
			settings: SpottingSettings::default(),
			next_discovery_at: 0.0,
			sample_cursor: 0,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn user_forgets_only_contacts_beyond_memory() -> anyhow::Result<()> {
		let keep = Entity::from_bits(1);
		let forget = Entity::from_bits(2);
		let mut user = SpottingUser::default().with_settings(SpottingSettings::new(4, 8, 2.0));
		user.contacts.insert(
			keep,
			SpottedContact::new(keep, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, None, 1.0, 0.1),
		);
		user.contacts.insert(
			forget,
			SpottedContact::new(forget, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, None, 0.9, 0.1),
		);
		user.forget_stale(3.0);
		assert!(user.contacts.contains_key(&keep));
		assert!(!user.contacts.contains_key(&forget));
		Ok(())
	}

	#[test]
	fn discovery_sample_cursor_advances_one_feature_per_pass() {
		let mut user = SpottingUser::default();
		assert_eq!(user.advance_sample_cursor(), 0);
		assert_eq!(user.advance_sample_cursor(), 1);
		assert_eq!(user.advance_sample_cursor(), 2);
	}
}
