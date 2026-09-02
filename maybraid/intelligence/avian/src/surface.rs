//! Sample endpoints for an objective, probe walk chains against Fixed colliders.

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;
use movement_intelligence::{CandidateBudget, MovementBody, MovementLocation, MovementObjective};
use std::f32::consts::TAU;

use crate::path::{AvianColliderPath, AvianPathHints};

/// Avian [`SpatialQuery`] over [`PhysicsInteractionLayer::Fixed`].
#[derive(SystemParam)]
pub struct AvianMovementSurface<'w, 's> {
	spatial: SpatialQuery<'w, 's>,
}

impl AvianMovementSurface<'_, '_> {
	pub fn collider_paths<A: MovementBody>(
		&self,
		from: MovementLocation,
		exclude: &[Entity],
		ability: &A,
		objective: MovementObjective,
		budget: CandidateBudget,
	) -> Vec<AvianColliderPath> {
		let filter = self.filter(exclude);
		let samples = Self::sample_endpoints(from, ability, objective, budget);
		let mut paths = Vec::new();
		for sample in samples {
			if paths.len() >= budget.max_candidates {
				break;
			}
			let Some(waypoints) =
				self.probe_walk(from.point, sample.point, ability, budget.max_steps, &filter)
			else {
				continue;
			};
			let end = waypoints.last().copied().unwrap_or(sample.point);
			let arrival = MovementLocation::new(end, sample.radius);
			let mut points: Vec<MovementLocation> = waypoints
				.iter()
				.take(waypoints.len().saturating_sub(1))
				.map(|point| MovementLocation::new(*point, ability.agent_radius() * 1.2))
				.collect();
			points.push(arrival);
			let cost = path_length(from.point, &waypoints);
			let hints = self.objective_hints(ability, objective, sample.point, &filter);
			paths.push(AvianColliderPath { points, cost, hints });
		}
		paths
	}

	fn filter(&self, exclude: &[Entity]) -> SpatialQueryFilter {
		SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed)
			.with_excluded_entities(exclude.iter().copied())
	}

	fn sample_endpoints<A: MovementBody>(
		from: MovementLocation,
		ability: &A,
		objective: MovementObjective,
		budget: CandidateBudget,
	) -> Vec<MovementLocation> {
		let location = objective.location();
		let y = from.point.y;
		let arrival = ability.agent_radius() * 1.25;
		match objective {
			MovementObjective::Reach(_) => {
				let mut samples = vec![MovementLocation::new(
					with_y(location.point, y),
					location.radius.max(arrival),
				)];
				samples.extend(ring(
					location.point,
					y,
					(location.radius * 0.65).max(0.4),
					6,
					arrival,
				));
				samples
			}
			MovementObjective::EdgeOf(_) => {
				ring(location.point, y, location.radius.max(arrival), 8, arrival)
			}
			MovementObjective::FleeFrom(_) => {
				Self::flee_samples(from.point, location, y, arrival, budget)
			}
			MovementObjective::VantageOn { .. } => {
				Self::vantage_samples(location.point, y, arrival, budget)
			}
		}
	}

	fn flee_samples(
		from: Vec3,
		location: MovementLocation,
		y: f32,
		arrival: f32,
		budget: CandidateBudget,
	) -> Vec<MovementLocation> {
		let away = {
			let delta = Vec3::new(from.x - location.point.x, 0.0, from.z - location.point.z);
			if delta.length_squared() < 1e-4 {
				Vec3::X
			} else {
				delta.normalize()
			}
		};
		let radius = location.radius.max(1.0) + 2.0;
		let mut samples = Vec::new();
		let n = 8.min(budget.max_candidates.max(1));
		for i in 0..n {
			let yaw = (i as f32 / n as f32) * TAU;
			let dir = Quat::from_axis_angle(Vec3::Y, yaw) * away;
			let dist = radius.min(budget.horizon);
			samples.push(MovementLocation::new(
				Vec3::new(location.point.x + dir.x * dist, y, location.point.z + dir.z * dist),
				arrival,
			));
		}
		samples
	}

	fn vantage_samples(
		center: Vec3,
		y: f32,
		arrival: f32,
		budget: CandidateBudget,
	) -> Vec<MovementLocation> {
		let rings = [3.5_f32, 6.5, 10.0];
		let mut samples = Vec::new();
		for radius in rings {
			if radius > budget.horizon {
				continue;
			}
			samples.extend(ring(center, y, radius, 8, arrival));
		}
		if samples.is_empty() {
			samples.extend(ring(center, y, budget.horizon.min(8.0).max(2.0), 8, arrival));
		}
		samples
	}

	fn probe_walk<A: MovementBody>(
		&self,
		from: Vec3,
		to: Vec3,
		ability: &A,
		max_steps: usize,
		filter: &SpatialQueryFilter,
	) -> Option<Vec<Vec3>> {
		if self.segment_clear(
			ability.hip_point(from),
			ability.hip_point(to),
			ability.agent_radius(),
			filter,
		) {
			return Some(vec![to]);
		}
		if max_steps < 2 {
			return None;
		}
		let delta = Vec3::new(to.x - from.x, 0.0, to.z - from.z);
		let len = delta.length();
		if len < 1e-3 {
			return Some(vec![to]);
		}
		let dir = delta / len;
		let perp = Vec3::new(-dir.z, 0.0, dir.x);
		for dist in [1.6_f32, 2.8, 4.2] {
			for sign in [1.0, -1.0] {
				let via = Vec3::new(
					from.x + dir.x * (len * 0.45) + perp.x * dist * sign,
					from.y,
					from.z + dir.z * (len * 0.45) + perp.z * dist * sign,
				);
				if self.segment_clear(
					ability.hip_point(from),
					ability.hip_point(via),
					ability.agent_radius(),
					filter,
				) && self.segment_clear(
					ability.hip_point(via),
					ability.hip_point(to),
					ability.agent_radius(),
					filter,
				) {
					return Some(vec![via, to]);
				}
			}
		}
		None
	}

	fn segment_clear(
		&self,
		start: Vec3,
		end: Vec3,
		agent_radius: f32,
		filter: &SpatialQueryFilter,
	) -> bool {
		let delta = end - start;
		let dist = delta.length();
		if dist < 1e-4 {
			return true;
		}
		let Ok(direction) = Dir3::new(delta) else {
			return true;
		};
		match self.spatial.cast_ray(start, direction, dist, true, filter) {
			None => true,
			Some(hit) => hit.distance >= (dist - agent_radius).max(0.0),
		}
	}

	fn objective_hints<A: MovementBody>(
		&self,
		ability: &A,
		objective: MovementObjective,
		sample: Vec3,
		filter: &SpatialQueryFilter,
	) -> AvianPathHints {
		let target = objective.location().point;
		let hide = self.occlusion(ability.hip_point(target), ability.hip_point(sample), filter);
		let sightline =
			1.0 - self.occlusion(ability.eye_point(sample), ability.eye_point(target), filter);
		AvianPathHints { hide, sightline, min_clearance: 1.0 }
	}

	fn occlusion(&self, start: Vec3, end: Vec3, filter: &SpatialQueryFilter) -> f32 {
		let delta = end - start;
		let dist = delta.length();
		if dist < 1e-4 {
			return 0.0;
		}
		let Ok(direction) = Dir3::new(delta) else {
			return 0.0;
		};
		match self.spatial.cast_ray(start, direction, dist, true, filter) {
			Some(hit) if hit.distance < dist - 0.12 => 1.0,
			_ => 0.0,
		}
	}
}

fn with_y(point: Vec3, y: f32) -> Vec3 {
	Vec3::new(point.x, y, point.z)
}

fn ring(center: Vec3, y: f32, radius: f32, count: u32, arrival: f32) -> Vec<MovementLocation> {
	(0..count)
		.map(|i| {
			let yaw = i as f32 / count as f32 * TAU;
			MovementLocation::new(
				Vec3::new(center.x + radius * yaw.cos(), y, center.z + radius * yaw.sin()),
				arrival,
			)
		})
		.collect()
}

fn path_length(start: Vec3, waypoints: &[Vec3]) -> f32 {
	let mut last = start;
	let mut total = 0.0;
	for point in waypoints {
		total += Vec2::new(last.x, last.z).distance(Vec2::new(point.x, point.z));
		last = *point;
	}
	total
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn path_length_sums_xz() -> anyhow::Result<()> {
		let len = path_length(Vec3::ZERO, &[Vec3::new(3.0, 9.0, 4.0)]);
		assert!((len - 5.0).abs() < 1e-4, "{len}");
		Ok(())
	}
}
