//! Distant-tile POI selection over an HCSG [`PoiOrigin`] scheme plus retained knowledge.

use std::collections::HashMap;

use bevy::prelude::*;
use lod::hcsg::{HcsgDemand, HcsgStorage};
use meandering_intelligence::MeanderingIntelligenceUser;
use poi_intelligence::{
	begin_poi_goal, choose_poi, DistantTilePoiLookup, DistantTileRequest, KnownPoi, PoiEntry,
	PoiGoal, PoiGoalCompleted, PoiGoalState, PoiIntelligenceUser, PoiInterests, PoiKnowledge,
	PoiSource, PoiSystems, PoiTilePoll, PoiTileQuery, PoiVisitPolicy, PoiVisitState,
};

const DEFAULT_TILE_SIZE: f32 = 256.0;
const MAX_TILE_SIZE: f32 = 1_000.0;
const MAX_TILE_DISTANCE: u32 = 32_767;

/// Probes deterministic distant tiles and delegates a selected destination to `PoiGoal`.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct JourneyingIntelligenceUser {
	pub tile_size: f32,
	pub min_tile_distance: u32,
	pub max_tile_distance: u32,
	pub tile_probes: usize,
	pub visit_policy: PoiVisitPolicy,
	pub seed: u64,
	pub selection_interval: f32,
	pub empty_tile_retry_secs: f32,
	pub linger_secs: f32,
	selection_step: u64,
	next_selection_at: f32,
	empty_tiles: HashMap<IVec2, f32>,
}

impl Default for JourneyingIntelligenceUser {
	fn default() -> Self {
		Self {
			tile_size: 256.0,
			min_tile_distance: 2,
			max_tile_distance: 8,
			tile_probes: 8,
			visit_policy: PoiVisitPolicy::default(),
			seed: 0,
			selection_interval: 0.5,
			empty_tile_retry_secs: 30.0,
			linger_secs: 0.0,
			selection_step: 0,
			next_selection_at: 0.0,
			empty_tiles: HashMap::new(),
		}
	}
}

impl JourneyingIntelligenceUser {
	pub fn new(seed: u64) -> Self {
		Self { seed, ..default() }
	}

	fn cycle_is_full(&self, visits: &PoiVisitState) -> bool {
		matches!(
			self.visit_policy,
			PoiVisitPolicy::Cycle { roster_size, .. }
				if roster_size > 0 && visits.cycle_roster().len() >= roster_size
		)
	}

	fn distant_tile(&self, at: Vec3, step: u64) -> IVec2 {
		let origin = tile_of(at, normalized_tile_size(self.tile_size));
		let hash = splitmix64(self.seed.wrapping_add(step));
		let minimum = self.min_tile_distance.clamp(1, MAX_TILE_DISTANCE);
		let maximum = self.max_tile_distance.clamp(minimum, MAX_TILE_DISTANCE);
		let span = u64::from(maximum) - u64::from(minimum) + 1;
		let distance = (u64::from(minimum) + hash % span) as i32;
		origin + square_ring_offset(distance, hash >> 32)
	}

	fn select_from_distant_tile(
		&mut self,
		selection: TileSelection<'_>,
		tile_matches: &mut Vec<PoiEntry>,
		pending: &mut Option<PendingTileSelection>,
	) -> Option<poi_intelligence::PoiId> {
		let TileSelection {
			at,
			user_entity,
			storage,
			demand,
			lookup,
			learner,
			knowledge,
			visits,
			now,
		} = selection;
		let empty_tile_retry_secs = self.empty_tile_retry_secs.max(0.0);
		self.empty_tiles
			.retain(|_, checked_at| now - *checked_at < empty_tile_retry_secs);
		let probes = self.tile_probes.max(1);
		let tile_size = normalized_tile_size(self.tile_size);
		if pending
			.as_ref()
			.is_some_and(|pending| !pending.matches(self, at, &learner.interests, lookup))
		{
			*pending = None;
		}
		let progress = pending.get_or_insert_with(|| PendingTileSelection {
			key: PendingSelectionKey::new(self, at, &learner.interests),
			at,
			retry_at: 0.0,
			probe: 0,
			request: None,
			lookup: lookup.cloned(),
		});
		while progress.probe < probes {
			let step = self.selection_step.wrapping_add(progress.probe as u64);
			let tile = self.distant_tile(progress.at, step);
			if self.empty_tiles.contains_key(&tile) {
				progress.probe += 1;
				continue;
			}
			tile_matches.clear();
			let mut generated_complete = false;
			let mut generation_pending = false;
			if let (Some(storage), Some(demand), Some(lookup)) = (storage, demand, lookup) {
				if progress.request.is_none() {
					progress.request = lookup.0.request_in_xz_tile(PoiTileQuery {
						tile,
						tile_size,
						center_y: progress.at.y,
						interests: learner.interests.clone(),
					});
				}
				if let Some(request) = progress.request.as_mut() {
					generation_pending = true;
					match request.poll(storage, demand, tile_matches, now >= progress.retry_at) {
						Ok(PoiTilePoll::Ready | PoiTilePoll::ReadyEmpty) => {
							generated_complete = true;
							generation_pending = false;
						}
						Ok(PoiTilePoll::Pending | PoiTilePoll::Capacity) | Err(_) => {}
						Ok(status @ (PoiTilePoll::Stale | PoiTilePoll::Failed)) => {
							// Retain the exact probe/step: no completion or empty proof.
							if status == PoiTilePoll::Stale {
								progress.request = None;
							}
							if status == PoiTilePoll::Failed {
								progress.retry_at = now + self.selection_interval.max(0.05);
							}
						}
					}
				}
			}
			if !generation_pending {
				progress.request = None;
				progress.probe += 1;
			}
			let mut candidates: Vec<_> = knowledge
				.matching(&learner.interests)
				.copied()
				.filter(|known| tile_of(known.position, tile_size) == tile)
				.collect();
			if tile_matches.is_empty() && candidates.is_empty() {
				if generation_pending {
					return None;
				}
				if generated_complete {
					self.empty_tiles.insert(tile, now);
				}
				break;
			}
			for entry in tile_matches.iter().copied() {
				if knowledge.get(entry.id).is_none() && !learner.try_take_learning_credit() {
					continue;
				}
				let source = entry.discovery_source();
				if let Some(known) = knowledge.observe(entry.observation(user_entity, source), now)
				{
					if !candidates.iter().any(|candidate| candidate.id == known.id) {
						candidates.push(known);
					}
				}
			}
			for known in knowledge.matching(&learner.interests).copied() {
				if visits.cycle_roster().contains(&known.id)
					&& !candidates.iter().any(|candidate| candidate.id == known.id)
				{
					candidates.push(known);
				}
			}
			if candidates.is_empty() {
				break;
			}
			self.empty_tiles.remove(&tile);
			let center = tile_center(tile, tile_size);
			let selected = choose_poi(visits, self.visit_policy, &candidates, now, |known| {
				base_score(known, &learner.interests) / (1.0 + known.position.xz().distance(center))
			});
			if selected.is_some() {
				self.selection_step = step.wrapping_add(1);
				*pending = None;
				return selected;
			}
			break;
		}
		if progress.probe >= probes {
			self.selection_step = self.selection_step.wrapping_add(probes as u64);
			*pending = None;
		}
		None
	}
}

/// Only semantic configuration and the actor's origin tile invalidate a probe.
/// Small pose changes (including gravity settling) must not restart generation.
#[derive(PartialEq)]
struct PendingSelectionKey {
	origin: IVec2,
	tile_size: f32,
	minimum: u32,
	maximum: u32,
	probes: usize,
	seed: u64,
	step: u64,
	interests: PoiInterests,
	visit_policy: PoiVisitPolicy,
}

impl PendingSelectionKey {
	fn matches(
		&self,
		user: &JourneyingIntelligenceUser,
		at: Vec3,
		interests: &PoiInterests,
	) -> bool {
		let minimum = user.min_tile_distance.clamp(1, MAX_TILE_DISTANCE);
		self.origin == tile_of(at, user.tile_size)
			&& self.tile_size == normalized_tile_size(user.tile_size)
			&& self.minimum == minimum
			&& self.maximum == user.max_tile_distance.clamp(minimum, MAX_TILE_DISTANCE)
			&& self.probes == user.tile_probes.max(1)
			&& self.seed == user.seed
			&& self.step == user.selection_step
			&& self.interests == *interests
			&& self.visit_policy == user.visit_policy
	}

	fn new(user: &JourneyingIntelligenceUser, at: Vec3, interests: &PoiInterests) -> Self {
		let minimum = user.min_tile_distance.clamp(1, MAX_TILE_DISTANCE);
		Self {
			origin: tile_of(at, user.tile_size),
			tile_size: normalized_tile_size(user.tile_size),
			minimum,
			maximum: user.max_tile_distance.clamp(minimum, MAX_TILE_DISTANCE),
			probes: user.tile_probes.max(1),
			seed: user.seed,
			step: user.selection_step,
			interests: interests.clone(),
			visit_policy: user.visit_policy,
		}
	}
}

/// Per-actor deterministic probe progress held by the selection system.
pub struct PendingTileSelection {
	key: PendingSelectionKey,
	at: Vec3,
	probe: usize,
	retry_at: f32,
	request: Option<Box<dyn DistantTileRequest>>,
	lookup: Option<DistantTilePoiLookup>,
}

impl PendingTileSelection {
	fn matches(
		&self,
		user: &JourneyingIntelligenceUser,
		at: Vec3,
		interests: &PoiInterests,
		lookup: Option<&DistantTilePoiLookup>,
	) -> bool {
		self.key.matches(user, at, interests)
			&& (at.y - self.at.y).abs() < 1_000.0
			&& match (self.lookup.as_ref(), lookup) {
				(Some(previous), Some(current)) => std::sync::Arc::ptr_eq(&previous.0, &current.0),
				(None, None) => true,
				_ => false,
			}
	}
}

type JourneyingSelection<'a> = (
	Entity,
	&'a GlobalTransform,
	&'a mut JourneyingIntelligenceUser,
	&'a mut PoiIntelligenceUser,
	&'a mut PoiKnowledge,
	&'a mut PoiVisitState,
	Option<&'a mut PoiGoalState>,
);
type JourneyingFilter = (Without<PoiGoal>, Without<MeanderingIntelligenceUser>);

struct TileSelection<'a> {
	at: Vec3,
	user_entity: Entity,
	storage: Option<&'a HcsgStorage>,
	demand: Option<&'a HcsgDemand>,
	lookup: Option<&'a DistantTilePoiLookup>,
	learner: &'a mut PoiIntelligenceUser,
	knowledge: &'a mut PoiKnowledge,
	visits: &'a mut PoiVisitState,
	now: f32,
}

pub struct JourneyingIntelligencePlugin;

impl Plugin for JourneyingIntelligencePlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			(record_journeying_completions, select_journeying_goals)
				.chain()
				.in_set(PoiSystems::Select)
				.after(lod::hcsg::HcsgSessionRestarted),
		);
	}
}

pub fn record_journeying_completions(
	time: Res<Time>,
	mut completed: MessageReader<PoiGoalCompleted>,
	mut users: Query<
		&mut PoiVisitState,
		(With<JourneyingIntelligenceUser>, Without<MeanderingIntelligenceUser>),
	>,
) {
	let now = time.elapsed_secs();
	for event in completed.read() {
		if let Ok(mut visits) = users.get_mut(event.user) {
			visits.complete(event.target, now);
		}
	}
}

pub fn select_journeying_goals(
	time: Res<Time>,
	storage: Option<Res<HcsgStorage>>,
	demand: Option<Res<HcsgDemand>>,
	lookup: Option<Res<DistantTilePoiLookup>>,
	mut users: Query<JourneyingSelection<'_>, JourneyingFilter>,
	mut commands: Commands,
	mut tile_matches: Local<Vec<PoiEntry>>,
	mut pending: Local<HashMap<Entity, Option<PendingTileSelection>>>,
) {
	let now = time.elapsed_secs();
	// The selection query excludes actors with a goal, meanderers and departed
	// actors. Drop their tickets every frame, even before the next due selection.
	pending.retain(|entity, _| users.get(*entity).is_ok());
	for (entity, transform, mut journeying, mut learner, mut knowledge, mut visits, mut state) in
		&mut users
	{
		if let Some(Some(progress)) = pending.get(&entity) {
			if !progress.matches(
				&journeying,
				transform.translation(),
				&learner.interests,
				lookup.as_deref(),
			) {
				pending.remove(&entity);
			}
		}
		if now < journeying.next_selection_at && pending.get(&entity).is_none_or(Option::is_none) {
			continue;
		}
		journeying.next_selection_at = now + journeying.selection_interval.max(0.05);
		if let PoiVisitPolicy::Cycle { roster_size, .. } = journeying.visit_policy {
			visits.reconcile_cycle(roster_size, |id| {
				knowledge.get(id).is_some_and(|known| learner.interests.contains(known.kind))
			});
		}
		let selected = if journeying.cycle_is_full(&visits) {
			pending.remove(&entity);
			let all: Vec<_> = knowledge.matching(&learner.interests).copied().collect();
			choose_poi(&mut visits, journeying.visit_policy, &all, now, |known| {
				base_score(known, &learner.interests)
			})
		} else {
			journeying.select_from_distant_tile(
				TileSelection {
					at: transform.translation(),
					user_entity: entity,
					storage: storage.as_deref(),
					demand: demand.as_deref(),
					lookup: lookup.as_deref(),
					learner: &mut learner,
					knowledge: &mut knowledge,
					visits: &mut visits,
					now,
				},
				&mut tile_matches,
				pending.entry(entity).or_default(),
			)
		};
		let Some(id) = selected else {
			continue;
		};
		let Some(known) = knowledge.get(id).copied() else {
			continue;
		};
		knowledge.include_source(id, PoiSource::OBJECTIVE);
		begin_poi_goal(
			&mut commands,
			entity,
			known,
			now,
			journeying.linger_secs,
			state.as_deref_mut(),
			0,
		);
	}
}

fn base_score(known: KnownPoi, interests: &poi_intelligence::PoiInterests) -> f32 {
	interests.weight(known.kind).unwrap_or(0.0) * known.salience * known.confidence
}

fn tile_of(position: Vec3, tile_size: f32) -> IVec2 {
	let size = normalized_tile_size(tile_size);
	(position.xz() / size).floor().as_ivec2()
}

fn tile_center(tile: IVec2, tile_size: f32) -> Vec2 {
	(tile.as_vec2() + Vec2::splat(0.5)) * normalized_tile_size(tile_size)
}

fn normalized_tile_size(tile_size: f32) -> f32 {
	if tile_size.is_finite() {
		tile_size.clamp(1.0, MAX_TILE_SIZE)
	} else {
		DEFAULT_TILE_SIZE
	}
}

fn square_ring_offset(distance: i32, index: u64) -> IVec2 {
	let side = distance as u64 * 2;
	let index = index % (side * 4);
	let along = (index % side) as i32;
	match index / side {
		0 => IVec2::new(-distance + along, -distance),
		1 => IVec2::new(distance, -distance + along),
		2 => IVec2::new(distance - along, distance),
		_ => IVec2::new(-distance, distance - along),
	}
}

fn splitmix64(mut value: u64) -> u64 {
	value = value.wrapping_add(0x9e37_79b9_7f4a_7c15);
	value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
	value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
	value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
	use super::*;
	use lod::hcsg::universal_bounds;
	use poi_intelligence::{PoiCatalog, PoiEntry, PoiId, PoiInterests, PoiKind, PoiSupport};

	#[test]
	fn distant_tiles_are_outside_the_local_origin() -> anyhow::Result<()> {
		let user =
			JourneyingIntelligenceUser { min_tile_distance: 3, max_tile_distance: 3, ..default() };
		for step in 0..32 {
			let tile = user.distant_tile(Vec3::ZERO, step);
			assert!(tile != IVec2::ZERO);
			assert_eq!(tile.x.abs().max(tile.y.abs()), 3);
		}
		Ok(())
	}

	#[test]
	fn tile_math_floors_negative_coordinates() -> anyhow::Result<()> {
		assert_eq!(tile_of(Vec3::new(-1.0, 0.0, -257.0), 256.0), IVec2::new(-1, -2));
		Ok(())
	}

	#[test]
	fn distant_tile_probe_learns_from_the_scheme() -> anyhow::Result<()> {
		let kind = PoiKind::new("test/destination");
		let mut world = World::new();
		let user_entity = world.spawn_empty().id();
		let mut journeying = JourneyingIntelligenceUser {
			min_tile_distance: 3,
			max_tile_distance: 3,
			tile_probes: 1,
			..default()
		};
		let tile = journeying.distant_tile(Vec3::ZERO, 0);
		let center = tile_center(tile, journeying.tile_size);
		let storage = HcsgStorage::default();
		storage.seed(
			PoiCatalog {
				entries: vec![PoiEntry {
					id: PoiId(7),
					kind,
					position: Vec3::new(center.x, 0.0, center.y),
					arrival_radius: 1.0,
					support: PoiSupport::Terrain,
					salience: 1.0,
					local: true,
					global: false,
				}],
			},
			universal_bounds(),
		);
		let lookup = DistantTilePoiLookup::default();
		let demand = HcsgDemand::default();
		let _worker = lod::hcsg::spawn_worker(storage.clone(), demand.clone())?;
		let mut learner = PoiIntelligenceUser::new(PoiInterests::one(kind));
		let mut knowledge = PoiKnowledge::default();
		let mut visits = PoiVisitState::default();
		let mut tile_matches = Vec::new();
		let mut pending = None;
		let mut selected = journeying.select_from_distant_tile(
			TileSelection {
				at: Vec3::ZERO,
				user_entity,
				storage: Some(&storage),
				demand: Some(&demand),
				lookup: Some(&lookup),
				learner: &mut learner,
				knowledge: &mut knowledge,
				visits: &mut visits,
				now: 0.0,
			},
			&mut tile_matches,
			&mut pending,
		);
		for _ in 0..10 {
			if selected.is_some() {
				break;
			}
			assert!(demand.wait_idle(std::time::Duration::from_secs(5)));
			selected = journeying.select_from_distant_tile(
				TileSelection {
					at: Vec3::ZERO,
					user_entity,
					storage: Some(&storage),
					demand: Some(&demand),
					lookup: Some(&lookup),
					learner: &mut learner,
					knowledge: &mut knowledge,
					visits: &mut visits,
					now: 0.0,
				},
				&mut tile_matches,
				&mut pending,
			);
		}
		assert_eq!(selected, Some(PoiId(7)));
		assert!(knowledge.get(PoiId(7)).is_some());
		Ok(())
	}
}

#[cfg(test)]
mod async_tests;
