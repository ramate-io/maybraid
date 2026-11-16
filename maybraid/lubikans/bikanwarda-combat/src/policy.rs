//! Inspectable Ward / Enraged policy and the pure target / victim predicates.
//!
//! Selection and area-damage filtering are separate functions. Callers feed
//! existing [`CombatTargeting`] membership plus real [`CombatContact`] memory.
//! Affiliation is an overlay used to record conflicts; it does not invent a
//! second enemy list.

use attacks::AttackId;
use bevy::prelude::*;
use combat_targeting::{CombatContact, CombatTargeting, TargetSource};
use threat_intelligence::Affiliations;

use crate::catalog::{ENRAGED_DEFINITION_ID, WARD_DEFINITION_ID};

/// Hostile membership bits Ward reads from the owner's targeting.
///
/// `SPOTTING` is visibility, not enemyship. `ALLY` is never hostile.
/// PLACEHOLDER: [#1137](https://github.com/ramate-io/maybraid/issues/1137)
/// does not name the exact source mask.
pub const WARD_HOSTILE_SOURCES: TargetSource = TargetSource::from_bits(
	TargetSource::OBJECTIVE.bits()
		| TargetSource::RECEIVED_FIRE.bits()
		| TargetSource::ENEMYSHIP.bits()
		| TargetSource::FIREARM.bits()
		| TargetSource::SHARED.bits(),
);

/// Encounter-facing attacks both forms reuse. `parade_blast` is a child only.
pub const AUTHORED_ATTACKS: [AttackId; 3] =
	[bikanwarda_attacks::PARADE, bikanwarda_attacks::GOD_BEAM, bikanwarda_attacks::BRAY];

/// Summoned Bikanwarda form. Also a component so hosts can query without the
/// full policy snapshot.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub enum BikanwardaSummonForm {
	/// Attacks the summoner's enemies. Area damage excludes the owner and allies.
	Ward,
	/// May select anyone, including the summoner. Area damage does not spare them.
	Enraged,
}

impl BikanwardaSummonForm {
	pub fn definition_id(self) -> lubikans::LubikanDefinitionId {
		match self {
			Self::Ward => WARD_DEFINITION_ID,
			Self::Enraged => ENRAGED_DEFINITION_ID,
		}
	}

	pub fn from_definition(id: lubikans::LubikanDefinitionId) -> Option<Self> {
		if id == WARD_DEFINITION_ID {
			Some(Self::Ward)
		} else if id == ENRAGED_DEFINITION_ID {
			Some(Self::Enraged)
		} else {
			None
		}
	}
}

/// Where affiliation and targeting disagree. Recorded, not resolved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolicyConflict {
	/// Owner targeting marks hostile; affiliation mitigates the candidate.
	TargetingHostileAffiliationAlly,
	/// Owner targeting marks ally; affiliation still reports threat.
	TargetingAllyAffiliationHostile,
	/// Affiliation reports threat, but the owner has no hostile targeting row.
	AffiliationHostileWithoutTargeting,
}

/// Snapshot a host or test can read without walking targeting maps.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct BikanwardaSummonPolicy {
	pub form: BikanwardaSummonForm,
	pub owner: Entity,
	/// Intended aim target after policy + contact. `None` if nothing is aimable.
	pub selected: Option<Entity>,
	/// Candidates the form may aim at this tick (have contact + pass selection).
	pub admitted_targets: Vec<Entity>,
	/// Entities listed on `BeginAttack::allies` this tick.
	pub excluded_victims: Vec<Entity>,
	/// PLACEHOLDER rotation index into [`AUTHORED_ATTACKS`].
	pub attack_index: u8,
	pub next_attack: AttackId,
	pub conflicts: Vec<(Entity, PolicyConflict)>,
}

impl BikanwardaSummonPolicy {
	pub fn new(form: BikanwardaSummonForm, owner: Entity) -> Self {
		Self {
			form,
			owner,
			selected: None,
			admitted_targets: Vec::new(),
			excluded_victims: Vec::new(),
			attack_index: 0,
			next_attack: AUTHORED_ATTACKS[0],
			conflicts: Vec::new(),
		}
	}

	pub fn advance_attack(&mut self) {
		let next = (self.attack_index as usize + 1) % AUTHORED_ATTACKS.len();
		self.attack_index = next as u8;
		self.next_attack = AUTHORED_ATTACKS[next];
	}
}

/// PLACEHOLDER combat knobs. [#1137](https://github.com/ramate-io/maybraid/issues/1137)
/// does not agree cadence, auto-cast, or the affiliation tie-break.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct BikanwardaSummonConfig {
	/// When false, hosts / tests write `BeginAttack` themselves.
	pub auto_cast: bool,
}

impl Default for BikanwardaSummonConfig {
	fn default() -> Self {
		Self { auto_cast: true }
	}
}

/// Inputs for one candidate. Contact is required for selection, not for
/// victim filtering (overlap is decided by the hazard).
#[derive(Clone, Copy, Debug)]
pub struct PolicyCandidate<'a> {
	pub entity: Entity,
	pub sources: TargetSource,
	pub contact: Option<&'a CombatContact>,
	pub affiliations: Option<&'a Affiliations>,
}

#[derive(Clone, Copy, Debug)]
pub struct PolicyView<'a> {
	pub form: BikanwardaSummonForm,
	pub summon: Entity,
	pub owner: Entity,
	pub owner_affiliations: Option<&'a Affiliations>,
	pub now: f32,
}

/// Whether `candidate` may be the intended aim target.
///
/// Requires real contact memory. Ward also requires a hostile targeting source
/// and refuses the owner / `ALLY` members. Enraged admits anyone except the
/// summon entity, including the summoner.
pub fn admits_target(view: PolicyView<'_>, candidate: PolicyCandidate<'_>) -> bool {
	if candidate.entity == view.summon {
		return false;
	}
	if candidate.contact.is_none() {
		return false;
	}
	match view.form {
		BikanwardaSummonForm::Ward => {
			if candidate.entity == view.owner || candidate.sources.contains(TargetSource::ALLY) {
				return false;
			}
			candidate.sources.intersects(WARD_HOSTILE_SOURCES)
		}
		BikanwardaSummonForm::Enraged => true,
	}
}

/// Whether a hazard overlapping `candidate` may write a [`damage::Hit`].
///
/// Independent of contact and of who was selected. Ward excludes the summon,
/// the protected owner, and `ALLY` members. Enraged excludes only the summon
/// (the attack owner); authored `FriendlyFire::ExcludeOwnerAndAllies` already
/// skips that entity when `allies` is empty.
pub fn admits_victim(view: PolicyView<'_>, candidate: PolicyCandidate<'_>) -> bool {
	if candidate.entity == view.summon {
		return false;
	}
	match view.form {
		BikanwardaSummonForm::Ward => {
			candidate.entity != view.owner && !candidate.sources.contains(TargetSource::ALLY)
		}
		BikanwardaSummonForm::Enraged => true,
	}
}

/// Allies stamped onto `BeginAttack` so authored friendly-fire can apply.
pub fn allies_for_begin(
	form: BikanwardaSummonForm,
	owner: Entity,
	owner_targeting: Option<&CombatTargeting>,
) -> Vec<Entity> {
	match form {
		BikanwardaSummonForm::Ward => {
			let mut allies = vec![owner];
			if let Some(targeting) = owner_targeting {
				for (entity, member) in &targeting.active {
					if *entity == owner {
						continue;
					}
					if member.has_source(TargetSource::ALLY) && member.is_member() {
						allies.push(*entity);
					}
				}
			}
			allies
		}
		BikanwardaSummonForm::Enraged => Vec::new(),
	}
}

/// Copy owner targeting + already-observed contacts onto the summon.
///
/// Does not read live transforms. Existing summon memory is kept so a host
/// can upsert a real contact (including the summoner for Enraged) without
/// this crate inventing one.
pub fn sync_summon_targeting(
	form: BikanwardaSummonForm,
	owner: Entity,
	owner_targeting: Option<&CombatTargeting>,
	summon_targeting: &mut CombatTargeting,
	exists: impl Fn(Entity) -> bool,
	now: f32,
) {
	let previous = summon_targeting.memory.clone();
	let mut next = CombatTargeting::default();
	next.memory_secs = summon_targeting.memory_secs;
	next.algebra = summon_targeting.algebra;
	next.enabled = true;

	if let Some(owner_targeting) = owner_targeting {
		for (entity, contact) in &owner_targeting.memory {
			if exists(*entity) {
				next.upsert_contact(*contact);
			}
		}
		for (entity, member) in &owner_targeting.active {
			if !exists(*entity) {
				continue;
			}
			let sources = member.active_sources();
			match form {
				BikanwardaSummonForm::Ward => {
					if sources.contains(TargetSource::ALLY) || *entity == owner {
						next.include(*entity, TargetSource::ALLY);
					} else if sources.intersects(WARD_HOSTILE_SOURCES) {
						next.include(*entity, sources & WARD_HOSTILE_SOURCES);
					}
				}
				BikanwardaSummonForm::Enraged => {
					let without_ally =
						TargetSource::from_bits(sources.bits() & !TargetSource::ALLY.bits());
					if !without_ally.is_empty() {
						next.include(*entity, without_ally);
					}
				}
			}
		}
	}

	if form == BikanwardaSummonForm::Ward {
		next.include(owner, TargetSource::ALLY);
	}

	for (entity, contact) in previous {
		if exists(entity) {
			next.upsert_contact(contact);
		}
	}

	next.rebalance(now);
	*summon_targeting = next;
}

pub fn refresh_policy(
	policy: &mut BikanwardaSummonPolicy,
	summon_targeting: &CombatTargeting,
	owner_targeting: Option<&CombatTargeting>,
	view: PolicyView<'_>,
	candidate_affiliations: &[(Entity, Affiliations)],
	exists: impl Fn(Entity) -> bool,
) {
	if !exists(policy.owner) {
		policy.selected = None;
		policy.admitted_targets.clear();
		policy.excluded_victims = allies_for_begin(policy.form, policy.owner, owner_targeting);
		policy.conflicts.clear();
		return;
	}

	policy.admitted_targets.clear();
	policy.conflicts.clear();
	policy.excluded_victims = allies_for_begin(policy.form, policy.owner, owner_targeting)
		.into_iter()
		.filter(|entity| exists(*entity))
		.collect();

	for (entity, member) in &summon_targeting.active {
		if !exists(*entity) {
			continue;
		}
		let contact = summon_targeting.contact(*entity);
		let affiliations = candidate_affiliations
			.iter()
			.find(|(candidate, _)| candidate == entity)
			.map(|(_, affiliations)| affiliations);
		let candidate = PolicyCandidate {
			entity: *entity,
			sources: owner_sources(owner_targeting, *entity, member.active_sources()),
			contact,
			affiliations,
		};
		record_conflicts(&mut policy.conflicts, view, candidate);
		if admits_target(view, candidate) {
			policy.admitted_targets.push(*entity);
		}
	}

	policy.admitted_targets.sort_by_key(|entity| entity.to_bits());
	policy.selected = choose_selected(policy, summon_targeting);
}

fn owner_sources(
	owner_targeting: Option<&CombatTargeting>,
	entity: Entity,
	fallback: TargetSource,
) -> TargetSource {
	owner_targeting
		.and_then(|targeting| targeting.active_target(entity))
		.map(combat_targeting::ActiveTarget::active_sources)
		.unwrap_or(fallback)
}

fn choose_selected(
	policy: &BikanwardaSummonPolicy,
	summon_targeting: &CombatTargeting,
) -> Option<Entity> {
	if let Some(current) = policy.selected {
		if policy.admitted_targets.contains(&current) && summon_targeting.contact(current).is_some()
		{
			return Some(current);
		}
	}
	summon_targeting.ranked.iter().find_map(|ranked| {
		policy.admitted_targets.contains(&ranked.entity).then_some(ranked.entity)
	})
}

fn record_conflicts(
	conflicts: &mut Vec<(Entity, PolicyConflict)>,
	view: PolicyView<'_>,
	candidate: PolicyCandidate<'_>,
) {
	let Some(owner) = view.owner_affiliations else {
		return;
	};
	let Some(candidate_aff) = candidate.affiliations else {
		return;
	};
	let hostile_aff = owner.threat_weight(candidate_aff, view.now) > 0.0;
	let targeting_hostile = candidate.sources.intersects(WARD_HOSTILE_SOURCES);
	let targeting_ally =
		candidate.entity == view.owner || candidate.sources.contains(TargetSource::ALLY);
	if targeting_hostile && !targeting_ally && !hostile_aff {
		conflicts.push((candidate.entity, PolicyConflict::TargetingHostileAffiliationAlly));
	} else if targeting_ally && hostile_aff {
		conflicts.push((candidate.entity, PolicyConflict::TargetingAllyAffiliationHostile));
	} else if !targeting_hostile && !targeting_ally && hostile_aff {
		conflicts.push((candidate.entity, PolicyConflict::AffiliationHostileWithoutTargeting));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use combat_targeting::CombatContact;
	use threat_intelligence::{AffiliationStrength, ThreatGroupId, ThreatId};

	fn contact(subject: Entity) -> CombatContact {
		CombatContact {
			subject,
			position: Vec3::ZERO,
			movement_vector: Vec3::ZERO,
			visible_point: Vec3::ZERO,
			visible_head: None,
			last_spotted_at: 0.0,
		}
	}

	fn view(form: BikanwardaSummonForm, summon: Entity, owner: Entity) -> PolicyView<'static> {
		PolicyView { form, summon, owner, owner_affiliations: None, now: 0.0 }
	}

	#[test]
	fn ward_requires_hostile_source_and_contact() {
		let summon = Entity::from_bits(1);
		let owner = Entity::from_bits(2);
		let raider = Entity::from_bits(3);
		let seen = contact(raider);
		let ward = view(BikanwardaSummonForm::Ward, summon, owner);
		assert!(!admits_target(
			ward,
			PolicyCandidate {
				entity: raider,
				sources: TargetSource::ENEMYSHIP,
				contact: None,
				affiliations: None,
			},
		));
		assert!(admits_target(
			ward,
			PolicyCandidate {
				entity: raider,
				sources: TargetSource::ENEMYSHIP,
				contact: Some(&seen),
				affiliations: None,
			},
		));
		assert!(!admits_target(
			ward,
			PolicyCandidate {
				entity: owner,
				sources: TargetSource::ENEMYSHIP,
				contact: Some(&seen),
				affiliations: None,
			},
		));
	}

	#[test]
	fn enraged_may_select_the_summoner_with_contact() {
		let summon = Entity::from_bits(4);
		let owner = Entity::from_bits(5);
		let seen = contact(owner);
		assert!(admits_target(
			view(BikanwardaSummonForm::Enraged, summon, owner),
			PolicyCandidate {
				entity: owner,
				sources: TargetSource::NONE,
				contact: Some(&seen),
				affiliations: None,
			},
		));
		assert!(!admits_victim(
			view(BikanwardaSummonForm::Ward, summon, owner),
			PolicyCandidate {
				entity: owner,
				sources: TargetSource::NONE,
				contact: None,
				affiliations: None,
			},
		));
		assert!(admits_victim(
			view(BikanwardaSummonForm::Enraged, summon, owner),
			PolicyCandidate {
				entity: owner,
				sources: TargetSource::NONE,
				contact: None,
				affiliations: None,
			},
		));
	}

	#[test]
	fn affiliation_without_targeting_is_not_an_enemy_list() {
		let raider = Entity::from_bits(8);
		let group = ThreatGroupId::group(32);
		let mut owner = Affiliations::with_self(ThreatId(1));
		owner.antagonize(group, AffiliationStrength::permanent(1.0));
		let mut candidate = Affiliations::default();
		candidate.join(group, AffiliationStrength::permanent(1.0));
		assert!(owner.threat_weight(&candidate, 0.0) > 0.0);
		assert!(!admits_target(
			view(BikanwardaSummonForm::Ward, Entity::from_bits(6), Entity::from_bits(7)),
			PolicyCandidate {
				entity: raider,
				sources: TargetSource::SPOTTING,
				contact: Some(&contact(raider)),
				affiliations: Some(&candidate),
			},
		));
	}
}
