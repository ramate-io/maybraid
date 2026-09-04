use bevy::prelude::*;
use movement_intelligence::{
	MovementIntelligence, MovementLocation, MovementObjective, ReplanMovement,
};
use routing_intelligence::RoutingIntelligenceUser;

use crate::{KnownPoi, PoiId, PoiKind, PoiKnowledge, PoiRegistry, PoiSource};

type CompletablePoiGoal<'a> = (
	Entity,
	&'a GlobalTransform,
	&'a PoiGoal,
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
	) -> Self {
		Self {
			generation,
			target,
			poi_entity,
			kind,
			location: MovementLocation::new(position, arrival_radius.max(0.0)),
			selected_at,
		}
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
			movement.objective = MovementObjective::Reach(goal.location);
			commands.entity(entity).insert(ReplanMovement);
		}
	}
}

/// Completes entity-bound goals independently of movement's internal plan state.
pub fn complete_poi_goals(
	mut users: Query<CompletablePoiGoal<'_>>,
	mut completed: MessageWriter<PoiGoalCompleted>,
	mut commands: Commands,
) {
	for (entity, transform, goal, mut routing, mut knowledge, mut state) in &mut users {
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
		if !goal.location.contains(transform.translation()) {
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
