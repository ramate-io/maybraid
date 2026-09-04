use std::collections::BTreeMap;

use bevy::prelude::*;

use crate::{
	AssailantAlgebra, AssailantContact, AssailantFactor, AssailantFactors, AssailantSource,
	EvasionActuator, EvasionSignal,
};

/// A temporary adjustment to one factor, exponentially decayed over time.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedInfluence {
	pub factor: AssailantFactor,
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

/// Membership, policy inputs, and latest computed weight for one assailant.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActiveAssailant {
	pub sources: AssailantSource,
	pub factors: AssailantFactors,
	pub weight: f32,
	pub influences: Vec<TimedInfluence>,
}

impl ActiveAssailant {
	pub fn is_member(&self) -> bool {
		!self.sources.is_empty()
	}

	pub fn has_source(&self, source: AssailantSource) -> bool {
		self.sources.contains(source)
	}

	fn effective_factors(&self, now: f32) -> AssailantFactors {
		let mut factors = self.factors;
		for influence in &self.influences {
			influence.factor.add_to(&mut factors, influence.decayed_value(now));
		}
		factors
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RankedAssailant {
	pub entity: Entity,
	pub weight: f32,
}

/// Distance thresholds used to route the exclusive hide | flee signal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EvasionSettings {
	/// Inside this xz distance, the signal is [`EvasionActuator::Flee`].
	pub flee_distance: f32,
	pub memory_secs: f32,
}

impl Default for EvasionSettings {
	fn default() -> Self {
		Self { flee_distance: 8.0, memory_secs: 4.0 }
	}
}

/// Assailant memory, ranking, and the hide | flee signal civilians consume.
#[derive(Component, Clone, Debug)]
pub struct EvasionIntelligenceUser {
	pub memory: BTreeMap<Entity, AssailantContact>,
	pub active: BTreeMap<Entity, ActiveAssailant>,
	pub ranked: Vec<RankedAssailant>,
	pub algebra: AssailantAlgebra,
	pub settings: EvasionSettings,
	pub signal: EvasionSignal,
	/// Higher-order grant. When false, spotting does not admit new assailants and ranking idles.
	pub enabled: bool,
	pub dirty: bool,
	pub(crate) next_rebalance_at: f32,
}

impl EvasionIntelligenceUser {
	pub fn new(settings: EvasionSettings) -> Self {
		Self { settings, ..Self::default() }
	}

	/// Records a visual snapshot and admits `SPOTTING`.
	pub fn upsert_sighting(&mut self, contact: AssailantContact) -> Option<AssailantContact> {
		self.include(contact.subject, AssailantSource::SPOTTING);
		self.dirty = true;
		self.memory.insert(contact.subject, contact)
	}

	/// Records a non-visual last-known position (heard shot, reported location).
	pub fn note_stimulus(
		&mut self,
		contact: AssailantContact,
		source: AssailantSource,
	) -> Option<AssailantContact> {
		self.include(contact.subject, source);
		self.dirty = true;
		self.memory.insert(contact.subject, contact)
	}

	pub fn include(&mut self, entity: Entity, source: AssailantSource) -> bool {
		if source.is_empty() {
			return false;
		}
		let assailant = self.active.entry(entity).or_default();
		let previous = assailant.sources;
		assailant.sources.insert(source);
		let changed = assailant.sources != previous;
		self.dirty |= changed;
		changed
	}

	pub fn remove_source(&mut self, entity: Entity, source: AssailantSource) -> bool {
		let Some(assailant) = self.active.get_mut(&entity) else {
			return false;
		};
		let previous = assailant.sources;
		assailant.sources.remove(source);
		let changed = assailant.sources != previous;
		if assailant.sources.is_empty() {
			self.active.remove(&entity);
		}
		self.dirty |= changed;
		changed
	}

	pub fn clear_source(&mut self, source: AssailantSource) {
		let subjects: Vec<_> = self
			.active
			.iter()
			.filter(|(_, assailant)| assailant.has_source(source))
			.map(|(entity, _)| *entity)
			.collect();
		for entity in subjects {
			self.remove_source(entity, source);
		}
	}

	pub fn set_factors(&mut self, entity: Entity, factors: AssailantFactors) -> bool {
		let Some(assailant) = self.active.get_mut(&entity) else {
			return false;
		};
		let changed = assailant.factors != factors;
		assailant.factors = factors;
		self.dirty |= changed;
		changed
	}

	pub fn add_influence(&mut self, entity: Entity, influence: TimedInfluence) -> bool {
		let Some(assailant) = self.active.get_mut(&entity) else {
			return false;
		};
		assailant.influences.push(influence);
		self.dirty = true;
		true
	}

	pub fn contact(&self, entity: Entity) -> Option<&AssailantContact> {
		self.memory.get(&entity)
	}

	pub fn active_assailant(&self, entity: Entity) -> Option<&ActiveAssailant> {
		self.active.get(&entity)
	}

	pub fn best(&self) -> Option<&RankedAssailant> {
		self.ranked.first()
	}

	/// Highest-ranked assailant that still has usable last-known memory.
	pub fn best_contact(&self) -> Option<&AssailantContact> {
		self.ranked.iter().find_map(|assailant| self.memory.get(&assailant.entity))
	}

	pub fn needs_rebalance(&self, now: f32) -> bool {
		self.dirty || now >= self.next_rebalance_at
	}

	pub fn rebalance(&mut self, now: f32, observer: Vec3) {
		let memory_secs = self.settings.memory_secs.max(0.0);
		self.memory.retain(|_, contact| contact.is_fresh(now, memory_secs));

		for (entity, assailant) in &mut self.active {
			if assailant.has_source(AssailantSource::SPOTTING) && !self.memory.contains_key(entity)
			{
				assailant.sources.remove(AssailantSource::SPOTTING);
			}
		}
		self.active.retain(|_, assailant| !assailant.sources.is_empty());

		self.ranked.clear();
		for (entity, assailant) in &mut self.active {
			assailant
				.influences
				.retain(|influence| influence.decayed_value(now).abs() >= 1e-3);
			if !assailant.is_member() {
				assailant.weight = 0.0;
				continue;
			}
			if let Some(contact) = self.memory.get(entity) {
				let distance = contact.xz_distance(observer);
				assailant.factors.proximity = 1.0 / (1.0 + distance.max(0.0));
				assailant.factors.bias = -distance;
			}
			let factors = assailant.effective_factors(now);
			assailant.weight = self.algebra.score(factors);
			self.ranked.push(RankedAssailant { entity: *entity, weight: assailant.weight });
		}
		self.ranked.sort_by(|a, b| {
			b.weight
				.total_cmp(&a.weight)
				.then_with(|| a.entity.to_bits().cmp(&b.entity.to_bits()))
		});
		self.dirty = false;
		self.signal = self.route_signal_from(observer);
		let next_memory_expiry = self
			.memory
			.values()
			.map(|contact| contact.last_known_at + memory_secs)
			.reduce(f32::min)
			.unwrap_or(f32::INFINITY);
		let has_influences = self.active.values().any(|assailant| !assailant.influences.is_empty());
		self.next_rebalance_at = if has_influences {
			(now + 1.0 / 30.0).min(next_memory_expiry)
		} else {
			next_memory_expiry
		};
	}

	pub fn route_signal_from(&self, observer: Vec3) -> EvasionSignal {
		let Some(contact) = self.best_contact() else {
			return EvasionSignal::idle();
		};
		let threat = Some(contact.subject);
		if contact.xz_distance(observer) <= self.settings.flee_distance.max(0.0) {
			EvasionSignal { actuator: EvasionActuator::Flee, threat }
		} else {
			EvasionSignal { actuator: EvasionActuator::Hide, threat }
		}
	}
}

impl Default for EvasionIntelligenceUser {
	fn default() -> Self {
		Self {
			memory: BTreeMap::new(),
			active: BTreeMap::new(),
			ranked: Vec::new(),
			algebra: AssailantAlgebra::default(),
			settings: EvasionSettings::default(),
			signal: EvasionSignal::idle(),
			enabled: true,
			dirty: true,
			next_rebalance_at: 0.0,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn contact(subject: Entity, position: Vec3, at: f32) -> AssailantContact {
		AssailantContact { subject, position, movement_vector: Vec3::ZERO, last_known_at: at }
	}

	#[test]
	fn best_contact_skips_membership_without_memory() -> anyhow::Result<()> {
		let unknown = Entity::from_bits(1);
		let known = Entity::from_bits(2);
		let mut user = EvasionIntelligenceUser::default();
		user.include(unknown, AssailantSource::ENEMYSHIP);
		user.upsert_sighting(contact(known, Vec3::X * 12.0, 0.0));
		user.rebalance(0.0, Vec3::ZERO);

		assert_eq!(user.best_contact().map(|contact| contact.subject), Some(known));
		assert_eq!(user.signal.actuator, EvasionActuator::Hide);
		Ok(())
	}

	#[test]
	fn close_actionable_contact_routes_to_flee() -> anyhow::Result<()> {
		let threat = Entity::from_bits(3);
		let mut user =
			EvasionIntelligenceUser::new(EvasionSettings { flee_distance: 8.0, memory_secs: 4.0 });
		user.upsert_sighting(contact(threat, Vec3::X * 3.0, 0.0));
		user.rebalance(0.0, Vec3::ZERO);
		assert!(user.signal.is_flee());
		assert_eq!(user.signal.threat, Some(threat));
		Ok(())
	}

	#[test]
	fn received_fire_keeps_membership_without_a_sighting_source() -> anyhow::Result<()> {
		let shooter = Entity::from_bits(4);
		let mut user =
			EvasionIntelligenceUser::new(EvasionSettings { flee_distance: 8.0, memory_secs: 1.0 });
		user.note_stimulus(contact(shooter, Vec3::X * 20.0, 0.0), AssailantSource::RECEIVED_FIRE);
		user.rebalance(0.5, Vec3::ZERO);
		assert!(user
			.active_assailant(shooter)
			.is_some_and(|assailant| assailant.has_source(AssailantSource::RECEIVED_FIRE)
				&& !assailant.has_source(AssailantSource::SPOTTING)));
		assert!(user.best_contact().is_some());
		Ok(())
	}
}
