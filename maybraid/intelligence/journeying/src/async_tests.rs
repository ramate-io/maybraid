use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::*;
use lod::hcsg::Busy;
use poi_intelligence::{DistantTilePois, PoiId, PoiKind, PoiSupport};

const KIND: PoiKind = PoiKind::new("test/async-journey");

#[derive(Clone, Default)]
struct ControlledLookup {
	status: Arc<AtomicUsize>,
	created: Arc<AtomicUsize>,
	dropped: Arc<AtomicUsize>,
	work_polls: Arc<AtomicUsize>,
}

struct ControlledRequest {
	query: PoiTileQuery,
	control: ControlledLookup,
}

impl Drop for ControlledRequest {
	fn drop(&mut self) {
		self.control.dropped.fetch_add(1, Ordering::Relaxed);
	}
}

fn entry(query: &PoiTileQuery) -> PoiEntry {
	let center = tile_center(query.tile, query.tile_size);
	PoiEntry {
		id: PoiId(71),
		kind: KIND,
		position: Vec3::new(center.x, 4.0, center.y),
		arrival_radius: 3.0,
		support: PoiSupport::Floor,
		salience: 1.4,
		local: true,
		global: false,
	}
}

impl DistantTileRequest for ControlledRequest {
	fn poll(
		&mut self,
		_: &HcsgStorage,
		_: &HcsgDemand,
		out: &mut Vec<PoiEntry>,
		allow_request: bool,
	) -> Result<PoiTilePoll, Busy> {
		out.clear();
		if !allow_request {
			return Ok(PoiTilePoll::Pending);
		}
		self.control.work_polls.fetch_add(1, Ordering::Relaxed);
		Ok(match self.control.status.load(Ordering::Relaxed) {
			0 => PoiTilePoll::Pending,
			1 => return Err(Busy),
			2 => PoiTilePoll::ReadyEmpty,
			3 => {
				out.push(entry(&self.query));
				PoiTilePoll::Ready
			}
			4 => PoiTilePoll::Failed,
			5 => PoiTilePoll::Stale,
			7 => {
				out.push(entry(&self.query));
				PoiTilePoll::Pending
			}
			_ => PoiTilePoll::Capacity,
		})
	}
}

impl DistantTilePois for ControlledLookup {
	fn request_in_xz_tile(&self, query: PoiTileQuery) -> Option<Box<dyn DistantTileRequest>> {
		self.created.fetch_add(1, Ordering::Relaxed);
		Some(Box::new(ControlledRequest { query, control: self.clone() }))
	}
}

struct Harness {
	storage: HcsgStorage,
	demand: HcsgDemand,
	lookup: DistantTilePoiLookup,
	control: ControlledLookup,
	user: JourneyingIntelligenceUser,
	learner: PoiIntelligenceUser,
	knowledge: PoiKnowledge,
	visits: PoiVisitState,
	entity: Entity,
	pending: Option<PendingTileSelection>,
}

impl Harness {
	fn new() -> Self {
		let control = ControlledLookup::default();
		let entity = World::new().spawn_empty().id();
		Self {
			storage: HcsgStorage::default(),
			demand: HcsgDemand::default(),
			lookup: DistantTilePoiLookup::new(control.clone()),
			control,
			user: JourneyingIntelligenceUser { tile_probes: 1, ..default() },
			learner: PoiIntelligenceUser::new(PoiInterests::one(KIND)),
			knowledge: PoiKnowledge::default(),
			visits: PoiVisitState::default(),
			entity,
			pending: None,
		}
	}
	fn select(&mut self, at: Vec3) -> Option<PoiId> {
		self.select_at_time(at, 0.0)
	}
	fn select_at_time(&mut self, at: Vec3, now: f32) -> Option<PoiId> {
		self.user.select_from_distant_tile(
			TileSelection {
				at,
				user_entity: self.entity,
				storage: Some(&self.storage),
				demand: Some(&self.demand),
				lookup: Some(&self.lookup),
				learner: &mut self.learner,
				knowledge: &mut self.knowledge,
				visits: &mut self.visits,
				now,
			},
			&mut Vec::new(),
			&mut self.pending,
		)
	}
}

#[test]
fn pending_busy_and_capacity_preserve_probe_step_credit_and_empty_cache() {
	let mut h = Harness::new();
	for status in [0, 1, 6, 0] {
		h.control.status.store(status, Ordering::Relaxed);
		assert_eq!(h.select(Vec3::ZERO), None);
		assert_eq!(h.user.selection_step, 0);
		assert!(h.user.empty_tiles.is_empty());
		assert_eq!(h.pending.as_ref().map(|p| p.probe), Some(0));
		assert!(h.learner.clone().try_take_learning_credit());
	}
	assert_eq!(h.control.created.load(Ordering::Relaxed), 1);
	h.control.status.store(3, Ordering::Relaxed);
	assert_eq!(h.select(Vec3::ZERO), Some(PoiId(71)));
	assert_eq!(h.user.selection_step, 1);
	assert!(!h.learner.try_take_learning_credit());
	assert!(h.pending.is_none());
}

#[test]
fn failure_and_stale_retry_same_probe_and_empty_requires_completion() {
	let mut h = Harness::new();
	for (now, status) in [(0.0, 4), (1.0, 5), (2.0, 0)] {
		h.control.status.store(status, Ordering::Relaxed);
		assert_eq!(h.select_at_time(Vec3::ZERO, now), None);
		assert_eq!(h.user.selection_step, 0);
		assert!(h.user.empty_tiles.is_empty());
	}
	h.control.status.store(2, Ordering::Relaxed);
	assert_eq!(h.select_at_time(Vec3::ZERO, 3.0), None);
	assert_eq!(h.user.selection_step, 1);
	assert_eq!(h.user.empty_tiles.len(), 1);
	assert!(h.learner.try_take_learning_credit());
}

#[test]
fn pose_drift_keeps_request_but_origin_and_interest_changes_cancel() {
	let mut h = Harness::new();
	assert_eq!(h.select(Vec3::ZERO), None);
	assert_eq!(h.select(Vec3::new(0.1, 0.01, 0.1)), None);
	assert_eq!(h.control.created.load(Ordering::Relaxed), 1);
	assert_eq!(h.select(Vec3::new(300.0, 0.0, 0.0)), None);
	assert_eq!(h.control.created.load(Ordering::Relaxed), 2);
	assert_eq!(h.control.dropped.load(Ordering::Relaxed), 1);
	h.learner.interests = PoiInterests::one(PoiKind::new("test/changed"));
	assert_eq!(h.select(Vec3::new(300.0, 0.0, 0.0)), None);
	assert_eq!(h.control.created.load(Ordering::Relaxed), 3);
	assert_eq!(h.user.selection_step, 0);
}

#[test]
fn retained_knowledge_can_select_without_generation_backend() -> anyhow::Result<()> {
	for visit_policy in [
		PoiVisitPolicy::default(),
		PoiVisitPolicy::Cycle { roster_size: 2, reshuffle_each_cycle: false },
	] {
		let mut h = Harness::new();
		h.user.visit_policy = visit_policy;
		let query = PoiTileQuery {
			tile: h.user.distant_tile(Vec3::ZERO, 0),
			tile_size: h.user.tile_size,
			center_y: 0.0,
			interests: h.learner.interests.clone(),
		};
		h.knowledge
			.observe(entry(&query).observation(h.entity, PoiSource::LOCAL_SCAN), 0.0)
			.ok_or_else(|| anyhow::anyhow!("valid known POI"))?;
		let selected = h.user.select_from_distant_tile(
			TileSelection {
				at: Vec3::ZERO,
				user_entity: h.entity,
				storage: None,
				demand: None,
				lookup: None,
				learner: &mut h.learner,
				knowledge: &mut h.knowledge,
				visits: &mut h.visits,
				now: 0.0,
			},
			&mut Vec::new(),
			&mut h.pending,
		);
		assert_eq!(selected, Some(PoiId(71)));
		assert!(h.user.empty_tiles.is_empty());
		assert!(h.learner.try_take_learning_credit());
	}
	Ok(())
}

#[test]
fn external_goal_and_departure_release_pending_requests_before_next_interval() {
	let control = ControlledLookup::default();
	let mut app = App::new();
	app.init_resource::<Time>()
		.init_resource::<HcsgStorage>()
		.init_resource::<HcsgDemand>()
		.insert_resource(DistantTilePoiLookup::new(control.clone()))
		.add_systems(Update, select_journeying_goals);
	let user = app
		.world_mut()
		.spawn((
			GlobalTransform::default(),
			JourneyingIntelligenceUser { selection_interval: 100.0, ..default() },
			PoiIntelligenceUser::new(PoiInterests::one(KIND)),
			PoiKnowledge::default(),
			PoiVisitState::default(),
		))
		.id();
	app.update();
	assert_eq!(control.created.load(Ordering::Relaxed), 1);
	app.world_mut().entity_mut(user).insert(PoiGoal::new(
		1,
		PoiId(99),
		None,
		KIND,
		Vec3::ZERO,
		1.0,
		0.0,
		0.0,
	));
	app.update();
	assert_eq!(control.dropped.load(Ordering::Relaxed), 1);
	app.world_mut().entity_mut(user).remove::<PoiGoal>();
	app.world_mut().entity_mut(user).insert(JourneyingIntelligenceUser::default());
	app.update();
	assert_eq!(control.created.load(Ordering::Relaxed), 2);
	app.world_mut().despawn(user);
	app.update();
	assert_eq!(control.dropped.load(Ordering::Relaxed), 2);
}

#[test]
fn pending_request_is_polled_each_frame_before_the_next_selection_interval() {
	let control = ControlledLookup::default();
	let mut app = App::new();
	app.init_resource::<Time>()
		.init_resource::<HcsgStorage>()
		.init_resource::<HcsgDemand>()
		.insert_resource(DistantTilePoiLookup::new(control.clone()))
		.add_message::<PoiGoalCompleted>()
		.add_plugins(JourneyingIntelligencePlugin);
	let user = app
		.world_mut()
		.spawn((
			GlobalTransform::default(),
			JourneyingIntelligenceUser { selection_interval: 100.0, ..default() },
			PoiIntelligenceUser::new(PoiInterests::one(KIND)),
			PoiKnowledge::default(),
			PoiVisitState::default(),
		))
		.id();
	app.update();
	assert_eq!(control.created.load(Ordering::Relaxed), 1);
	assert!(app.world().get::<PoiGoal>(user).is_none());
	control.status.store(3, Ordering::Relaxed);
	// Time is unchanged and the 100-second new-cycle deadline is still ahead.
	app.update();
	assert_eq!(app.world().get::<PoiGoal>(user).map(|goal| goal.target), Some(PoiId(71)));
	assert_eq!(control.dropped.load(Ordering::Relaxed), 1);
}

#[test]
fn failed_request_retries_at_selection_cadence_without_losing_probe() {
	let mut h = Harness::new();
	h.user.selection_interval = 100.0;
	h.control.status.store(4, Ordering::Relaxed);
	assert_eq!(h.select_at_time(Vec3::ZERO, 0.0), None);
	assert_eq!(h.control.created.load(Ordering::Relaxed), 1);
	h.control.status.store(3, Ordering::Relaxed);
	for now in [0.0, 1.0, 99.0] {
		assert_eq!(h.select_at_time(Vec3::ZERO, now), None);
		assert_eq!(h.control.created.load(Ordering::Relaxed), 1);
		assert_eq!(h.user.selection_step, 0);
		assert!(h.user.empty_tiles.is_empty());
		assert!(h.learner.clone().try_take_learning_credit());
	}
	assert_eq!(h.select_at_time(Vec3::ZERO, 100.0), Some(PoiId(71)));
	assert_eq!(h.control.created.load(Ordering::Relaxed), 1);
	assert_eq!(h.control.work_polls.load(Ordering::Relaxed), 2);
	assert_eq!(h.user.selection_step, 1);
}

#[test]
fn available_partial_poi_can_be_selected_without_generation_completion() {
	let mut h = Harness::new();
	h.control.status.store(7, Ordering::Relaxed);
	assert_eq!(h.select(Vec3::ZERO), Some(PoiId(71)));
	assert_eq!(h.user.selection_step, 1);
	assert_eq!(
		h.control.dropped.load(Ordering::Relaxed),
		1,
		"goal selection naturally releases demand"
	);
	assert!(h.user.empty_tiles.is_empty());
}

#[test]
fn repeated_partial_entries_do_not_spend_learning_credit_for_known_ids() -> anyhow::Result<()> {
	let mut h = Harness::new();
	let query = PoiTileQuery {
		tile: h.user.distant_tile(Vec3::ZERO, 0),
		tile_size: h.user.tile_size,
		center_y: 0.0,
		interests: h.learner.interests.clone(),
	};
	h.knowledge
		.observe(entry(&query).observation(h.entity, PoiSource::LOCAL_SCAN), 0.0)
		.ok_or_else(|| anyhow::anyhow!("known POI"))?;
	h.control.status.store(7, Ordering::Relaxed);
	assert_eq!(h.select(Vec3::ZERO), Some(PoiId(71)));
	assert!(h.learner.try_take_learning_credit());
	Ok(())
}
