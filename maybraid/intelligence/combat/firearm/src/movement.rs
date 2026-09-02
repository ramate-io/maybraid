//! Firearm movement brain: where to stand relative to combat targets.

use bevy::prelude::*;
use movement_intelligence::{
	MovementIntelligence, MovementLocation, MovementObjective, ReplanMovement,
};

use crate::target::{pick_target, FirearmMovementObjective};

const REFRESH_DISTANCE: f32 = 1.35;

/// How a firearm combatant stands relative to its targets.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FirearmMovementIntelligenceSettings {
	/// `(preferred_distance, strength)`. Distance is the vantage / edge radius.
	/// Strength scales how much that standoff matters versus other hints.
	pub range: (f32, f32),
	/// How strongly to seek cover (scales hide on [`MovementObjective::VantageOn`]).
	pub cover: f32,
	/// `(trigger_distance, flee_radius)`. Inside the trigger, write [`MovementObjective::FleeFrom`].
	pub flee: (f32, f32),
	/// `(hide, sightline)` weights written onto [`MovementObjective::VantageOn`].
	pub vantage: (f32, f32),
}

impl Default for FirearmMovementIntelligenceSettings {
	fn default() -> Self {
		Self { range: (1.4, 1.0), cover: 1.0, flee: (1.2, 8.0), vantage: (10.0, 14.0) }
	}
}

/// Per-user firearm movement install. Fields [`FirearmMovementObjective`] and
/// writes [`MovementIntelligence::objective`].
#[derive(Component, Debug, Clone)]
pub struct FirearmMovementIntelligence {
	pub objective: FirearmMovementObjective,
	pub settings: FirearmMovementIntelligenceSettings,
}

impl FirearmMovementIntelligence {
	pub fn new(objective: FirearmMovementObjective) -> Self {
		Self { objective, settings: FirearmMovementIntelligenceSettings::default() }
	}

	pub fn compose(&self, from: Vec3, target: Vec3) -> MovementObjective {
		let dist = Vec2::new(from.x, from.z).distance(Vec2::new(target.x, target.z));
		let (flee_at, flee_radius) = self.settings.flee;
		if flee_at > 0.0 && dist < flee_at {
			return MovementObjective::FleeFrom(MovementLocation::new(
				target,
				flee_radius.max(0.5),
			));
		}
		let (standoff, strength) = self.settings.range;
		let radius = standoff.max(0.4);
		let hide = self.settings.vantage.0 * self.settings.cover.max(0.0) * strength.max(0.0);
		let sightline = self.settings.vantage.1 * strength.max(0.0);
		MovementObjective::VantageOn {
			location: MovementLocation::new(target, radius),
			hide_weight: hide,
			sightline_weight: sightline,
		}
	}
}

pub(crate) fn write_firearm_movement_objectives(
	mut combatants: Query<(Entity, &FirearmMovementIntelligence, &mut MovementIntelligence)>,
	transforms: Query<&Transform>,
	mut commands: Commands,
) {
	for (entity, brain, mut movement) in &mut combatants {
		let Ok(from_tf) = transforms.get(entity) else {
			continue;
		};
		let Some(target) = pick_target(
			from_tf.translation,
			&brain.objective.0,
			|target| transforms.get(target).ok().map(|tf| tf.translation),
			None,
			0.0,
		) else {
			continue;
		};
		let Ok(target_tf) = transforms.get(target) else {
			continue;
		};
		let next = brain.compose(from_tf.translation, target_tf.translation);
		if !should_replan(movement.objective, next) {
			continue;
		}
		movement.objective = next;
		commands.entity(entity).insert(ReplanMovement);
	}
}

fn should_replan(current: MovementObjective, next: MovementObjective) -> bool {
	if std::mem::discriminant(&current) != std::mem::discriminant(&next) {
		return true;
	}
	if (current.hide_weight() - next.hide_weight()).abs() > 0.05
		|| (current.sightline_weight() - next.sightline_weight()).abs() > 0.05
	{
		return true;
	}
	let a = current.location().point;
	let b = next.location().point;
	Vec2::new(a.x, a.z).distance(Vec2::new(b.x, b.z)) >= REFRESH_DISTANCE
		|| (a.y - b.y).abs() >= REFRESH_DISTANCE
		|| (current.location().radius - next.location().radius).abs() > 0.05
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn compose_vantage_when_outside_flee_range() -> anyhow::Result<()> {
		let brain = FirearmMovementIntelligence::new(FirearmMovementObjective::default());
		let objective = brain.compose(Vec3::ZERO, Vec3::X * 6.0);
		assert!(objective.is_vantage_on());
		assert!((objective.location().radius - 1.4).abs() < 1e-4);
		assert!((objective.hide_weight() - 10.0).abs() < 1e-4);
		assert!((objective.sightline_weight() - 14.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn compose_flee_when_inside_trigger_distance() -> anyhow::Result<()> {
		let brain = FirearmMovementIntelligence::new(FirearmMovementObjective::default());
		let objective = brain.compose(Vec3::ZERO, Vec3::X * 0.5);
		assert!(matches!(objective, MovementObjective::FleeFrom(_)));
		assert!((objective.location().radius - 8.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn cover_scales_hide_but_not_sightline() -> anyhow::Result<()> {
		let mut brain = FirearmMovementIntelligence::new(FirearmMovementObjective::default());
		brain.settings.cover = 0.5;
		let objective = brain.compose(Vec3::ZERO, Vec3::X * 6.0);
		assert!((objective.hide_weight() - 5.0).abs() < 1e-4);
		assert!((objective.sightline_weight() - 14.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn should_replan_when_the_watch_point_moves() {
		let a = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::ZERO, 1.4),
			hide_weight: 10.0,
			sightline_weight: 14.0,
		};
		let near = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::X * 0.2, 1.4),
			hide_weight: 10.0,
			sightline_weight: 14.0,
		};
		let far = MovementObjective::VantageOn {
			location: MovementLocation::new(Vec3::X * 2.0, 1.4),
			hide_weight: 10.0,
			sightline_weight: 14.0,
		};
		assert!(!should_replan(a, near));
		assert!(should_replan(a, far));
	}
}
