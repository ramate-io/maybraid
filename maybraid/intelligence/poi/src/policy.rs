use crate::{PoiKind, PoiSource};

/// One semantic POI interest and its selection weight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoiInterest {
	pub kind: PoiKind,
	pub weight: f32,
}

impl PoiInterest {
	pub fn new(kind: PoiKind, weight: f32) -> Self {
		Self { kind, weight: weight.max(0.0) }
	}
}

/// Small, ordered set of semantic interests.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PoiInterests(Vec<PoiInterest>);

impl PoiInterests {
	pub fn new(interests: impl IntoIterator<Item = PoiInterest>) -> Self {
		Self(interests.into_iter().collect())
	}

	pub fn one(kind: PoiKind) -> Self {
		Self(vec![PoiInterest::new(kind, 1.0)])
	}

	pub fn weight(&self, kind: PoiKind) -> Option<f32> {
		self.0
			.iter()
			.find(|interest| interest.kind == kind)
			.map(|interest| interest.weight)
	}

	pub fn contains(&self, kind: PoiKind) -> bool {
		self.weight(kind).is_some_and(|weight| weight > 0.0)
	}

	pub fn iter(&self) -> impl Iterator<Item = PoiInterest> + '_ {
		self.0.iter().copied()
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	/// Add another ordered interest table, summing duplicate kind weights.
	pub fn combined(&self, other: &Self) -> Self {
		let mut combined = self.0.clone();
		for interest in other.iter() {
			if let Some(existing) =
				combined.iter_mut().find(|existing| existing.kind == interest.kind)
			{
				existing.weight += interest.weight;
			} else {
				combined.push(interest);
			}
		}
		Self(combined)
	}
}

/// Discovery cadence, acquisition rate, and memory bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PoiLearningPolicy {
	pub local_radius: f32,
	pub local_scan_interval: f32,
	pub global_scan_interval: f32,
	pub learning_rate_per_second: f32,
	pub retention_secs: f32,
	pub max_known: usize,
	pub candidates_per_scan: usize,
	/// Entries with any of these sources do not expire by age.
	pub durable_sources: PoiSource,
}

impl Default for PoiLearningPolicy {
	fn default() -> Self {
		Self {
			local_radius: 200.0,
			local_scan_interval: 0.5,
			global_scan_interval: 5.0,
			learning_rate_per_second: 4.0,
			retention_secs: 300.0,
			max_known: 256,
			candidates_per_scan: 24,
			durable_sources: PoiSource::OBJECTIVE,
		}
	}
}

/// Explicit revisit behavior; cycling is stateful rather than a numeric bias.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PoiVisitPolicy {
	Weighted {
		novelty_weight: f32,
		/// Prefer not to return before this many seconds. Not a hard freeze:
		/// if every candidate is still cooling, selection keeps circulating.
		revisit_cooldown_secs: f32,
		repeat_weight: f32,
	},
	/// Learn up to `roster_size` destinations, then visit that roster in order.
	Cycle { roster_size: usize, reshuffle_each_cycle: bool },
}

impl Default for PoiVisitPolicy {
	fn default() -> Self {
		Self::Weighted { novelty_weight: 1.5, revisit_cooldown_secs: 60.0, repeat_weight: 1.0 }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn combined_interests_retain_both_tables_and_sum_duplicates() {
		let camp = PoiKind::new("test/camp");
		let forage = PoiKind::new("test/forage");
		let personal =
			PoiInterests::new([PoiInterest::new(camp, 0.5), PoiInterest::new(forage, 1.0)]);
		let mob = PoiInterests::one(camp);
		let combined = personal.combined(&mob);
		assert_eq!(combined.weight(camp), Some(1.5));
		assert_eq!(combined.weight(forage), Some(1.0));
	}
}
