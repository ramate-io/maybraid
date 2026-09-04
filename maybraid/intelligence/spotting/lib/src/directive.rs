use crate::{InterestLayers, SpottedContact};

/// Read-only semantic and spatial context for testing a remembered contact.
#[derive(Clone, Copy, Debug)]
pub struct SpotContactView<'a> {
	pub contact: &'a SpottedContact,
	pub layers: InterestLayers,
	pub distance: f32,
}

/// One category and cadence of observation requested by a [`crate::SpottingUser`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpotDirective {
	pub layers: InterestLayers,
	pub range: f32,
	pub priority: i32,
	pub desired_count: usize,
	pub freshness_secs: f32,
	pub discovery_interval_secs: f32,
	pub respot_interval_secs: f32,
	pub max_samples_per_subject: usize,
}

impl SpotDirective {
	pub fn new(layers: InterestLayers, range: f32) -> Self {
		Self { layers, range: range.max(0.0), ..Self::default() }
	}

	pub fn with_priority(mut self, priority: i32) -> Self {
		self.priority = priority;
		self
	}

	pub fn with_desired_count(mut self, desired_count: usize) -> Self {
		self.desired_count = desired_count;
		self
	}

	pub fn matches(self, layers: InterestLayers, distance: f32) -> bool {
		self.layers.intersects(layers) && distance <= self.range.max(0.0)
	}

	pub fn fresh_match_count<'a>(
		self,
		now: f32,
		contacts: impl IntoIterator<Item = SpotContactView<'a>>,
	) -> usize {
		contacts
			.into_iter()
			.filter(|view| {
				self.matches(view.layers, view.distance)
					&& view.contact.is_fresh(now, self.freshness_secs)
			})
			.count()
	}

	pub fn is_satisfied<'a>(
		self,
		now: f32,
		contacts: impl IntoIterator<Item = SpotContactView<'a>>,
	) -> bool {
		self.fresh_match_count(now, contacts) >= self.desired_count
	}
}

impl Default for SpotDirective {
	fn default() -> Self {
		Self {
			layers: InterestLayers::CHARACTER,
			range: 40.0,
			priority: 0,
			desired_count: 1,
			freshness_secs: 0.35,
			discovery_interval_secs: 0.2,
			respot_interval_secs: 0.1,
			max_samples_per_subject: 9,
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;

	use super::*;

	fn contact(subject: Entity, at: f32) -> SpottedContact {
		SpottedContact::new(subject, Vec3::ZERO, Vec3::ZERO, Vec3::ZERO, None, at, 0.1)
	}

	#[test]
	fn directive_satisfaction_counts_only_fresh_matching_contacts() -> anyhow::Result<()> {
		let directive = SpotDirective::new(InterestLayers::CHARACTER, 10.0).with_desired_count(2);
		let a = contact(Entity::from_bits(1), 1.0);
		let b = contact(Entity::from_bits(2), 1.0);
		let c = contact(Entity::from_bits(3), 0.0);
		let views = [
			SpotContactView { contact: &a, layers: InterestLayers::CHARACTER, distance: 5.0 },
			SpotContactView { contact: &b, layers: InterestLayers::WEAPON, distance: 5.0 },
			SpotContactView { contact: &c, layers: InterestLayers::CHARACTER, distance: 5.0 },
		];
		assert!(!directive.is_satisfied(1.2, views));

		let matching = [
			views[0],
			SpotContactView { contact: &b, layers: InterestLayers::CHARACTER, distance: 5.0 },
		];
		assert!(directive.is_satisfied(1.2, matching));
		Ok(())
	}
}
