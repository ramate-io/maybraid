use bevy::prelude::*;

use crate::{
	GlobalPoi, LocalPoi, Poi, PoiIntelligenceUser, PoiKnowledge, PoiObservation, PoiRecord,
	PoiRegistry, PoiSource,
};

type PoiEntity<'a> = (Entity, &'a Poi, &'a GlobalTransform, Has<LocalPoi>, Has<GlobalPoi>);
type ChangedPoiEntity =
	Or<(Added<Poi>, Changed<Poi>, Changed<GlobalTransform>, Added<LocalPoi>, Added<GlobalPoi>)>;

/// Mirrors changed entity-backed POIs into the query-optimized registry.
pub fn sync_poi_registry(
	changed: Query<PoiEntity<'_>, ChangedPoiEntity>,
	current: Query<PoiEntity<'_>>,
	mut removed_pois: RemovedComponents<Poi>,
	mut removed_local: RemovedComponents<LocalPoi>,
	mut removed_global: RemovedComponents<GlobalPoi>,
	mut registry: ResMut<PoiRegistry>,
) {
	for (entity, poi, transform, local, global) in &changed {
		upsert(&mut registry, entity, *poi, transform.translation(), local, global);
	}

	let mut removed: Vec<_> = removed_pois
		.read()
		.chain(removed_local.read())
		.chain(removed_global.read())
		.collect();
	removed.sort();
	removed.dedup();
	for entity in removed {
		if let Ok((entity, poi, transform, local, global)) = current.get(entity) {
			upsert(&mut registry, entity, *poi, transform.translation(), local, global);
		} else {
			registry.remove_entity(entity);
		}
	}
}

fn upsert(
	registry: &mut PoiRegistry,
	entity: Entity,
	poi: Poi,
	position: Vec3,
	local: bool,
	global: bool,
) {
	if !position.is_finite() {
		warn!("ignoring POI {entity} with non-finite position {position}");
		registry.remove_entity(entity);
		return;
	}
	if !local && !global {
		registry.remove_entity(entity);
		return;
	}
	if let Err(error) = registry.upsert(entity, poi, position, local, global) {
		warn!("failed to index POI {entity}: {error}");
	}
}

/// Accepts directed findings from perception, scripting, sharing, or other systems.
pub fn ingest_poi_observations(
	time: Res<Time>,
	mut observations: MessageReader<PoiObservation>,
	mut users: Query<(&PoiIntelligenceUser, &mut PoiKnowledge)>,
) {
	let now = time.elapsed_secs();
	for observation in observations.read() {
		let Ok((user, mut knowledge)) = users.get_mut(observation.user) else {
			continue;
		};
		if user.interests.contains(observation.kind) {
			knowledge.observe(*observation, now);
		}
	}
}

/// Performs budgeted local and sparse-global scans, then maintains retained memory.
pub fn discover_pois(
	time: Res<Time>,
	registry: Res<PoiRegistry>,
	mut users: Query<(Entity, &GlobalTransform, &mut PoiIntelligenceUser, &mut PoiKnowledge)>,
) {
	let now = time.elapsed_secs();
	let delta = time.delta_secs();
	for (entity, transform, mut user, mut knowledge) in &mut users {
		user.accrue_learning(delta);
		let position = transform.translation();

		if now >= user.next_local_scan_at {
			user.next_local_scan_at =
				now + staggered_interval(user.policy.local_scan_interval, entity, 0);
			let records =
				registry.local_matching(position, user.policy.local_radius, &user.interests);
			let cursor = user.local_cursor;
			learn_records(
				entity,
				records,
				PoiSource::LOCAL_SCAN,
				now,
				cursor,
				&mut user,
				&mut knowledge,
			);
			user.local_cursor = user.local_cursor.wrapping_add(user.policy.candidates_per_scan);
		}

		if now >= user.next_global_scan_at {
			user.next_global_scan_at =
				now + staggered_interval(user.policy.global_scan_interval, entity, 1);
			let records = registry.global_matching(&user.interests);
			let cursor = user.global_cursor;
			learn_records(
				entity,
				records,
				PoiSource::GLOBAL_SCAN,
				now,
				cursor,
				&mut user,
				&mut knowledge,
			);
			user.global_cursor = user.global_cursor.wrapping_add(user.policy.candidates_per_scan);
		}

		knowledge.maintain(now, user.policy);
	}
}

fn staggered_interval(interval: f32, entity: Entity, salt: u64) -> f32 {
	let bits = entity.to_bits().wrapping_add(salt.wrapping_mul(0x9e37_79b9));
	let jitter = (bits % 1_001) as f32 / 1_000.0;
	interval.max(0.05) * (0.8 + jitter * 0.4)
}

fn learn_records(
	user_entity: Entity,
	records: Vec<PoiRecord>,
	source: PoiSource,
	now: f32,
	cursor: usize,
	user: &mut PoiIntelligenceUser,
	knowledge: &mut PoiKnowledge,
) {
	if records.is_empty() {
		return;
	}
	let budget = user.policy.candidates_per_scan.min(records.len());
	for offset in 0..budget {
		let record = records[(cursor + offset) % records.len()];
		if knowledge.get(record.id).is_none() && !user.try_take_learning_credit() {
			continue;
		}
		knowledge.observe(record.observation(user_entity, source), now);
	}
}
