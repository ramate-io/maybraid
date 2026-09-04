//! Sample endpoints for an objective, probe walk chains against Fixed colliders.
//!
//! [`VantageOn`](MovementObjective::VantageOn) ranks standpoints with cheap hide/sightline
//! rays, then spends walk probes on the best first.

use avian3d::prelude::{SpatialQuery, SpatialQueryFilter};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;
use movement_intelligence::{CandidateBudget, MovementLocation, MovementObjective, MovementSheet};
use std::f32::consts::TAU;

use crate::path::{AvianColliderPath, AvianPathHints};

struct RankedStandpoint {
	location: MovementLocation,
	hints: AvianPathHints,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct FallProfile {
	max_drop: f32,
	risk: f32,
}

/// Avian [`SpatialQuery`] over [`PhysicsInteractionLayer::Fixed`].
#[derive(SystemParam)]
pub struct AvianMovementSurface<'w, 's> {
	spatial: SpatialQuery<'w, 's>,
}

impl AvianMovementSurface<'_, '_> {
	pub fn collider_paths<A: MovementSheet>(
		&self,
		from: MovementLocation,
		exclude: &[Entity],
		ability: &A,
		objective: MovementObjective,
		budget: CandidateBudget,
	) -> Vec<AvianColliderPath> {
		let filter = Self::filter(exclude);
		let mut ranked = self.rank_standpoints(from, ability, objective, budget, &filter);
		if objective.is_vantage_on() {
			ranked.sort_by(|a, b| {
				a.hints
					.as_candidate_hints()
					.covering_score(objective)
					.total_cmp(&b.hints.as_candidate_hints().covering_score(objective))
			});
		}

		let mut paths = Vec::new();
		for sample in ranked {
			if paths.len() >= budget.max_candidates {
				break;
			}
			let Some((waypoints, fall)) = self.probe_walk(
				from.point,
				sample.location.point,
				ability,
				budget.max_steps,
				&filter,
			) else {
				continue;
			};
			let end = waypoints.last().copied().unwrap_or(sample.location.point);
			let arrival = MovementLocation::new(end, sample.location.radius);
			let mut points: Vec<MovementLocation> = waypoints
				.iter()
				.take(waypoints.len().saturating_sub(1))
				.map(|point| MovementLocation::new(*point, ability.agent_radius() * 1.2))
				.collect();
			points.push(arrival);
			let cost = path_length(from.point, &waypoints);
			let mut hints = sample.hints;
			hints.max_drop = fall.max_drop;
			hints.fall_risk = fall.risk;
			paths.push(AvianColliderPath { points, cost, hints });
		}
		paths
	}

	fn filter(exclude: &[Entity]) -> SpatialQueryFilter {
		SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed)
			.with_excluded_entities(exclude.iter().copied())
	}

	fn rank_standpoints<A: MovementSheet>(
		&self,
		from: MovementLocation,
		ability: &A,
		objective: MovementObjective,
		budget: CandidateBudget,
		filter: &SpatialQueryFilter,
	) -> Vec<RankedStandpoint> {
		let samples = Self::sample_endpoints(from, ability, objective, budget);
		samples
			.into_iter()
			.map(|location| {
				let hints = if objective.is_vantage_on() {
					self.objective_hints(ability, objective, location.point, filter)
				} else {
					AvianPathHints::default()
				};
				RankedStandpoint { location, hints }
			})
			.collect()
	}

	fn sample_endpoints<A: MovementSheet>(
		from: MovementLocation,
		ability: &A,
		objective: MovementObjective,
		budget: CandidateBudget,
	) -> Vec<MovementLocation> {
		let location = objective.location();
		let y = from.point.y;
		let arrival = ability.agent_radius() * 1.25;
		let azimuths = ability.vantage_azimuths();
		match objective {
			MovementObjective::Reach(_) => {
				let mut samples =
					vec![location.with_y(y).with_radius(location.radius.max(arrival))];
				samples.extend(MovementLocation::ring_around(
					location.point,
					y,
					(location.radius * 0.65).max(0.4),
					azimuths,
					arrival,
				));
				samples
			}
			MovementObjective::EdgeOf(_) => MovementLocation::ring_around(
				location.point,
				y,
				location.radius.max(arrival),
				azimuths,
				arrival,
			),
			MovementObjective::FleeFrom(_) => {
				Self::flee_samples(from.point, location, y, arrival, budget, azimuths)
			}
			MovementObjective::VantageOn { .. } => {
				Self::vantage_samples(ability, location.point, y, arrival, budget)
			}
		}
	}

	fn flee_samples(
		from: Vec3,
		location: MovementLocation,
		y: f32,
		arrival: f32,
		budget: CandidateBudget,
		azimuths: u32,
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
		let n = (azimuths as usize).min(budget.max_candidates.max(1)).max(1) as u32;
		for i in 0..n {
			let yaw = i as f32 / n as f32 * TAU;
			let dir = Quat::from_axis_angle(Vec3::Y, yaw) * away;
			let dist = radius.min(budget.horizon);
			samples.push(MovementLocation::new(
				Vec3::new(location.point.x + dir.x * dist, y, location.point.z + dir.z * dist),
				arrival,
			));
		}
		samples
	}

	fn vantage_samples<A: movement_intelligence::Covering>(
		ability: &A,
		center: Vec3,
		y: f32,
		arrival: f32,
		budget: CandidateBudget,
	) -> Vec<MovementLocation> {
		let azimuths = ability.vantage_azimuths();
		let mut samples = Vec::new();
		for radius in ability.vantage_standoffs() {
			if *radius > budget.horizon {
				continue;
			}
			samples.extend(MovementLocation::ring_around(center, y, *radius, azimuths, arrival));
		}
		if samples.is_empty() {
			samples.extend(MovementLocation::ring_around(
				center,
				y,
				budget.horizon.min(8.0).max(2.0),
				azimuths,
				arrival,
			));
		}
		samples
	}

	fn probe_walk<A: movement_intelligence::MovementBody>(
		&self,
		from: Vec3,
		to: Vec3,
		ability: &A,
		max_steps: usize,
		filter: &SpatialQueryFilter,
	) -> Option<(Vec<Vec3>, FallProfile)> {
		let direct = [to];
		if let Some(walk) = self.trace_ground_path(from, &direct, ability, filter) {
			return Some(walk);
		}
		if max_steps < 2 {
			return None;
		}
		let delta = Vec3::new(to.x - from.x, 0.0, to.z - from.z);
		let len = delta.length();
		if len < 1e-3 {
			return self.trace_ground_path(from, &direct, ability, filter);
		}
		let dir = delta / len;
		let perp = Vec3::new(-dir.z, 0.0, dir.x);
		let step = ability.max_step().max(0.1);
		for dist in [step * 4.0, step * 7.0, step * 10.5] {
			for sign in [1.0, -1.0] {
				let via = Vec3::new(
					from.x + dir.x * (len * 0.45) + perp.x * dist * sign,
					from.y,
					from.z + dir.z * (len * 0.45) + perp.z * dist * sign,
				);
				let detour = [via, to];
				if let Some(walk) = self.trace_ground_path(from, &detour, ability, filter) {
					return Some(walk);
				}
			}
		}
		None
	}

	/// Ground-snap a route in short sequential samples.
	///
	/// `max_fall` limits one unsupported local drop, not the cumulative
	/// elevation change of an otherwise supported downhill walk.
	fn trace_ground_path<A: movement_intelligence::MovementBody>(
		&self,
		from: Vec3,
		waypoints: &[Vec3],
		ability: &A,
		filter: &SpatialQueryFilter,
	) -> Option<(Vec<Vec3>, FallProfile)> {
		let max_fall = ability.max_fall().max(0.0);
		let probe_lift = ability.max_step().max(0.05) + 0.08;
		let probe_distance = probe_lift + max_fall + 0.05;
		let spacing = (ability.agent_radius() * 0.75).clamp(0.2, 0.5);
		let feet_below_origin = ability.feet_below_origin();
		let mut previous = from;
		let mut ground_y = from.y - feet_below_origin;
		let mut fall = FallProfile::default();
		let mut snapped_waypoints = Vec::with_capacity(waypoints.len());

		for waypoint in waypoints {
			let start = previous;
			let length = start.xz().distance(waypoint.xz());
			let count = (length / spacing).ceil().max(1.0) as usize;
			for i in 1..=count {
				let t = i as f32 / count as f32;
				let xz = start.xz().lerp(waypoint.xz(), t);
				let origin = Vec3::new(xz.x, ground_y + probe_lift, xz.y);
				let hit =
					self.spatial.cast_ray(origin, Dir3::NEG_Y, probe_distance, true, filter)?;
				// A near-zero hit means the probe origin is inside Fixed
				// geometry: the surface rose farther than max_step.
				if hit.distance < 0.02 {
					return None;
				}
				let next_ground_y = origin.y - hit.distance;
				if !admit_ground_delta(
					&mut fall,
					next_ground_y - ground_y,
					ability.max_step(),
					max_fall,
				) {
					return None;
				}
				let next = Vec3::new(xz.x, next_ground_y + feet_below_origin, xz.y);
				if !self.segment_clear(
					ability.hip_point(previous),
					ability.hip_point(next),
					ability.agent_radius(),
					filter,
				) {
					return None;
				}
				previous = next;
				ground_y = next_ground_y;
			}
			snapped_waypoints.push(previous);
		}

		fall.risk = normalized_fall_risk(fall.max_drop, max_fall);
		Some((snapped_waypoints, fall))
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

	fn objective_hints<A: movement_intelligence::MovementBody>(
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
		AvianPathHints { hide, sightline, min_clearance: 1.0, ..Default::default() }
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

fn path_length(start: Vec3, waypoints: &[Vec3]) -> f32 {
	let mut last = start;
	let mut total = 0.0;
	for point in waypoints {
		total += Vec2::new(last.x, last.z).distance(Vec2::new(point.x, point.z));
		last = *point;
	}
	total
}

fn admit_ground_delta(profile: &mut FallProfile, delta: f32, max_step: f32, max_fall: f32) -> bool {
	const SLOP: f32 = 0.04;
	if delta > max_step.max(0.0) + SLOP || -delta > max_fall.max(0.0) + SLOP {
		return false;
	}
	profile.max_drop = profile.max_drop.max((-delta).max(0.0));
	true
}

fn normalized_fall_risk(drop: f32, tolerance: f32) -> f32 {
	if tolerance <= 0.04 {
		if drop > 0.04 {
			1.0
		} else {
			0.0
		}
	} else {
		(drop / tolerance).clamp(0.0, 1.0)
	}
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

	#[test]
	fn cumulative_supported_descent_is_not_treated_as_one_fall() -> anyhow::Result<()> {
		let mut profile = FallProfile::default();
		for _ in 0..20 {
			assert!(admit_ground_delta(&mut profile, -0.3, 0.4, 1.2));
		}
		assert!((profile.max_drop - 0.3).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn one_unsupported_drop_is_rejected() -> anyhow::Result<()> {
		let mut profile = FallProfile::default();
		assert!(!admit_ground_delta(&mut profile, -1.5, 0.4, 1.2));
		assert_eq!(profile, FallProfile::default());
		Ok(())
	}

	#[test]
	fn one_too_tall_step_is_rejected() -> anyhow::Result<()> {
		let mut profile = FallProfile::default();
		assert!(!admit_ground_delta(&mut profile, 0.6, 0.4, 1.2));
		assert_eq!(profile, FallProfile::default());
		Ok(())
	}

	#[test]
	fn fall_risk_is_fraction_of_tolerance() {
		assert!((normalized_fall_risk(0.6, 1.2) - 0.5).abs() < 1e-4);
		assert_eq!(normalized_fall_risk(2.0, 1.2), 1.0);
		assert_eq!(normalized_fall_risk(0.0, 0.0), 0.0);
	}
}
