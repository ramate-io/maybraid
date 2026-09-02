//! Compose Avian collider probes with [`CirculationStairwell`] / [`CirculationStorey`].

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use movement_intelligence::{
	CandidateBudget, MovementBody, MovementCandidate, MovementCandidateHints,
	MovementIntelligenceSurface, MovementLocation, MovementObjective, MovementSheet, MovementStep,
};
use movement_intelligence_avian::{AvianColliderPath, AvianMovementSurface, AvianPathHints};

use crate::circulation::{CirculationStairwell, CirculationStorey};

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
		let Some(to_id) = self.storey_at(goal) else {
			return self.avian.recommend_candidates(from, exclude, ability, objective, budget);
		};
		if !ability.can_use_stairs() || from_id == to_id {
			return self.avian.recommend_candidates(from, exclude, ability, objective, budget);
		}

		let from_walk = from.point - Vec3::Y * ability.feet_below_origin();
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
			mouth_prefixes(&self.avian, from, exclude, ability, approach_loc, budget)
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
			max_candidates: 1,
			max_steps: budget.max_steps,
			horizon: budget.horizon,
		};
		let mut upper =
			self.avian.collider_paths(upper_from, exclude, ability, objective, upper_budget);
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
		for (mut steps, cost, hints) in prefixes {
			if out.len() >= budget.max_candidates {
				break;
			}
			steps.extend(climb_steps.iter().copied());
			let tail = &upper[0];
			steps.extend(tail.clone().into_steps());
			out.push(MovementCandidate::new(steps, cost + climb_cost + tail.cost, hints));
		}
		out
	}
}

fn link_serves(link: &CirculationStairwell, from_id: u32, to_id: u32) -> bool {
	(link.from_storey == from_id && link.to_storey == to_id)
		|| (link.from_storey == to_id && link.to_storey == from_id)
}

fn mouth_prefixes<A: MovementSheet>(
	avian: &AvianMovementSurface,
	from: MovementLocation,
	exclude: &[Entity],
	ability: &A,
	approach: MovementLocation,
	budget: CandidateBudget,
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
		.map(|path| {
			let cost = path.cost;
			let hints = path.hints.as_candidate_hints();
			(path.into_steps(), cost, hints)
		})
		.collect()
}

fn lift_location<A: MovementBody>(walk: Vec3, ability: &A, radius: f32) -> MovementLocation {
	MovementLocation::new(Vec3::new(walk.x, walk.y + ability.feet_below_origin(), walk.z), radius)
}
