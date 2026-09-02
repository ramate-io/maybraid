//! Compose Avian collider probes with [`CirculationStairwell`] / [`CirculationStorey`].

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use movement_intelligence::{
	CandidateBudget, MovementBody, MovementCandidate, MovementCandidateHints,
	MovementIntelligenceSurface, MovementLocation, MovementObjective, MovementSheet, MovementStep,
};
use movement_intelligence_avian::{AvianColliderPath, AvianMovementSurface, AvianPathHints};

use crate::circulation::{CirculationStairwell, CirculationStorey};

const STOREY_DROP_SLOP: f32 = 0.55;
const STOREY_RECIRCULATION_BASE_COST: f32 = 24.0;
const STOREY_RECIRCULATION_VERTICAL_COST: f32 = 2.0;

/// Avian fine probes plus Richmond storey / stairwell IR.
#[derive(SystemParam)]
pub struct RichmondAvianMovementSurface<'w, 's> {
	avian: AvianMovementSurface<'w, 's>,
	storeys: Query<'w, 's, &'static CirculationStorey>,
	links: Query<'w, 's, &'static CirculationStairwell>,
}

impl RichmondAvianMovementSurface<'_, '_> {
	fn storey_at(&self, p: Vec3) -> Option<u32> {
		self.storeys
			.iter()
			.filter(|storey| storey.contains(p))
			.min_by(|a, b| (p.y - a.floor_y).abs().total_cmp(&(p.y - b.floor_y).abs()))
			.map(|storey| storey.id)
	}

	fn storey(&self, id: u32) -> Option<&CirculationStorey> {
		self.storeys.iter().find(|storey| storey.id == id)
	}

	fn link_on_actor(&self, p: Vec3) -> Option<&CirculationStairwell> {
		self.links.iter().find(|link| {
			let low = link.mouth.y.min(link.landing.y) + 0.08;
			let high = link.mouth.y.max(link.landing.y) - 0.08;
			link.contains_actor(p) && p.y > low && p.y < high
		})
	}

	fn step_link(
		&self,
		from_storey: u32,
		to_storey: u32,
		near: Vec3,
	) -> Option<&CirculationStairwell> {
		self.links
			.iter()
			.filter(|link| {
				(link.from_storey == from_storey && link.to_storey == to_storey)
					|| (link.from_storey == to_storey && link.to_storey == from_storey)
			})
			.min_by(|a, b| a.mouth.xz_distance_to(near).total_cmp(&b.mouth.xz_distance_to(near)))
	}

	fn climb_chain(
		&self,
		from_storey: u32,
		to_storey: u32,
		from: Vec3,
	) -> Option<Vec<&CirculationStairwell>> {
		if from_storey == to_storey {
			return Some(Vec::new());
		}
		let mut chain = Vec::new();
		let mut current = from_storey;
		let mut at = from;
		while current != to_storey {
			let next = if to_storey > current { current + 1 } else { current.saturating_sub(1) };
			let link = self.step_link(current, next, at)?;
			chain.push(link);
			at = if next > current { link.landing } else { link.mouth };
			current = next;
			if chain.len() > 8 {
				return None;
			}
		}
		Some(chain)
	}

	fn candidates_to_stair_goal<A: MovementSheet>(
		&mut self,
		from: MovementLocation,
		from_id: u32,
		exclude: &[Entity],
		ability: &A,
		objective: MovementObjective,
		budget: CandidateBudget,
		goal_link: &CirculationStairwell,
		goal_walk: Vec3,
	) -> Option<Vec<MovementCandidate<MovementStep>>> {
		let from_walk = from.point - Vec3::Y * ability.feet_below_origin();
		let on_link = self.link_on_actor(from_walk).cloned();
		let already_on_goal = on_link.as_ref().is_some_and(|link| link == goal_link);
		let entry_storey = if already_on_goal {
			from_id
		} else {
			let lower_steps = from_id.abs_diff(goal_link.from_storey);
			let upper_steps = from_id.abs_diff(goal_link.to_storey);
			if lower_steps < upper_steps
				|| (lower_steps == upper_steps
					&& goal_walk.distance_squared(goal_link.mouth)
						<= goal_walk.distance_squared(goal_link.landing))
			{
				goal_link.from_storey
			} else {
				goal_link.to_storey
			}
		};
		let chain = if already_on_goal {
			Vec::new()
		} else {
			self.climb_chain(from_id, entry_storey, from.point)?
		};
		let going_up = entry_storey > from_id;
		let approach = chain
			.first()
			.map(|link| if going_up { link.mouth } else { link.landing })
			.unwrap_or_else(|| {
				if entry_storey == goal_link.from_storey {
					goal_link.mouth
				} else {
					goal_link.landing
				}
			});
		let approach_radius = (ability.agent_radius() * 0.75).clamp(0.25, 0.45);
		let approach_loc = lift_location(approach, ability, approach_radius);
		let prefixes = if already_on_goal || on_link.is_some() || approach_loc.contains(from.point)
		{
			vec![(Vec::new(), 0.0, MovementCandidateHints::default())]
		} else {
			let floor_y = self.storey(from_id).map(|storey| storey.floor_y).unwrap_or(from_walk.y);
			mouth_prefixes(&self.avian, from, exclude, ability, approach_loc, budget, floor_y)
		};

		let climb_arrival = (ability.agent_radius() * 0.55).clamp(0.18, 0.3);
		let mut route_steps = Vec::new();
		let mut route_cost = 0.0;
		let mut cursor_walk = if already_on_goal {
			from_walk
		} else if on_link.is_some() {
			from_walk
		} else {
			approach
		};
		let mut cursor = if already_on_goal || on_link.is_some() {
			from.point
		} else {
			lift_location(cursor_walk, ability, climb_arrival).point
		};
		for link in chain {
			for point in link.oriented_polyline(going_up, cursor_walk) {
				let location = lift_location(point, ability, climb_arrival);
				route_cost += location.point.distance(cursor);
				route_steps.push(MovementStep::MoveTo(location));
				cursor = location.point;
				cursor_walk = point;
			}
		}

		if !already_on_goal {
			cursor_walk = if entry_storey == goal_link.from_storey {
				goal_link.mouth
			} else {
				goal_link.landing
			};
		}
		for point in goal_link.route_toward(cursor_walk, goal_walk, objective.location().radius) {
			let location = lift_location(point, ability, climb_arrival);
			route_cost += location.point.distance(cursor);
			route_steps.push(MovementStep::MoveTo(location));
			cursor = location.point;
		}

		Some(
			prefixes
				.into_iter()
				.take(budget.max_candidates.max(1))
				.map(|(mut steps, prefix_cost, hints)| {
					append_distinct_transit_steps(&mut steps, route_steps.iter().copied());
					MovementCandidate::new(steps, prefix_cost + route_cost, hints)
				})
				.collect(),
		)
	}
}

trait XzDist {
	fn xz_distance_to(self, other: Vec3) -> f32;
}

impl XzDist for Vec3 {
	fn xz_distance_to(self, other: Vec3) -> f32 {
		Vec2::new(self.x, self.z).distance(Vec2::new(other.x, other.z))
	}
}

impl<A> MovementIntelligenceSurface<MovementStep, A> for RichmondAvianMovementSurface<'_, '_>
where
	A: MovementSheet + Send + Sync + 'static,
{
	fn recommend_candidates(
		&mut self,
		from: MovementLocation,
		exclude: &[Entity],
		ability: &A,
		objective: MovementObjective,
		budget: CandidateBudget,
	) -> Vec<MovementCandidate<MovementStep>> {
		let goal = objective.location().point;
		let Some(from_id) = self.storey_at(from.point) else {
			return self.avian.recommend_candidates(from, exclude, ability, objective, budget);
		};
		let from_walk = from.point - Vec3::Y * ability.feet_below_origin();
		let goal_walk = goal - Vec3::Y * ability.feet_below_origin();
		if ability.can_use_stairs() {
			if let Some(goal_link) = self.link_on_actor(goal_walk).cloned() {
				if let Some(candidates) = self.candidates_to_stair_goal(
					from, from_id, exclude, ability, objective, budget, &goal_link, goal_walk,
				) {
					return candidates;
				}
			}
		}
		let Some(to_id) = self.storey_at(goal) else {
			return self.avian.recommend_candidates(from, exclude, ability, objective, budget);
		};
		if from_id == to_id {
			let floor_y = self.storey(from_id).map(|storey| storey.floor_y).unwrap_or(from.point.y);
			return self
				.avian
				.collider_paths(from, exclude, ability, objective, budget)
				.into_iter()
				.map(|mut path| {
					penalize_storey_drop(&mut path, from.point, ability, floor_y);
					path.into_movement_candidate()
				})
				.collect();
		}
		if !ability.can_use_stairs() {
			return self.avian.recommend_candidates(from, exclude, ability, objective, budget);
		}

		let on_stairs = self.link_on_actor(from_walk);
		let chain = match (on_stairs, self.climb_chain(from_id, to_id, from.point)) {
			(Some(link), _) if link_serves(link, from_id, to_id) => vec![link],
			(_, Some(c)) if !c.is_empty() => c,
			_ => {
				return self.avian.recommend_candidates(from, exclude, ability, objective, budget);
			}
		};

		let going_up = to_id > from_id;
		let first = chain[0];
		let approach = if going_up { first.mouth } else { first.landing };
		let approach_radius = (ability.agent_radius() * 0.75).clamp(0.25, 0.45);
		let approach_loc = lift_location(approach, ability, approach_radius);
		let already_at_mouth = approach_loc.contains(from.point);

		let prefixes = if already_at_mouth || on_stairs.is_some() {
			vec![(Vec::new(), 0.0, MovementCandidateHints::default())]
		} else {
			let floor_y = self.storey(from_id).map(|storey| storey.floor_y).unwrap_or(from_walk.y);
			mouth_prefixes(&self.avian, from, exclude, ability, approach_loc, budget, floor_y)
		};

		let mut climb_steps = Vec::new();
		let mut climb_cost = 0.0;
		let mut cursor = from.point;
		// Prefixes finish at the mouth/landing. Select from the actor's feet only
		// when already between storeys; otherwise begin the stair chain at that
		// approach so a nearby side point cannot skip the lineup waypoint.
		let mut cursor_walk = if on_stairs.is_some() { from_walk } else { approach };
		let climb_arrival = (ability.agent_radius() * 0.55).clamp(0.18, 0.3);
		for link in &chain {
			for p in link.oriented_polyline(going_up, cursor_walk) {
				let loc = lift_location(p, ability, climb_arrival);
				climb_cost += loc.xz_distance(cursor);
				climb_steps.push(MovementStep::MoveTo(loc));
				cursor = loc.point;
				cursor_walk = p;
			}
		}

		let upper_from = MovementLocation::new(cursor, ability.agent_radius());
		let upper_budget = CandidateBudget {
			max_candidates: budget.max_candidates.max(1).min(4),
			max_steps: budget.max_steps,
			horizon: budget.horizon,
		};
		let mut upper =
			self.avian.collider_paths(upper_from, exclude, ability, objective, upper_budget);
		let upper_floor_y =
			self.storey(to_id).map(|storey| storey.floor_y).unwrap_or(cursor_walk.y);
		for path in &mut upper {
			penalize_storey_drop(path, upper_from.point, ability, upper_floor_y);
		}
		if upper.is_empty() {
			let dest = objective
				.location()
				.with_y(cursor.y)
				.with_radius(objective.location().radius.max(ability.agent_radius()));
			upper.push(AvianColliderPath {
				points: vec![dest],
				cost: dest.xz_distance(cursor),
				hints: AvianPathHints::default(),
			});
		}

		let mut out = Vec::new();
		for (prefix_steps, prefix_cost, prefix_hints) in prefixes {
			for tail in &upper {
				if out.len() >= budget.max_candidates {
					break;
				}
				let mut steps = prefix_steps.clone();
				append_distinct_transit_steps(&mut steps, climb_steps.iter().copied());
				steps.extend(tail.clone().into_steps());
				let hints = merge_hints(prefix_hints, tail.hints.as_candidate_hints());
				out.push(MovementCandidate::new(
					steps,
					prefix_cost + climb_cost + tail.cost,
					hints,
				));
			}
		}
		out
	}
}

fn link_serves(link: &CirculationStairwell, from_id: u32, to_id: u32) -> bool {
	(link.from_storey == from_id && link.to_storey == to_id)
		|| (link.from_storey == to_id && link.to_storey == from_id)
}

fn append_distinct_transit_steps(
	steps: &mut Vec<MovementStep>,
	additions: impl IntoIterator<Item = MovementStep>,
) {
	for step in additions {
		let duplicate = match (steps.last(), step) {
			(Some(MovementStep::MoveTo(previous)), MovementStep::MoveTo(next)) => {
				previous.point.distance_squared(next.point) <= 1e-4
			}
			_ => false,
		};
		if !duplicate {
			steps.push(step);
		}
	}
}

fn mouth_prefixes<A: MovementSheet>(
	avian: &AvianMovementSurface,
	from: MovementLocation,
	exclude: &[Entity],
	ability: &A,
	approach: MovementLocation,
	budget: CandidateBudget,
	storey_floor_y: f32,
) -> Vec<(Vec<MovementStep>, f32, MovementCandidateHints)> {
	let mouth_budget = CandidateBudget {
		max_candidates: budget.max_candidates.max(1).min(4),
		max_steps: budget.max_steps,
		horizon: budget.horizon,
	};
	let paths = avian.collider_paths(
		from,
		exclude,
		ability,
		MovementObjective::Reach(approach),
		mouth_budget,
	);
	if paths.is_empty() {
		return vec![(
			vec![MovementStep::MoveTo(approach)],
			approach.xz_distance(from.point),
			MovementCandidateHints::default(),
		)];
	}
	paths
		.into_iter()
		.map(|mut path| {
			penalize_storey_drop(&mut path, from.point, ability, storey_floor_y);
			let cost = path.cost;
			let hints = path.hints.as_candidate_hints();
			(path.into_steps(), cost, hints)
		})
		.collect()
}

fn penalize_storey_drop<A: MovementBody>(
	path: &mut AvianColliderPath,
	from: Vec3,
	ability: &A,
	storey_floor_y: f32,
) {
	let starting_feet_y = from.y - ability.feet_below_origin();
	let support_y = starting_feet_y - path.hints.max_drop;
	let storey_drop = storey_floor_y - support_y;
	if storey_drop > STOREY_DROP_SLOP {
		path.cost +=
			STOREY_RECIRCULATION_BASE_COST + storey_drop * STOREY_RECIRCULATION_VERTICAL_COST;
	}
}

fn merge_hints(a: MovementCandidateHints, b: MovementCandidateHints) -> MovementCandidateHints {
	MovementCandidateHints {
		hide: b.hide.max(a.hide),
		sightline: b.sightline.max(a.sightline),
		min_clearance: if a.min_clearance <= 0.0 {
			b.min_clearance
		} else if b.min_clearance <= 0.0 {
			a.min_clearance
		} else {
			a.min_clearance.min(b.min_clearance)
		},
		fall_risk: a.fall_risk.max(b.fall_risk),
	}
}

fn lift_location<A: MovementBody>(walk: Vec3, ability: &A, radius: f32) -> MovementLocation {
	MovementLocation::new(Vec3::new(walk.x, walk.y + ability.feet_below_origin(), walk.z), radius)
}

#[cfg(test)]
mod tests {
	use super::*;
	use movement_intelligence::MovementAbility;

	#[test]
	fn dropping_below_storey_adds_recirculation_cost() {
		let ability = MovementAbility { max_fall: 8.0, ..Default::default() };
		let from = Vec3::new(0.0, ability.feet_below_origin, 0.0);
		let mut path = AvianColliderPath {
			points: Vec::new(),
			cost: 3.0,
			hints: AvianPathHints {
				max_drop: 3.0,
				fall_risk: 3.0 / ability.max_fall,
				..Default::default()
			},
		};
		penalize_storey_drop(&mut path, from, &ability, 0.0);
		assert!(path.cost > STOREY_RECIRCULATION_BASE_COST);
	}

	#[test]
	fn small_same_storey_drop_keeps_geometric_cost() {
		let ability = MovementAbility::default();
		let from = Vec3::new(0.0, ability.feet_below_origin, 0.0);
		let mut path = AvianColliderPath {
			points: Vec::new(),
			cost: 3.0,
			hints: AvianPathHints { max_drop: 0.25, ..Default::default() },
		};
		penalize_storey_drop(&mut path, from, &ability, 0.0);
		assert!((path.cost - 3.0).abs() < 1e-4);
	}

	#[test]
	fn composed_path_keeps_worst_fall_risk() {
		let prefix =
			MovementCandidateHints { fall_risk: 0.2, min_clearance: 0.8, ..Default::default() };
		let tail =
			MovementCandidateHints { fall_risk: 0.7, min_clearance: 0.5, ..Default::default() };
		let merged = merge_hints(prefix, tail);
		assert!((merged.fall_risk - 0.7).abs() < 1e-4);
		assert!((merged.min_clearance - 0.5).abs() < 1e-4);
	}

	#[test]
	fn composed_transit_steps_drop_duplicate_join_point() {
		let join = Vec3::new(1.0, 2.0, 3.0);
		let mut steps = vec![MovementStep::MoveTo(MovementLocation::new(join, 0.4))];
		append_distinct_transit_steps(
			&mut steps,
			[
				MovementStep::MoveTo(MovementLocation::new(join, 0.2)),
				MovementStep::MoveTo(MovementLocation::new(join + Vec3::X, 0.2)),
			],
		);
		assert_eq!(steps.len(), 2);
		assert_eq!(steps[0], MovementStep::MoveTo(MovementLocation::new(join, 0.4)));
	}
}
