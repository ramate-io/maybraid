use bevy::prelude::*;

use crate::{
	choose_poi, GlobalPoi, KnownPoi, LocalPoi, Poi, PoiGoalState, PoiGoalStatus, PoiId,
	PoiInterest, PoiInterests, PoiKind, PoiKnowledge, PoiLearningPolicy, PoiObservation,
	PoiRegistry, PoiSource, PoiVisitPolicy, PoiVisitState,
};

const CAMP: PoiKind = PoiKind::new("test/camp");
const WATER: PoiKind = PoiKind::new("test/water");

fn observation(user: Entity, id: u64, source: PoiSource, at: f32) -> PoiObservation {
	PoiObservation {
		user,
		id: PoiId(id),
		entity: None,
		kind: CAMP,
		position: Vec3::X * at,
		arrival_radius: 1.0,
		salience: 1.0,
		confidence: 0.8,
		source,
	}
}

#[test]
fn registry_separates_local_and_sparse_global_sets() -> anyhow::Result<()> {
	let mut world = World::new();
	let near = world.spawn_empty().id();
	let far = world.spawn_empty().id();
	let water = world.spawn_empty().id();
	let mut registry = PoiRegistry::default();
	registry.upsert(near, Poi::new(PoiId(1), CAMP), Vec3::X * 10.0, true, false)?;
	registry.upsert(far, Poi::new(PoiId(2), CAMP), Vec3::X * 1_000.0, false, true)?;
	registry.upsert(water, Poi::new(PoiId(3), WATER), Vec3::X * 8.0, true, true)?;

	let interests = PoiInterests::new([PoiInterest::new(CAMP, 1.0)]);
	let local = registry.local_matching(Vec3::ZERO, 200.0, &interests);
	let global = registry.global_matching(&interests);
	assert_eq!(local.iter().map(|record| record.id).collect::<Vec<_>>(), [PoiId(1)]);
	assert_eq!(global.iter().map(|record| record.id).collect::<Vec<_>>(), [PoiId(2)]);
	Ok(())
}

#[test]
fn registry_tracks_an_entity_when_its_poi_id_changes() -> anyhow::Result<()> {
	let mut world = World::new();
	let entity = world.spawn_empty().id();
	let mut registry = PoiRegistry::default();
	registry.upsert(entity, Poi::new(PoiId(1), CAMP), Vec3::ZERO, true, false)?;
	registry.upsert(entity, Poi::new(PoiId(2), CAMP), Vec3::X, true, false)?;
	assert!(registry.get(PoiId(1)).is_none());
	assert!(registry.get(PoiId(2)).is_some());
	registry.remove_entity(entity);
	assert!(registry.is_empty());
	Ok(())
}

#[test]
fn registry_rejects_duplicate_stable_ids() -> anyhow::Result<()> {
	let mut world = World::new();
	let first = world.spawn_empty().id();
	let second = world.spawn_empty().id();
	let mut registry = PoiRegistry::default();
	registry.upsert(first, Poi::new(PoiId(1), CAMP), Vec3::ZERO, true, false)?;
	registry.upsert(second, Poi::new(PoiId(2), CAMP), Vec3::X, true, false)?;
	assert!(registry.upsert(second, Poi::new(PoiId(1), CAMP), Vec3::X, true, false).is_err());
	assert!(registry.get(PoiId(1)).is_some_and(|record| record.entity == first));
	assert!(registry.get(PoiId(2)).is_none());
	assert!(registry.remove_entity(second).is_none());
	Ok(())
}

#[test]
fn knowledge_unions_sources_and_expires_only_non_durable_entries() -> anyhow::Result<()> {
	let mut world = World::new();
	let user = world.spawn_empty().id();
	let mut knowledge = PoiKnowledge::default();
	knowledge.observe(observation(user, 1, PoiSource::LOCAL_SCAN, 1.0), 1.0);
	knowledge.observe(observation(user, 1, PoiSource::SHARED, 2.0), 2.0);
	knowledge.observe(observation(user, 2, PoiSource::GLOBAL_SCAN, 1.0), 1.0);
	assert!(knowledge
		.get(PoiId(1))
		.is_some_and(|known| known.sources.contains(PoiSource::LOCAL_SCAN | PoiSource::SHARED)));

	let policy = PoiLearningPolicy {
		retention_secs: 5.0,
		durable_sources: PoiSource::GLOBAL_SCAN,
		..default()
	};
	knowledge.maintain(10.0, policy);
	assert!(knowledge.get(PoiId(1)).is_none());
	assert!(knowledge.get(PoiId(2)).is_some());
	Ok(())
}

#[test]
fn removing_one_source_preserves_other_membership() -> anyhow::Result<()> {
	let mut world = World::new();
	let user = world.spawn_empty().id();
	let mut knowledge = PoiKnowledge::default();
	knowledge.observe(observation(user, 1, PoiSource::LOCAL_SCAN, 1.0), 1.0);
	knowledge.observe(observation(user, 1, PoiSource::EXTERNAL, 1.0), 1.0);
	assert!(knowledge.remove_source(PoiId(1), PoiSource::LOCAL_SCAN));
	assert!(knowledge.get(PoiId(1)).is_some());
	assert!(knowledge.remove_source(PoiId(1), PoiSource::EXTERNAL));
	assert!(knowledge.get(PoiId(1)).is_none());
	Ok(())
}

#[test]
fn knowledge_rejects_non_finite_external_findings() -> anyhow::Result<()> {
	let mut world = World::new();
	let user = world.spawn_empty().id();
	let mut knowledge = PoiKnowledge::default();
	let mut invalid = observation(user, 1, PoiSource::EXTERNAL, 0.0);
	invalid.arrival_radius = f32::INFINITY;
	assert!(knowledge.observe(invalid, 0.0).is_none());
	assert!(knowledge.is_empty());
	Ok(())
}

#[test]
fn cycle_roster_has_an_explicit_stable_order() -> anyhow::Result<()> {
	let mut visits = PoiVisitState::default();
	assert!(visits.add_to_cycle(PoiId(4), 2));
	assert!(visits.add_to_cycle(PoiId(7), 2));
	assert!(!visits.add_to_cycle(PoiId(9), 2));
	assert_eq!(visits.next_cycle(false, |_| true), Some(PoiId(4)));
	assert_eq!(visits.next_cycle(false, |_| true), Some(PoiId(7)));
	assert_eq!(visits.next_cycle(false, |_| true), Some(PoiId(4)));
	Ok(())
}

#[test]
fn weighted_policy_prefers_novelty_then_keeps_circulating() -> anyhow::Result<()> {
	let poi = |id, x| KnownPoi {
		id: PoiId(id),
		entity: None,
		kind: CAMP,
		position: Vec3::X * x,
		arrival_radius: 1.0,
		salience: 1.0,
		confidence: 1.0,
		sources: PoiSource::LOCAL_SCAN,
		first_observed_at: 0.0,
		last_observed_at: 0.0,
	};
	let camp = poi(4, 0.0);
	let other = poi(7, 8.0);
	let policy = PoiVisitPolicy::Weighted {
		novelty_weight: 2.0,
		revisit_cooldown_secs: 10.0,
		repeat_weight: 1.0,
	};
	let mut visits = PoiVisitState::default();
	visits.complete(camp.id, 5.0);
	assert_eq!(choose_poi(&mut visits, policy, &[camp], 6.0, |_| 1.0), Some(camp.id));
	assert_eq!(choose_poi(&mut visits, policy, &[camp, other], 6.0, |_| 1.0), Some(other.id));
	visits.complete(other.id, 6.0);
	assert_eq!(choose_poi(&mut visits, policy, &[camp, other], 7.0, |_| 1.0), Some(camp.id));
	Ok(())
}

#[test]
fn goal_state_advances_generation_when_replaced() -> anyhow::Result<()> {
	let mut state = PoiGoalState::new(PoiId(1));
	state.status = PoiGoalStatus::Completed;
	assert_eq!(state.begin(PoiId(2)), 2);
	assert_eq!(state.target, PoiId(2));
	assert_eq!(state.status, PoiGoalStatus::Active);
	Ok(())
}

#[test]
fn marker_types_are_zero_sized() -> anyhow::Result<()> {
	assert_eq!(std::mem::size_of::<LocalPoi>(), 0);
	assert_eq!(std::mem::size_of::<GlobalPoi>(), 0);
	Ok(())
}
