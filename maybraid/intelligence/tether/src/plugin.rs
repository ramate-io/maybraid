use bevy::prelude::*;
use movement_intelligence::{
	MovementIntelligence, MovementIntelligenceSystems, MovementLocation, MovementObjective,
	ReplanMovement, ReplanThreshold,
};
use routing_intelligence::{RoutingIntelligenceUser, RoutingSystems};

use crate::memory::TetherMemory;
use crate::objective::TetherObjective;
use crate::user::{Tether, TetherAction, TetherIntelligenceUser};

const REPLAN_THRESHOLD: ReplanThreshold =
	ReplanThreshold { refresh_distance: 1.2, check_radius: true, check_vantage_weights: false };

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TetherSystems {
	Write,
	/// Reclaim stolen movement without a full evaluate. Apps can run this on
	/// the pursuit cadence so an intervening firearm tick cannot walk the leash.
	Reclaim,
}

pub struct TetherPlugin;

impl Plugin for TetherPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			(
				TetherSystems::Write
					.before(RoutingSystems::Plan)
					.before(MovementIntelligenceSystems::Replan),
				TetherSystems::Reclaim
					.after(TetherSystems::Write)
					.before(RoutingSystems::Plan)
					.before(MovementIntelligenceSystems::Replan),
			),
		)
		.add_systems(Update, write_tether_objectives.in_set(TetherSystems::Write))
		.add_systems(Update, reclaim_stolen_tether_objectives.in_set(TetherSystems::Reclaim));
	}
}

pub fn write_tether_objectives(
	time: Res<Time>,
	mut users: Query<(
		Entity,
		&Transform,
		Option<&GlobalTransform>,
		&mut TetherIntelligenceUser,
		&mut TetherMemory,
		&mut MovementIntelligence,
		Option<&mut RoutingIntelligenceUser>,
	)>,
	anchors: Query<(&Transform, Option<&GlobalTransform>), With<Tether>>,
	mut commands: Commands,
) {
	let now = time.elapsed_secs();
	for (entity, transform, global, mut tether, mut memory, mut movement, routing) in &mut users {
		let dt = if memory.last_checked_at > 0.0 {
			(now - memory.last_checked_at).max(0.0)
		} else {
			time.delta_secs()
		};
		let subject_id = tether.objective.subject();
		let Ok((anchor, anchor_global)) = anchors.get(subject_id) else {
			continue;
		};
		let from = pose_translation(transform, global);
		let subject = pose_translation(anchor, anchor_global);
		let action = tether.evaluate(&mut memory, from, subject, dt, now);
		let action = reclaim_stolen_objective(
			action,
			tether.enabled,
			memory.satisfied,
			memory.remaining,
			tether.horizon,
			tether.objective,
			from,
			subject,
			movement.objective,
		);
		apply_action(
			entity,
			from,
			cleared_local(&tether, from, subject, action),
			&mut movement,
			routing,
			&mut commands,
		);
	}
}

fn pose_translation(transform: &Transform, global: Option<&GlobalTransform>) -> Vec3 {
	global.map(|global| global.translation()).unwrap_or(transform.translation)
}

fn cleared_local(
	tether: &TetherIntelligenceUser,
	from: Vec3,
	subject: Vec3,
	action: TetherAction,
) -> TetherAction {
	match action {
		TetherAction::Local(_) => TetherAction::Local(tether.local_objective(from, subject)),
		other => other,
	}
}

/// Recompute remaining from live poses and reclaim a stolen objective.
///
/// `evaluate` is cadence-owned by the app. Firearm movement can run on a
/// shorter timer; this system restores tether priority on those ticks without
/// waiting for the next write.
pub fn reclaim_stolen_tether_objectives(
	mut users: Query<(
		Entity,
		&Transform,
		Option<&GlobalTransform>,
		&TetherIntelligenceUser,
		&mut MovementIntelligence,
		Option<&mut RoutingIntelligenceUser>,
	)>,
	anchors: Query<(&Transform, Option<&GlobalTransform>), With<Tether>>,
	mut commands: Commands,
) {
	for (entity, transform, global, tether, mut movement, routing) in &mut users {
		if !tether.enabled {
			continue;
		}
		let Ok((anchor, anchor_global)) = anchors.get(tether.objective.subject()) else {
			continue;
		};
		let from = pose_translation(transform, global);
		let subject = pose_translation(anchor, anchor_global);
		let remaining = tether.remaining_work(from, subject);
		let action = reclaim_stolen_objective(
			TetherAction::None,
			tether.enabled,
			remaining <= 0.0,
			remaining,
			tether.horizon,
			tether.objective,
			from,
			subject,
			movement.objective,
		);
		apply_action(
			entity,
			from,
			cleared_local(tether, from, subject, action),
			&mut movement,
			routing,
			&mut commands,
		);
	}
}

/// `evaluate` yields `None` while remaining work is shrinking so a lone tether
/// does not replan every frame. Firearm (and other) brains write earlier in the
/// same Update, so that yield would leave their vantage in force past the leash.
/// Reclaim when the grant is on, unsatisfied remaining is still > 0, and the
/// live objective is no longer the tether's. A retracted grant must not spend
/// stale [`TetherMemory`] to overwrite flee or combat movement.
pub(crate) fn reclaim_stolen_objective(
	action: TetherAction,
	enabled: bool,
	satisfied: bool,
	remaining: f32,
	horizon: f32,
	objective: TetherObjective,
	from: Vec3,
	subject: Vec3,
	current: MovementObjective,
) -> TetherAction {
	if !enabled || !matches!(action, TetherAction::None) || satisfied || remaining <= 0.0 {
		return action;
	}
	let next = if remaining <= horizon {
		TetherAction::Local(objective.movement_objective(from, subject))
	} else {
		TetherAction::Route(objective.route_point(from, subject))
	};
	let candidate = match next {
		TetherAction::Local(objective) => objective,
		TetherAction::Route(point) => MovementObjective::Reach(MovementLocation::new(
			point,
			current.location().radius.max(0.4),
		)),
		TetherAction::None | TetherAction::Hold => return action,
	};
	if current.needs_replan(candidate, REPLAN_THRESHOLD) {
		next
	} else {
		action
	}
}

fn apply_action(
	entity: Entity,
	at: Vec3,
	action: TetherAction,
	movement: &mut MovementIntelligence,
	mut routing: Option<Mut<'_, RoutingIntelligenceUser>>,
	commands: &mut Commands,
) {
	match action {
		TetherAction::None => {}
		TetherAction::Hold => {
			if let Some(routing) = routing.as_deref_mut() {
				routing.clear_destination();
			}
			movement.hold_at(at);
			commands.entity(entity).remove::<ReplanMovement>();
		}
		TetherAction::Local(next) => {
			if let Some(routing) = routing.as_deref_mut() {
				routing.clear_destination();
			}
			if movement.objective.needs_replan(next, REPLAN_THRESHOLD) {
				movement.objective = next;
				commands.entity(entity).insert(ReplanMovement);
			}
		}
		TetherAction::Route(point) => {
			if let Some(routing) = routing.as_deref_mut() {
				routing.set_destination(point);
			} else {
				let next = MovementObjective::Reach(MovementLocation::new(
					point,
					movement.ability.agent_radius,
				));
				if movement.objective.needs_replan(next, REPLAN_THRESHOLD) {
					movement.objective = next;
					commands.entity(entity).insert(ReplanMovement);
				}
			}
		}
	}
}
