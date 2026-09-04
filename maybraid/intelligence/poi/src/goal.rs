use bevy::prelude::*;
use movement_intelligence::{
	MovementIntelligence, MovementLocation, MovementObjective, ReplanMovement,
};
use routing_intelligence::RoutingIntelligenceUser;

use crate::{KnownPoi, PoiId, PoiKind, PoiKnowledge, PoiRegistry, PoiSource};

type CompletablePoiGoal<'a> = (
	Entity,
	&'a GlobalTransform,
	&'a mut PoiGoal,
	Option<&'a mut RoutingIntelligenceUser>,
	Option<&'a mut PoiKnowledge>,
	Option<&'a mut PoiGoalState>,
);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoiGoalStatus {
	Active,
	Completed,
}

/// Persistent status of the newest POI goal, including completed goals.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoiGoalState {
	pub generation: u64,
	pub target: PoiId,
	pub status: PoiGoalStatus,
}

impl PoiGoalState {
	pub fn new(target: PoiId) -> Self {
		Self { generation: 1, target, status: PoiGoalStatus::Active }
	}

	pub fn begin(&mut self, target: PoiId) -> u64 {
		self.generation = self.generation.wrapping_add(1).max(1);
		self.target = target;
		self.status = PoiGoalStatus::Active;
		self.generation
	}
}

/// Entity-bound intent to arrive at one remembered POI.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct PoiGoal {
	pub generation: u64,
	pub target: PoiId,
	pub poi_entity: Option<Entity>,
	pub kind: PoiKind,
	pub location: MovementLocation,
	pub selected_at: f32,
	/// Seconds to remain inside [`Self::location`] after first arrival before
	/// the goal completes. Zero finishes on the first containing sample.
	pub linger_secs: f32,
	arrived_at: Option<f32>,
}

impl PoiGoal {
	pub fn new(
		generation: u64,
		target: PoiId,
		poi_entity: Option<Entity>,
		kind: PoiKind,
		position: Vec3,
		arrival_radius: f32,
		selected_at: f32,
		linger_secs: f32,
	) -> Self {
		Self {
			generation,
			target,
			poi_entity,
			kind,
			location: MovementLocation::new(position, arrival_radius.max(0.0)),
			selected_at,
			linger_secs: linger_secs.max(0.0),
			arrived_at: None,
		}
	}

	pub fn arrived_at(self) -> Option<f32> {
		self.arrived_at
	}

	/// Starts the linger clock on first containment; leaving the disk resets it.
	pub fn linger_ready(&mut self, inside: bool, now: f32) -> bool {
		if !inside {
			self.arrived_at = None;
			return false;
		}
		let linger = self.linger_secs.max(0.0);
		if linger <= 0.0 {
			return true;
		}
		let arrived = *self.arrived_at.get_or_insert(now);
		now - arrived >= linger
	}
}

/// Emitted exactly once when the bound entity reaches its current POI goal.
#[derive(Message, Clone, Copy, Debug, PartialEq)]
pub struct PoiGoalCompleted {
	pub user: Entity,
	pub generation: u64,
	pub target: PoiId,
	pub poi_entity: Option<Entity>,
	pub kind: PoiKind,
	pub location: MovementLocation,
}

/// Keeps entity-backed destinations current while preserving remembered POIs.
pub fn refresh_poi_goals(registry: Res<PoiRegistry>, mut goals: Query<&mut PoiGoal>) {
	for mut goal in &mut goals {
		let Some(record) = registry.get(goal.target) else {
			continue;
		};
		let next = MovementLocation::new(record.position, record.arrival_radius);
		let poi_entity = Some(record.entity);
		if goal.kind != record.kind || goal.location != next || goal.poi_entity != poi_entity {
			goal.kind = record.kind;
			goal.location = next;
			goal.poi_entity = poi_entity;
		}
	}
}

/// Starts a generation-tracked goal and preserves its status after completion.
pub fn begin_poi_goal(
	commands: &mut Commands,
	user: Entity,
	known: KnownPoi,
	now: f32,
	linger_secs: f32,
	state: Option<&mut PoiGoalState>,
) {
	let generation = if let Some(state) = state {
		state.begin(known.id)
	} else {
		commands.entity(user).insert(PoiGoalState::new(known.id));
		1
	};
	commands.entity(user).insert(PoiGoal::new(
		generation,
		known.id,
		known.entity,
		known.kind,
		known.position,
		known.arrival_radius,
		now,
		linger_secs,
	));
}

/// Converts newly selected or moved POI goals into route or movement objectives.
pub fn drive_poi_goals(
	mut users: Query<
		(Entity, &PoiGoal, &mut MovementIntelligence, Option<&mut RoutingIntelligenceUser>),
		Changed<PoiGoal>,
	>,
	mut commands: Commands,
) {
	for (entity, goal, mut movement, routing) in &mut users {
		if let Some(mut routing) = routing {
			routing.set_destination(goal.location.point);
		} else {
			let next = MovementObjective::Reach(goal.location);
			if movement.objective != next {
				movement.objective = next;
				commands.entity(entity).insert(ReplanMovement);
			}
		}
	}
}

/// Completes entity-bound goals independently of movement's internal plan state.
pub fn complete_poi_goals(
	time: Res<Time>,
	mut users: Query<CompletablePoiGoal<'_>>,
	mut completed: MessageWriter<PoiGoalCompleted>,
	mut commands: Commands,
) {
	let now = time.elapsed_secs();
	for (entity, transform, mut goal, mut routing, mut knowledge, mut state) in &mut users {
		if state.as_deref().is_some_and(|state| {
			state.generation != goal.generation
				|| state.target != goal.target
				|| state.status != PoiGoalStatus::Active
		}) {
			if let Some(routing) = routing.as_deref_mut() {
				routing.clear_destination();
			}
			if let Some(knowledge) = knowledge.as_deref_mut() {
				knowledge.remove_source(goal.target, PoiSource::OBJECTIVE);
			}
			commands.entity(entity).remove::<PoiGoal>();
			continue;
		}
		let inside = goal.location.contains(transform.translation());
		if !goal.linger_ready(inside, now) {
			continue;
		}
		if let Some(routing) = routing.as_deref_mut() {
			routing.clear_destination();
		}
		if let Some(knowledge) = knowledge.as_deref_mut() {
			knowledge.remove_source(goal.target, PoiSource::OBJECTIVE);
		}
		if let Some(state) = state.as_deref_mut() {
			state.status = PoiGoalStatus::Completed;
		}
		completed.write(PoiGoalCompleted {
			user: entity,
			generation: goal.generation,
			target: goal.target,
			poi_entity: goal.poi_entity,
			kind: goal.kind,
			location: goal.location,
		});
		commands.entity(entity).remove::<PoiGoal>();
	}
}
