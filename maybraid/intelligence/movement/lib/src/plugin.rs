//! Generic plugin over a [`MovementIntelligenceSurface`] [`SystemParam`].

use std::marker::PhantomData;

use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::prelude::*;
use player::MoveWish;

use crate::ability::MovementSheet;
use crate::configure_movement_intelligence_sets;
use crate::location::MovementLocation;
use crate::step::MovementDrive;
use crate::surface::{MovementIntelligenceLimits, MovementIntelligenceSurface, WalkProbeBudget};
use crate::user::{MovementDriveResult, MovementIntelligence, ReplanMovement};
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
	A: MovementSheet + Send + Sync + 'static,
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
	A: MovementSheet + Send + Sync + 'static,
{
	let mut surface = surface.into_inner();
	let mut remaining = limits.max_replans_per_frame;
	let mut probes = WalkProbeBudget::new(limits.max_walk_probes_per_frame);
	for (entity, transform, mut brain) in &mut movers {
		if remaining == 0 || probes.is_exhausted() {
			break;
		}
		remaining -= 1;
		let from = MovementLocation::new(transform.translation, brain.ability.agent_radius());
		let exclude = [entity];
		let objective = brain.objective;
		let budget = brain
			.ability
			.candidate_budget()
			.clamp_to(limits.max_budget)
			.lod_for(transform.translation, objective);
		let candidates = surface.recommend_candidates_budgeted(
			from,
			&exclude,
			&brain.ability,
			objective,
			budget,
			&mut probes,
		);
		if probes.is_starved() && candidates.is_empty() {
			continue;
		}
		if let Some(candidate) = brain.pick_best_candidate(candidates) {
			brain.adopt_plan(candidate.steps);
		} else {
			brain.note_replan_failed();
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
		match brain.drive(dt, transform.translation) {
			MovementDriveResult::Wish(dir) => {
				// Motor unstick may override this wish later this frame.
				wish.0 = dir;
			}
			MovementDriveResult::Stuck { wish: dir } => {
				wish.0 = dir;
				if dir.length_squared() < 1e-6 {
					commands.entity(entity).insert(ReplanMovement);
				}
			}
			MovementDriveResult::Hold => wish.0 = Vec3::ZERO,
			MovementDriveResult::Retry => {
				wish.0 = Vec3::ZERO;
				commands.entity(entity).insert(ReplanMovement);
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::SystemParam;

	use crate::objective::MovementObjective;
	use crate::surface::CandidateBudget;
	use crate::{MovementAbility, MovementStep};

	use super::*;

	#[derive(Resource, Default)]
	struct ReplanCalls(usize);

	#[derive(SystemParam)]
	struct CountingSurface<'w> {
		calls: ResMut<'w, ReplanCalls>,
	}

	impl<I, A> MovementIntelligenceSurface<I, A> for CountingSurface<'_> {
		fn recommend_candidates(
			&mut self,
			_from: MovementLocation,
			_exclude: &[Entity],
			_ability: &A,
			_objective: MovementObjective,
			_budget: CandidateBudget,
		) -> Vec<crate::MovementCandidate<I>> {
			self.calls.0 += 1;
			Vec::new()
		}

		fn recommend_candidates_budgeted(
			&mut self,
			from: MovementLocation,
			exclude: &[Entity],
			ability: &A,
			objective: MovementObjective,
			budget: CandidateBudget,
			probes: &mut WalkProbeBudget,
		) -> Vec<crate::MovementCandidate<I>> {
			for _ in 0..budget.max_candidates.max(1) {
				if !probes.take() {
					break;
				}
			}
			self.recommend_candidates(from, exclude, ability, objective, budget)
		}
	}

	fn spawn_pending(world: &mut World, count: usize) {
		let goal = MovementObjective::Reach(MovementLocation::new(Vec3::X * 4.0, 0.5));
		for index in 0..count {
			world.spawn((
				Transform::from_xyz(index as f32, 0.0, 0.0),
				MovementIntelligence::<MovementStep, MovementAbility>::new(goal),
				ReplanMovement,
			));
		}
	}

	fn pending_replans(world: &mut World) -> usize {
		world.query::<&ReplanMovement>().iter(world).count()
	}

	#[test]
	fn replan_drains_a_frame_budget_and_leaves_the_rest() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(MovementIntelligenceLimits { max_replans_per_frame: 2, ..default() })
			.init_resource::<ReplanCalls>()
			.add_systems(Update, replan_movement::<CountingSurface, MovementStep, MovementAbility>);
		spawn_pending(app.world_mut(), 5);

		app.update();
		anyhow::ensure!(app.world().resource::<ReplanCalls>().0 == 2);
		anyhow::ensure!(pending_replans(app.world_mut()) == 3);

		app.update();
		anyhow::ensure!(app.world().resource::<ReplanCalls>().0 == 4);
		anyhow::ensure!(pending_replans(app.world_mut()) == 1);
		Ok(())
	}

	#[test]
	fn walk_probe_cap_stops_starting_new_replans() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(MovementIntelligenceLimits {
				max_replans_per_frame: 8,
				max_walk_probes_per_frame: 2,
				..default()
			})
			.init_resource::<ReplanCalls>()
			.add_systems(Update, replan_movement::<CountingSurface, MovementStep, MovementAbility>);
		spawn_pending(app.world_mut(), 5);

		app.update();
		anyhow::ensure!(app.world().resource::<ReplanCalls>().0 == 2);
		anyhow::ensure!(pending_replans(app.world_mut()) == 3);
		Ok(())
	}

	#[test]
	fn starved_vantage_keeps_the_marker() -> anyhow::Result<()> {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(MovementIntelligenceLimits {
				max_replans_per_frame: 8,
				max_walk_probes_per_frame: 3,
				..default()
			})
			.init_resource::<ReplanCalls>()
			.add_systems(Update, replan_movement::<CountingSurface, MovementStep, MovementAbility>);
		let goal = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::X * 4.0, 1.0),
			hide_weight: 1.0,
			sightline_weight: 1.0,
		};
		app.world_mut().spawn((
			Transform::default(),
			MovementIntelligence::<MovementStep, MovementAbility>::new(goal),
			ReplanMovement,
		));

		app.update();
		anyhow::ensure!(app.world().resource::<ReplanCalls>().0 == 1);
		anyhow::ensure!(pending_replans(app.world_mut()) == 1);
		Ok(())
	}
}
