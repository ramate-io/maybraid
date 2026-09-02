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
	pub movement_vector: Vec3,
	/// [`Time::elapsed_secs`] when this observation was made.
	pub spotted_at: f32,
}

impl SpottedTarget {
	pub fn aim_point(self, headshots: f32) -> Vec3 {
		self.capsule
			.center_mass(self.position)
			.lerp(self.capsule.head(self.position), headshots.clamp(0.0, 1.0))
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

#[cfg(test)]
mod tests {
	use super::*;

	fn spotted(entity: Entity, position: Vec3) -> SpottedTarget {
		SpottedTarget {
			entity,
			position,
			capsule: TargetCapsule::new(0.4, 0.9),
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
	fn aim_point_blends_center_mass_and_head() -> anyhow::Result<()> {
		let target = spotted(Entity::from_bits(1), Vec3::ZERO);
		assert_eq!(target.aim_point(0.0), Vec3::ZERO);
		assert!(target.aim_point(1.0).y > 0.0);
		Ok(())
	}
}
