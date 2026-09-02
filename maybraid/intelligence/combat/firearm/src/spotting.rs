//! Sightline-backed target observation and memory.

use avian3d::prelude::{LinearVelocity, SpatialQuery, SpatialQueryFilter};
use bevy::prelude::*;
use lod_avian::PhysicsInteractionLayer;
use movement_intelligence::{MovementBody, MovementIntelligence};

use crate::combat::FirearmIntelligence;
use crate::los::clear_segment;
use crate::movement::FirearmMovementIntelligence;
use crate::target::{
	allocate_vision, cascade_vision, rank_candidates, retain_live_candidates, upsert_observation,
	FirearmSpotting, SpottedTarget, TargetCapsule,
};

pub(crate) fn spot_firearm_targets(
	spatial: SpatialQuery,
	time: Res<Time>,
	mut spotters: Query<(
		&Transform,
		&MovementIntelligence,
		&FirearmSpotting,
		&mut FirearmIntelligence,
		&mut FirearmMovementIntelligence,
	)>,
	targets: Query<(&Transform, Option<&LinearVelocity>)>,
) {
	let now = time.elapsed_secs();
	let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed);
	for (transform, movement, spotting, mut combat, mut combat_movement) in &mut spotters {
		let memory = combat.settings.target_spotting_memory.max(0.0);
		retain_live_candidates(&mut combat.objective.0, &spotting.candidates, now, memory);
		combat_movement.objective.0.clear();

		let observer = movement.ability.eye_point(transform.translation);
		let mut live = Vec::new();
		for candidate in &spotting.candidates {
			let Ok((target, velocity)) = targets.get(candidate.entity) else {
				continue;
			};
			live.push((
				*candidate,
				target.translation,
				velocity.map_or(Vec3::ZERO, |velocity| velocity.0),
			));
		}

		let ranked_keys: Vec<(Entity, Vec3)> = live
			.iter()
			.map(|(candidate, position, _)| (candidate.entity, *position))
			.collect();
		let order = rank_candidates(observer, &ranked_keys, combat.engaged, combat.settings.focus);
		let mut rays = allocate_vision(combat.settings.vision, order.len(), combat.settings.focus);
		cascade_vision(&mut rays, SPOT_SAMPLE_COUNT as u16);

		for (candidate, position, movement_vector) in &live {
			let observation = SpottedTarget {
				entity: candidate.entity,
				position: *position,
				capsule: candidate.capsule,
				visible: candidate.capsule.center_mass(*position),
				visible_head: None,
				movement_vector: *movement_vector,
				spotted_at: now,
			};
			upsert_observation(&mut combat_movement.objective.0, observation);
		}

		for (rank, &index) in order.iter().enumerate() {
			let budget = rays.get(rank).copied().unwrap_or(0) as usize;
			if budget == 0 {
				continue;
			}
			let (candidate, position, movement_vector) = live[index];
			let (visible, visible_head) =
				visible_points(observer, position, candidate.capsule, budget, &spatial, &filter);
			let observation = SpottedTarget {
				entity: candidate.entity,
				position,
				capsule: candidate.capsule,
				visible: visible.unwrap_or_else(|| candidate.capsule.center_mass(position)),
				visible_head,
				movement_vector,
				spotted_at: now,
			};
			upsert_observation(&mut combat_movement.objective.0, observation);
			let Some(visible) = visible else {
				continue;
			};
			upsert_observation(
				&mut combat.objective.0,
				SpottedTarget { visible, visible_head, ..observation },
			);
		}
	}
}

const SPOT_SAMPLE_COUNT: usize = 9;

fn visible_points(
	observer: Vec3,
	position: Vec3,
	capsule: TargetCapsule,
	budget: usize,
	spatial: &SpatialQuery,
	filter: &SpatialQueryFilter,
) -> (Option<Vec3>, Option<Vec3>) {
	let samples = capsule_samples(observer, position, capsule);
	let mut body = None;
	let mut head = None;
	for (index, point) in samples.into_iter().take(budget.min(SPOT_SAMPLE_COUNT)).enumerate() {
		if !clear_segment(observer, point, spatial, filter) {
			continue;
		}
		if index == 1 {
			head = Some(point);
		} else {
			body = body.or(Some(point));
		}
	}
	(body.or(head), head)
}

fn capsule_samples(
	observer: Vec3,
	position: Vec3,
	capsule: TargetCapsule,
) -> [Vec3; SPOT_SAMPLE_COUNT] {
	let toward =
		Vec3::new(position.x - observer.x, 0.0, position.z - observer.z).normalize_or(Vec3::Z);
	let right = Vec3::new(toward.z, 0.0, -toward.x) * (capsule.radius * 0.8);
	let center = capsule.center_mass(position);
	let head = capsule.head(position);
	let hips = position - Vec3::Y * (capsule.half_height * 0.45);
	[
		center,
		head,
		hips,
		center + right,
		center - right,
		head + right,
		head - right,
		hips + right,
		hips - right,
	]
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn capsule_samples_cover_center_head_and_sides() {
		let samples = capsule_samples(Vec3::Z * 5.0, Vec3::ZERO, TargetCapsule::new(0.4, 0.9));
		assert_eq!(samples[0], Vec3::ZERO);
		assert!(samples[1].y > 0.0);
		assert!(samples[3].x.abs() > 0.2);
		assert_eq!(samples[3].x, -samples[4].x);
	}
}
