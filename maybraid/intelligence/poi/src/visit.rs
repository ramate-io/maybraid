use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::{KnownPoi, PoiId, PoiVisitPolicy};

/// Visit history and the explicit roster/cursor used by cycle policies.
#[derive(Component, Clone, Debug, Default)]
pub struct PoiVisitState {
	last_visited_at: BTreeMap<PoiId, f32>,
	visit_counts: BTreeMap<PoiId, u32>,
	cycle_roster: Vec<PoiId>,
	cycle_cursor: usize,
	cycle_round: u64,
	cycle_capacity: Option<usize>,
}

impl PoiVisitState {
	pub fn last_visited_at(&self, id: PoiId) -> Option<f32> {
		self.last_visited_at.get(&id).copied()
	}

	pub fn cycle_roster(&self) -> &[PoiId] {
		&self.cycle_roster
	}

	pub fn cycle_cursor(&self) -> usize {
		self.cycle_cursor
	}

	pub fn visit_count(&self, id: PoiId) -> u32 {
		self.visit_counts.get(&id).copied().unwrap_or(0)
	}

	pub fn add_to_cycle(&mut self, id: PoiId, roster_size: usize) -> bool {
		if roster_size == 0
			|| self.cycle_roster.len() >= roster_size
			|| self.cycle_roster.contains(&id)
		{
			return false;
		}
		self.cycle_roster.push(id);
		true
	}

	pub fn reconcile_cycle(&mut self, roster_size: usize, available: impl Fn(PoiId) -> bool) {
		if self.cycle_capacity != Some(roster_size) {
			self.cycle_roster.clear();
			self.cycle_cursor = 0;
			self.cycle_round = 0;
			self.cycle_capacity = Some(roster_size);
		}
		self.cycle_roster.retain(|id| available(*id));
		self.cycle_roster.truncate(roster_size);
		self.cycle_cursor = self.cycle_cursor.min(self.cycle_roster.len().saturating_sub(1));
	}

	pub fn next_cycle(
		&mut self,
		reshuffle_each_cycle: bool,
		available: impl Fn(PoiId) -> bool,
	) -> Option<PoiId> {
		if self.cycle_roster.is_empty() {
			return None;
		}
		if reshuffle_each_cycle && self.cycle_cursor == 0 && self.cycle_round > 0 {
			shuffle(&mut self.cycle_roster, self.cycle_round);
		}
		for _ in 0..self.cycle_roster.len() {
			let index = self.cycle_cursor % self.cycle_roster.len();
			self.cycle_cursor = (index + 1) % self.cycle_roster.len();
			if self.cycle_cursor == 0 {
				self.cycle_round = self.cycle_round.wrapping_add(1);
			}
			let id = self.cycle_roster[index];
			if available(id) {
				return Some(id);
			}
		}
		None
	}

	pub fn complete(&mut self, id: PoiId, now: f32) {
		self.last_visited_at.insert(id, now);
		let count = self.visit_counts.entry(id).or_default();
		*count = count.saturating_add(1);
	}
}

/// Selects from an already spatially filtered candidate slice.
pub fn choose_poi(
	state: &mut PoiVisitState,
	policy: PoiVisitPolicy,
	candidates: &[KnownPoi],
	now: f32,
	mut score: impl FnMut(KnownPoi) -> f32,
) -> Option<PoiId> {
	match policy {
		PoiVisitPolicy::Cycle { roster_size, reshuffle_each_cycle } => {
			if roster_size == 0 {
				return None;
			}
			state.reconcile_cycle(roster_size, |id| {
				candidates.iter().any(|candidate| candidate.id == id)
			});
			if state.cycle_roster.len() >= roster_size {
				return state.next_cycle(reshuffle_each_cycle, |id| {
					candidates.iter().any(|candidate| candidate.id == id)
				});
			}
			let selected = best(
				candidates
					.iter()
					.copied()
					.filter(|candidate| !state.cycle_roster.contains(&candidate.id)),
				&mut score,
			)
			.map(|candidate| candidate.id);
			if let Some(id) = selected {
				state.add_to_cycle(id, roster_size);
				return Some(id);
			}
			None
		}
		PoiVisitPolicy::Weighted { novelty_weight, revisit_cooldown_secs, repeat_weight } => {
			state.cycle_roster.clear();
			state.cycle_cursor = 0;
			state.cycle_round = 0;
			state.cycle_capacity = None;
			best(candidates.iter().copied(), &mut |candidate| {
				let base = score(candidate);
				let Some(last_visited) = state.last_visited_at(candidate.id) else {
					return base * novelty_weight.max(0.0);
				};
				let age = (now - last_visited).max(0.0);
				let cooldown = revisit_cooldown_secs.max(0.0);
				if age < cooldown {
					return 0.0;
				}
				base * repeat_weight.max(0.0)
			})
			.map(|candidate| candidate.id)
		}
	}
}

fn best(
	candidates: impl Iterator<Item = KnownPoi>,
	score: &mut impl FnMut(KnownPoi) -> f32,
) -> Option<KnownPoi> {
	candidates
		.filter_map(|candidate| {
			let weight = score(candidate);
			weight.is_finite().then_some((candidate, weight))
		})
		.filter(|(_, weight)| *weight > 0.0)
		.max_by(|(a, a_weight), (b, b_weight)| {
			a_weight.total_cmp(b_weight).then_with(|| b.id.cmp(&a.id))
		})
		.map(|(candidate, _)| candidate)
}

fn shuffle(values: &mut [PoiId], seed: u64) {
	let mut state = seed;
	for index in (1..values.len()).rev() {
		state = splitmix64(state);
		values.swap(index, state as usize % (index + 1));
	}
}

fn splitmix64(mut value: u64) -> u64 {
	value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
	value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ (value >> 31)
}
