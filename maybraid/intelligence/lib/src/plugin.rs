//! Generic plugin over a [`MovementIntelligenceSurface`] [`SystemParam`].

use std::marker::PhantomData;

use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::prelude::*;
use player::MoveWish;

use crate::ability::{Covering, MovementBody};
use crate::configure_movement_intelligence_sets;
use crate::location::MovementLocation;
use crate::step::MovementDrive;
use crate::surface::{MovementIntelligenceLimits, MovementIntelligenceSurface};
use crate::user::{MovementIntelligence, ReplanMovement};
use crate::MovementIntelligenceSystems;

/// Registers replan + drive for surface `S` and interaction / ability types `I`, `A`.
pub struct MovementIntelligencePlugin<S, I = crate::MovementStep, A = crate::MovementAbility>
where
	S: SystemParam + 'static,
{
	_marker: PhantomData<fn() -> (S, I, A)>,
}

impl<S, I, A> Default for MovementIntelligencePlugin<S, I, A>
where
	S: SystemParam + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<S, I, A> Plugin for MovementIntelligencePlugin<S, I, A>
where
	S: SystemParam + 'static,
	for<'w, 's> S::Item<'w, 's>: MovementIntelligenceSurface<I, A>,
	I: MovementDrive + Clone + Send + Sync + 'static,
	A: MovementBody + Covering + Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		configure_movement_intelligence_sets(app);
		app.init_resource::<MovementIntelligenceLimits>()
			.add_systems(
				Update,
				replan_movement::<S, I, A>.in_set(MovementIntelligenceSystems::Replan),
			)
			.add_systems(Update, drive_movement::<I, A>.in_set(MovementIntelligenceSystems::Drive));
	}
}

pub fn replan_movement<S, I, A>(
	surface: StaticSystemParam<S>,
	limits: Res<MovementIntelligenceLimits>,
	mut movers: Query<(Entity, &Transform, &mut MovementIntelligence<I, A>), With<ReplanMovement>>,
	mut commands: Commands,
) where
	S: SystemParam + 'static,
	for<'w, 's> S::Item<'w, 's>: MovementIntelligenceSurface<I, A>,
	I: Send + Sync + 'static,
	A: MovementBody + Covering + Send + Sync + 'static,
{
	let mut surface = surface.into_inner();
	for (entity, transform, mut brain) in &mut movers {
		let from = MovementLocation::new(transform.translation, brain.ability.agent_radius());
		let exclude = [entity];
		let budget = brain.ability.candidate_budget().clamp_to(limits.max_budget);
		let objective = brain.objective;
		let candidates =
			surface.recommend_candidates(from, &exclude, &brain.ability, objective, budget);
		let best = candidates
			.into_iter()
			.min_by(|a, b| brain.score_candidate(a).total_cmp(&brain.score_candidate(b)));
		if let Some(candidate) = best {
			brain.adopt_plan(candidate.steps);
		} else {
			brain.adopt_plan(Vec::new());
		}
		commands.entity(entity).remove::<ReplanMovement>();
	}
}

pub fn drive_movement<I, A>(
	time: Res<Time>,
	mut movers: Query<(Entity, &Transform, &mut MovementIntelligence<I, A>, &mut MoveWish)>,
	mut commands: Commands,
) where
	I: MovementDrive + Send + Sync + 'static,
	A: Send + Sync + 'static,
{
	let dt = time.delta_secs();
	for (entity, transform, mut brain, mut wish) in &mut movers {
		loop {
			if brain.at_plan_end() {
				wish.0 = Vec3::ZERO;
				break;
			}
			let Some(target) = brain.plan.get(brain.cursor).and_then(I::drive_target) else {
				brain.cursor += 1;
				continue;
			};
			if target.contains_xz(transform.translation) {
				brain.cursor += 1;
				brain.stuck_seconds = 0.0;
				brain.last_goal_distance = f32::MAX;
				continue;
			}
			let dist = target.xz_distance(transform.translation);
			if dist + 0.08 < brain.last_goal_distance {
				brain.last_goal_distance = dist;
				brain.stuck_seconds = 0.0;
			} else {
				brain.stuck_seconds += dt;
				if brain.stuck_seconds >= brain.settings.stuck_timeout {
					brain.stuck_seconds = 0.0;
					commands.entity(entity).insert(ReplanMovement);
				}
			}
			let delta = Vec3::new(
				target.point.x - transform.translation.x,
				0.0,
				target.point.z - transform.translation.z,
			);
			wish.0 = delta.normalize_or_zero();
			break;
		}
	}
}
