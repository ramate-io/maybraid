//! Perception candidates and spotted target snapshots.

use bevy::prelude::*;

/// Semantic target shape used for spotting and body/head aim points.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetCapsule {
	pub radius: f32,
	pub half_height: f32,
}

impl TargetCapsule {
	pub fn new(radius: f32, half_height: f32) -> Self {
		Self { radius: radius.max(0.0), half_height: half_height.max(radius) }
	}

	pub fn center_mass(self, origin: Vec3) -> Vec3 {
		origin
	}

	pub fn head(self, origin: Vec3) -> Vec3 {
		origin + Vec3::Y * (self.half_height - self.radius * 0.5).max(0.0)
	}
}

/// A possible enemy supplied to the spotting system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CombatTarget {
	pub entity: Entity,
	pub capsule: TargetCapsule,
}

impl CombatTarget {
	pub fn new(entity: Entity, capsule: TargetCapsule) -> Self {
		Self { entity, capsule }
	}
}

/// Targets perception should try to observe.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct FirearmSpotting {
	pub candidates: Vec<CombatTarget>,
}

/// Last observed state of one combat target.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpottedTarget {
	pub entity: Entity,
	/// Capsule origin at the observation time.
	pub position: Vec3,
	pub capsule: TargetCapsule,
	/// A point on the capsule that was actually clear when spotted.
	pub visible: Vec3,
	/// Head sample if that ray was clear; used when the shooter prefers headshots.
	pub visible_head: Option<Vec3>,
	pub movement_vector: Vec3,
	/// [`Time::elapsed_secs`] when this observation was made.
	pub spotted_at: f32,
}

impl SpottedTarget {
	pub fn aim_point(self, headshots: f32) -> Vec3 {
		if headshots > 0.5 {
			self.visible_head.unwrap_or(self.visible)
		} else {
			self.visible
		}
	}

	pub fn is_fresh(self, now: f32, window: f32) -> bool {
		now - self.spotted_at <= window.max(0.0)
	}
}

/// Who to shoot. Written by perception; fielded by [`crate::FirearmIntelligence`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FirearmObjective(pub Vec<SpottedTarget>);

/// Who to stand relative to. Written by perception; fielded by
/// [`crate::FirearmMovementIntelligence`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FirearmMovementObjective(pub Vec<SpottedTarget>);

/// Pick an observation given current `from` and optional sticky `engaged`.
///
/// `focus` 0 always takes the nearest. `focus` 1 keeps `engaged` until it leaves
/// the list. In between, switch only when the nearest is closer by more than
/// `focus` × 8 m.
pub fn pick_target(
	from: Vec3,
	candidates: &[SpottedTarget],
	engaged: Option<Entity>,
	focus: f32,
) -> Option<&SpottedTarget> {
	let mut nearest: Option<(&SpottedTarget, f32)> = None;
	let mut engaged_target = None;
	for target in candidates {
		let dist =
			Vec2::new(from.x, from.z).distance(Vec2::new(target.position.x, target.position.z));
		if engaged == Some(target.entity) {
			engaged_target = Some((target, dist));
		}
		let take = nearest.is_none_or(|(_, best)| dist < best);
		if take {
			nearest = Some((target, dist));
		}
	}
	let (nearest_target, nearest_dist) = nearest?;
	let Some((engaged_target, engaged_dist)) = engaged_target else {
		return Some(nearest_target);
	};
	let stick = focus.clamp(0.0, 1.0) * 8.0;
	if nearest_dist + stick < engaged_dist {
		Some(nearest_target)
	} else {
		Some(engaged_target)
	}
}

/// Rank live candidates for spotting. Closer is more important. `focus` adds
/// `focus` × 8 m of stickiness to `engaged`, matching [`pick_target`].
pub fn rank_candidates(
	from: Vec3,
	candidates: &[(Entity, Vec3)],
	engaged: Option<Entity>,
	focus: f32,
) -> Vec<usize> {
	let stick = focus.clamp(0.0, 1.0) * 8.0;
	let mut order: Vec<(usize, f32)> = candidates
		.iter()
		.enumerate()
		.map(|(index, (entity, position))| {
			let dist = Vec2::new(from.x, from.z).distance(Vec2::new(position.x, position.z));
			let score = if engaged == Some(*entity) { stick - dist } else { -dist };
			(index, score)
		})
		.collect();
	order.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
	order.into_iter().map(|(index, _)| index).collect()
}

/// Split `vision` rays across `n` ranked targets. `focus` 0 shares evenly.
/// `focus` 1 spends the whole budget on the first-ranked target.
pub fn allocate_vision(vision: u16, n: usize, focus: f32) -> Vec<u16> {
	if n == 0 {
		return Vec::new();
	}
	let vision = vision as usize;
	let focus = focus.clamp(0.0, 1.0);
	let decay = 1.0 - focus;
	let mut weights = Vec::with_capacity(n);
	for i in 0..n {
		let weight = if i == 0 { 1.0 } else { decay.powi(i as i32) };
		weights.push(weight.max(0.0));
	}
	let sum: f32 = weights.iter().sum::<f32>().max(1e-6);
	let mut rays = vec![0u16; n];
	let mut used = 0usize;
	let mut remainder: Vec<(usize, f32)> = Vec::with_capacity(n);
	for (i, weight) in weights.iter().enumerate() {
		let exact = vision as f32 * weight / sum;
		let whole = exact.floor() as usize;
		rays[i] = whole as u16;
		used = used.saturating_add(whole);
		remainder.push((i, exact - whole as f32));
	}
	remainder.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
	let mut leftover = vision.saturating_sub(used);
	for (i, _) in remainder {
		if leftover == 0 {
			break;
		}
		rays[i] = rays[i].saturating_add(1);
		leftover -= 1;
	}
	rays
}

/// Cap each target at `max_per` unique sample points and give unused rays to
/// the next-ranked target that still has room.
pub fn cascade_vision(rays: &mut [u16], max_per: u16) {
	let max_per = max_per.max(1);
	let mut extra = 0u32;
	for slot in rays.iter_mut() {
		let granted = (*slot as u32) + extra;
		if granted > max_per as u32 {
			*slot = max_per;
			extra = granted - max_per as u32;
		} else {
			*slot = granted as u16;
			extra = 0;
		}
	}
}

pub(crate) fn upsert_observation(targets: &mut Vec<SpottedTarget>, observation: SpottedTarget) {
	if let Some(target) = targets.iter_mut().find(|target| target.entity == observation.entity) {
		*target = observation;
	} else {
		targets.push(observation);
	}
}

pub(crate) fn retain_recent(targets: &mut Vec<SpottedTarget>, now: f32, memory: f32) {
	let memory = memory.max(0.0);
	targets.retain(|target| now - target.spotted_at <= memory);
}

pub(crate) fn retain_live_candidates(
	targets: &mut Vec<SpottedTarget>,
	candidates: &[CombatTarget],
	now: f32,
	memory: f32,
) {
	retain_recent(targets, now, memory);
	targets.retain(|target| candidates.iter().any(|candidate| candidate.entity == target.entity));
}

/// Translate a remembered aim point onto the target's current capsule origin.
pub(crate) fn live_aim_point(target: SpottedTarget, headshots: f32, current: Option<Vec3>) -> Vec3 {
	let point = target.aim_point(headshots);
	match current {
		Some(position) => point + (position - target.position),
		None => point,
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn spotted(entity: Entity, position: Vec3) -> SpottedTarget {
		let capsule = TargetCapsule::new(0.4, 0.9);
		SpottedTarget {
			entity,
			position,
			capsule,
			visible: capsule.center_mass(position),
			visible_head: Some(capsule.head(position)),
			movement_vector: Vec3::ZERO,
			spotted_at: 0.0,
		}
	}

	#[test]
	fn pick_target_takes_nearest_when_unfocused() -> anyhow::Result<()> {
		let a = Entity::from_bits(1);
		let b = Entity::from_bits(2);
		let targets = [spotted(a, Vec3::X * 4.0), spotted(b, Vec3::X * 2.0)];
		let picked = pick_target(Vec3::ZERO, &targets, Some(a), 0.0);
		assert_eq!(picked.map(|target| target.entity), Some(b));
		Ok(())
	}

	#[test]
	fn pick_target_keeps_engaged_when_focused() -> anyhow::Result<()> {
		let a = Entity::from_bits(1);
		let b = Entity::from_bits(2);
		let targets = [spotted(a, Vec3::X * 4.0), spotted(b, Vec3::X * 2.0)];
		let picked = pick_target(Vec3::ZERO, &targets, Some(a), 1.0);
		assert_eq!(picked.map(|target| target.entity), Some(a));
		Ok(())
	}

	#[test]
	fn memory_expires_old_observations() -> anyhow::Result<()> {
		let mut targets = vec![spotted(Entity::from_bits(1), Vec3::ZERO)];
		retain_recent(&mut targets, 1.1, 1.0);
		assert!(targets.is_empty());
		Ok(())
	}

	#[test]
	fn memory_drops_entities_that_left_the_candidate_list() {
		let a = Entity::from_bits(1);
		let b = Entity::from_bits(2);
		let mut targets = vec![spotted(a, Vec3::ZERO), spotted(b, Vec3::X)];
		let candidates = [CombatTarget::new(b, TargetCapsule::new(0.4, 0.9))];
		retain_live_candidates(&mut targets, &candidates, 0.1, 1.0);
		assert_eq!(targets.len(), 1);
		assert_eq!(targets[0].entity, b);
	}

	#[test]
	fn live_aim_follows_the_current_capsule_origin() {
		let target = spotted(Entity::from_bits(1), Vec3::ZERO);
		let moved = live_aim_point(target, 0.0, Some(Vec3::X * 2.0));
		assert_eq!(moved, Vec3::X * 2.0);
	}

	#[test]
	fn aim_point_uses_visible_sliver_not_occluded_center() {
		let mut target = spotted(Entity::from_bits(1), Vec3::ZERO);
		target.visible = Vec3::X * 0.3;
		target.visible_head = None;
		assert_eq!(target.aim_point(0.0), Vec3::X * 0.3);
		assert_eq!(target.aim_point(1.0), Vec3::X * 0.3);
	}

	#[test]
	fn fresh_window_rejects_stale_observations() {
		let target = spotted(Entity::from_bits(1), Vec3::ZERO);
		assert!(target.is_fresh(0.1, 0.2));
		assert!(!target.is_fresh(1.0, 0.2));
	}

	#[test]
	fn rank_candidates_prefers_nearest_when_unfocused() {
		let a = Entity::from_bits(1);
		let b = Entity::from_bits(2);
		let candidates = [(a, Vec3::X * 6.0), (b, Vec3::X * 2.0)];
		assert_eq!(rank_candidates(Vec3::ZERO, &candidates, Some(a), 0.0), vec![1, 0]);
	}

	#[test]
	fn rank_candidates_keeps_engaged_when_focused() {
		let a = Entity::from_bits(1);
		let b = Entity::from_bits(2);
		let candidates = [(a, Vec3::X * 6.0), (b, Vec3::X * 2.0)];
		assert_eq!(rank_candidates(Vec3::ZERO, &candidates, Some(a), 1.0), vec![0, 1]);
	}

	#[test]
	fn allocate_vision_shares_evenly_when_unfocused() {
		assert_eq!(allocate_vision(9, 3, 0.0), vec![3, 3, 3]);
	}

	#[test]
	fn allocate_vision_spends_all_on_first_when_focused() {
		assert_eq!(allocate_vision(9, 3, 1.0), vec![9, 0, 0]);
	}

	#[test]
	fn allocate_vision_gives_remainders_to_higher_ranks() {
		assert_eq!(allocate_vision(10, 3, 0.0), vec![4, 3, 3]);
	}

	#[test]
	fn cascade_vision_moves_overflow_down_the_rank_list() {
		let mut rays = vec![12, 0, 0];
		cascade_vision(&mut rays, 9);
		assert_eq!(rays, vec![9, 3, 0]);
	}
}
