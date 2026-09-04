use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::{CombatContact, TargetAlgebra, TargetFactor, TargetFactors, TargetSource};

const ENGAGED_CONTINUITY: f32 = 1.0;

/// A temporary adjustment to one factor, exponentially decayed over time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedInfluence {
	pub factor: TargetFactor,
	pub magnitude: f32,
	pub applied_at: f32,
	pub half_life: f32,
}

impl TimedInfluence {
	pub fn decayed_value(self, now: f32) -> f32 {
		let elapsed = (now - self.applied_at).max(0.0);
		if self.half_life <= 0.0 {
			return if elapsed == 0.0 { self.magnitude } else { 0.0 };
		}
		self.magnitude * (-elapsed / self.half_life).exp2()
	}
}

/// Membership, policy inputs, and latest computed weight for one target.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActiveTarget {
	pub sources: TargetSource,
	/// Source bits blocked from contributing membership.
	pub exclusions: TargetSource,
	pub factors: TargetFactors,
	pub weight: f32,
	pub influences: Vec<TimedInfluence>,
}

impl ActiveTarget {
	pub fn active_sources(&self) -> TargetSource {
		self.sources & !self.exclusions
	}

	pub fn is_member(&self) -> bool {
		!self.active_sources().is_empty()
	}

	pub fn has_source(&self, source: TargetSource) -> bool {
		self.sources.contains(source)
	}

	pub fn include(&mut self, source: TargetSource) {
		self.sources.insert(source);
	}

	pub fn remove(&mut self, source: TargetSource) {
		self.sources.remove(source);
	}

	pub fn exclude(&mut self, source: TargetSource) {
		self.exclusions.insert(source);
	}

	pub fn allow(&mut self, source: TargetSource) {
		self.exclusions.remove(source);
	}

	fn effective_factors(&self, now: f32, engaged: bool) -> TargetFactors {
		let mut factors = self.factors;
		if engaged {
			factors.continuity += ENGAGED_CONTINUITY;
		}
		for influence in &self.influences {
			influence.factor.add_to(&mut factors, influence.decayed_value(now));
		}
		factors
	}
}

/// One entry in descending target order.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RankedTarget {
	pub entity: Entity,
	pub weight: f32,
}

/// Per-combatant target memory, membership, and ranking state.
#[derive(Component, Clone, Debug)]
pub struct CombatTargeting {
	pub memory: BTreeMap<Entity, CombatContact>,
	pub active: BTreeMap<Entity, ActiveTarget>,
	pub ranked: Vec<RankedTarget>,
	pub algebra: TargetAlgebra,
	pub engaged: Option<Entity>,
	pub memory_secs: f32,
	/// Higher-order grant. When false, spotting and ranking do not admit new work.
	pub enabled: bool,
	pub dirty: bool,
	pub(crate) next_rebalance_at: f32,
}

impl CombatTargeting {
	pub fn upsert_contact(&mut self, contact: CombatContact) -> Option<CombatContact> {
		let subject = contact.subject;
		self.include(subject, TargetSource::SPOTTING);
		self.dirty = true;
		self.memory.insert(subject, contact)
	}

	pub fn include(&mut self, entity: Entity, source: TargetSource) -> bool {
		if source.is_empty() {
			return false;
		}
		let target = self.active.entry(entity).or_default();
		let previous = target.sources;
		target.include(source);
		let changed = target.sources != previous;
		self.dirty |= changed;
		changed
	}

	pub fn remove_source(&mut self, entity: Entity, source: TargetSource) -> bool {
		let Some(target) = self.active.get_mut(&entity) else {
			return false;
		};
		let previous = target.sources;
		target.remove(source);
		let changed = target.sources != previous;
		let empty = target.sources.is_empty();
		if empty {
			self.active.remove(&entity);
		}
		self.dirty |= changed;
		changed
	}

	pub fn clear_source(&mut self, source: TargetSource) {
		let subjects: Vec<_> = self
			.active
			.iter()
			.filter(|(_, target)| target.has_source(source))
			.map(|(entity, _)| *entity)
			.collect();
		for entity in subjects {
			self.remove_source(entity, source);
		}
	}

	pub fn exclude(&mut self, entity: Entity, source: TargetSource) -> bool {
		let Some(target) = self.active.get_mut(&entity) else {
			return false;
		};
		let previous = target.exclusions;
		target.exclude(source);
		let changed = target.exclusions != previous;
		self.dirty |= changed;
		changed
	}

	pub fn allow(&mut self, entity: Entity, source: TargetSource) -> bool {
		let Some(target) = self.active.get_mut(&entity) else {
			return false;
		};
		let previous = target.exclusions;
		target.allow(source);
		let changed = target.exclusions != previous;
		self.dirty |= changed;
		changed
	}

	pub fn set_factors(&mut self, entity: Entity, factors: TargetFactors) -> bool {
		let Some(target) = self.active.get_mut(&entity) else {
			return false;
		};
		let changed = target.factors != factors;
		target.factors = factors;
		self.dirty |= changed;
		changed
	}

	pub fn add_influence(&mut self, entity: Entity, influence: TimedInfluence) -> bool {
		let Some(target) = self.active.get_mut(&entity) else {
			return false;
		};
		target.influences.push(influence);
		self.dirty = true;
		true
	}

	pub fn contact(&self, entity: Entity) -> Option<&CombatContact> {
		self.memory.get(&entity)
	}

	pub fn active_target(&self, entity: Entity) -> Option<&ActiveTarget> {
		self.active.get(&entity)
	}

	pub fn best(&self) -> Option<&RankedTarget> {
		self.ranked.first()
	}

	/// Returns the highest-ranked target backed by usable contact memory.
	pub fn best_contact(&self) -> Option<&CombatContact> {
		self.ranked.iter().find_map(|target| self.memory.get(&target.entity))
	}

	/// Returns the highest current algebra result. Engagement continuity is
	/// already represented as a factor during ranking.
	pub fn current(&self) -> Option<&RankedTarget> {
		self.best()
	}

	pub fn engage(&mut self, entity: Entity) -> bool {
		let can_engage = self.active.get(&entity).is_some_and(ActiveTarget::is_member);
		if can_engage {
			self.dirty |= self.engaged != Some(entity);
			self.engaged = Some(entity);
		}
		can_engage
	}

	pub fn clear_engagement(&mut self) {
		self.dirty |= self.engaged.is_some();
		self.engaged = None;
	}

	pub fn needs_rebalance(&self, now: f32) -> bool {
		self.dirty || now >= self.next_rebalance_at
	}

	/// Expires contacts, applies current influence decay, and rebuilds rank.
	pub fn rebalance(&mut self, now: f32) {
		self.memory.retain(|_, contact| contact.is_fresh(now, self.memory_secs));

		for (entity, target) in &mut self.active {
			if target.has_source(TargetSource::SPOTTING) && !self.memory.contains_key(entity) {
				target.remove(TargetSource::SPOTTING);
			}
		}
		self.active.retain(|_, target| !target.sources.is_empty());

		self.ranked.clear();
		for (entity, target) in &mut self.active {
			target.influences.retain(|influence| influence.decayed_value(now).abs() >= 1e-3);
			if !target.is_member() {
				target.weight = 0.0;
				continue;
			}
			let factors = target.effective_factors(now, self.engaged == Some(*entity));
			target.weight = self.algebra.score(factors);
			self.ranked.push(RankedTarget { entity: *entity, weight: target.weight });
		}
		self.ranked.sort_by(|a, b| {
			b.weight
				.total_cmp(&a.weight)
				.then_with(|| a.entity.to_bits().cmp(&b.entity.to_bits()))
		});

		if self
			.engaged
			.is_some_and(|engaged| !self.ranked.iter().any(|target| target.entity == engaged))
		{
			self.engaged = None;
		}
		self.dirty = false;
		let next_memory_expiry = self
			.memory
			.values()
			.map(|contact| contact.last_spotted_at + self.memory_secs.max(0.0))
			.reduce(f32::min)
			.unwrap_or(f32::INFINITY);
		let has_influences = self.active.values().any(|target| !target.influences.is_empty());
		self.next_rebalance_at = if has_influences {
			(now + 1.0 / 30.0).min(next_memory_expiry)
		} else {
			next_memory_expiry
		};
	}
}

impl Default for CombatTargeting {
	fn default() -> Self {
		Self {
			memory: BTreeMap::new(),
			active: BTreeMap::new(),
			ranked: Vec::new(),
			algebra: TargetAlgebra::default(),
			engaged: None,
			memory_secs: 3.0,
			enabled: true,
			dirty: true,
			next_rebalance_at: 0.0,
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;

	use crate::{
		CombatContact, CombatTargeting, TargetFactor, TargetFactors, TargetSource, TimedInfluence,
	};

	fn contact(subject: Entity, spotted_at: f32) -> CombatContact {
		CombatContact {
			subject,
			position: Vec3::X,
			movement_vector: Vec3::ZERO,
			visible_point: Vec3::X,
			visible_head: Some(Vec3::X + Vec3::Y),
			last_spotted_at: spotted_at,
		}
	}

	#[test]
	fn removing_active_membership_preserves_memory() -> anyhow::Result<()> {
		let entity = Entity::from_bits(1);
		let mut targeting = CombatTargeting::default();
		targeting.upsert_contact(contact(entity, 0.0));

		assert!(targeting.remove_source(entity, TargetSource::SPOTTING));
		assert!(targeting.contact(entity).is_some());
		assert!(targeting.active_target(entity).is_none());
		Ok(())
	}

	#[test]
	fn influence_decays_by_half_life() -> anyhow::Result<()> {
		let influence = TimedInfluence {
			factor: TargetFactor::Threat,
			magnitude: 8.0,
			applied_at: 2.0,
			half_life: 4.0,
		};

		assert_eq!(influence.decayed_value(2.0), 8.0);
		assert_eq!(influence.decayed_value(6.0), 4.0);
		assert_eq!(influence.decayed_value(10.0), 2.0);
		Ok(())
	}

	#[test]
	fn clear_source_removes_matching_membership() -> anyhow::Result<()> {
		let entity = Entity::from_bits(1);
		let mut targeting = CombatTargeting::default();
		targeting.include(entity, TargetSource::ENEMYSHIP | TargetSource::SPOTTING);
		targeting.clear_source(TargetSource::SPOTTING);
		assert!(targeting
			.active_target(entity)
			.is_some_and(|target| target.has_source(TargetSource::ENEMYSHIP)
				&& !target.has_source(TargetSource::SPOTTING)));
		targeting.clear_source(TargetSource::ENEMYSHIP);
		assert!(targeting.active_target(entity).is_none());
		Ok(())
	}

	#[test]
	fn ranking_is_descending_with_entity_bit_ties() -> anyhow::Result<()> {
		let a = Entity::from_bits(1);
		let b = Entity::from_bits(2);
		let c = Entity::from_bits(3);
		let mut targeting = CombatTargeting::default();
		for entity in [a, b, c] {
			targeting.include(entity, TargetSource::OBJECTIVE);
		}
		assert!(targeting.set_factors(a, TargetFactors { opportunity: 2.0, ..Default::default() },));
		for entity in [b, c] {
			assert!(targeting
				.set_factors(entity, TargetFactors { hostility: 2.0, ..Default::default() },));
		}

		targeting.rebalance(0.0);
		let entities: Vec<Entity> = targeting.ranked.iter().map(|target| target.entity).collect();
		assert_eq!(entities, vec![b, c, a]);
		Ok(())
	}

	#[test]
	fn best_contact_skips_higher_ranked_members_without_memory() -> anyhow::Result<()> {
		let unknown = Entity::from_bits(1);
		let known = Entity::from_bits(2);
		let mut targeting = CombatTargeting::default();
		targeting.include(unknown, TargetSource::ENEMYSHIP);
		assert!(targeting
			.set_factors(unknown, TargetFactors { opportunity: 10.0, ..Default::default() },));
		targeting.upsert_contact(contact(known, 0.0));
		targeting.rebalance(0.0);

		assert_eq!(targeting.best().map(|target| target.entity), Some(unknown));
		assert_eq!(targeting.best_contact().map(|contact| contact.subject), Some(known));
		Ok(())
	}

	#[test]
	fn engagement_adds_continuity_without_mutating_base_factors() -> anyhow::Result<()> {
		let a = Entity::from_bits(1);
		let b = Entity::from_bits(2);
		let mut targeting = CombatTargeting::default();
		targeting.include(a, TargetSource::OBJECTIVE);
		targeting.include(b, TargetSource::OBJECTIVE);
		assert!(targeting.engage(b));

		targeting.rebalance(0.0);
		assert_eq!(targeting.best().map(|target| target.entity), Some(b));
		assert_eq!(targeting.current().map(|target| target.entity), Some(b));
		assert_eq!(targeting.active_target(b).map(|target| target.factors.continuity), Some(0.0),);
		Ok(())
	}

	#[test]
	fn a_better_algebra_result_can_displace_engagement() -> anyhow::Result<()> {
		let engaged = Entity::from_bits(1);
		let better = Entity::from_bits(2);
		let mut targeting = CombatTargeting::default();
		targeting.include(engaged, TargetSource::OBJECTIVE);
		targeting.include(better, TargetSource::OBJECTIVE);
		assert!(targeting.engage(engaged));
		assert!(targeting
			.set_factors(better, TargetFactors { opportunity: 10.0, ..Default::default() },));

		targeting.rebalance(0.0);
		assert_eq!(targeting.current().map(|target| target.entity), Some(better));
		Ok(())
	}

	#[test]
	fn expiry_removes_spotting_but_preserves_other_sources() -> anyhow::Result<()> {
		let spotting_only = Entity::from_bits(1);
		let objective = Entity::from_bits(2);
		let mut targeting = CombatTargeting { memory_secs: 1.0, ..Default::default() };
		targeting.upsert_contact(contact(spotting_only, 0.0));
		targeting.upsert_contact(contact(objective, 0.0));
		targeting.include(objective, TargetSource::OBJECTIVE);
		assert!(targeting.engage(spotting_only));

		targeting.rebalance(2.0);
		assert!(targeting.memory.is_empty());
		assert!(targeting.active_target(spotting_only).is_none());
		let objective_target = targeting.active_target(objective);
		assert!(objective_target.is_some_and(|target| {
			target.has_source(TargetSource::OBJECTIVE) && !target.has_source(TargetSource::SPOTTING)
		}));
		assert_eq!(targeting.engaged, None);
		Ok(())
	}
}
