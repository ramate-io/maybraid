//! Flee actuator: get away from the ranked assailant as quickly as possible.

use bevy::prelude::*;
use evasion_intelligence::{EvasionIntelligenceUser, EvasionSystems};
use movement_intelligence::{
	MovementIntelligence, MovementIntelligenceSystems, MovementLocation, MovementObjective,
	ReplanMovement,
};

const REFRESH_DISTANCE: f32 = 0.8;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FleeingSettings {
	pub radius: f32,
}

impl Default for FleeingSettings {
	fn default() -> Self {
		Self { radius: 16.0 }
	}
}

/// Per-user flee policy. Writes [`MovementObjective::FleeFrom`] only while the
/// evasion signal is flee.
#[derive(Component, Clone, Debug)]
pub struct FleeingUser {
	pub settings: FleeingSettings,
	driving: bool,
}

impl FleeingUser {
	pub fn new(settings: FleeingSettings) -> Self {
		Self { settings, driving: false }
	}

	pub fn objective(&self, threat: Vec3) -> MovementObjective {
		MovementObjective::FleeFrom(MovementLocation::new(threat, self.settings.radius.max(0.5)))
	}
}

impl Default for FleeingUser {
	fn default() -> Self {
		Self::new(FleeingSettings::default())
	}
}

pub struct FleeingPlugin;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FleeingSystems {
	Write,
}

impl Plugin for FleeingPlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(
			Update,
			FleeingSystems::Write
				.after(EvasionSystems::Rank)
				.before(MovementIntelligenceSystems::Replan),
		)
		.add_systems(Update, write_flee_objectives.in_set(FleeingSystems::Write));
	}
}

pub fn write_flee_objectives(
	mut users: Query<(
		Entity,
		&Transform,
		&EvasionIntelligenceUser,
		&mut FleeingUser,
		&mut MovementIntelligence,
	)>,
	mut commands: Commands,
) {
	for (entity, transform, evasion, mut fleeing, mut movement) in &mut users {
		if !evasion.signal.is_flee() {
			let was_driving = fleeing.driving;
			fleeing.driving = false;
			if was_driving && evasion.signal.is_idle() {
				hold_in_place(entity, transform.translation, &mut movement, &mut commands);
			}
			continue;
		}
		let Some(contact) = evasion.best_contact() else {
			if fleeing.driving {
				fleeing.driving = false;
				hold_in_place(entity, transform.translation, &mut movement, &mut commands);
			}
			continue;
		};
		let next = fleeing.objective(contact.position);
		fleeing.driving = true;
		if !should_replan(movement.objective, next) {
			continue;
		}
		movement.objective = next;
		commands.entity(entity).insert(ReplanMovement);
	}
}

fn hold_in_place(
	entity: Entity,
	at: Vec3,
	movement: &mut MovementIntelligence,
	commands: &mut Commands,
) {
	movement.objective =
		MovementObjective::Reach(MovementLocation::new(at, movement.ability.agent_radius));
	movement.adopt_plan(Vec::new());
	commands.entity(entity).remove::<ReplanMovement>();
}

fn should_replan(current: MovementObjective, next: MovementObjective) -> bool {
	if std::mem::discriminant(&current) != std::mem::discriminant(&next) {
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
	use evasion_intelligence::{AssailantContact, AssailantSource};

	#[test]
	fn flee_writes_a_disk_around_the_threat_snapshot() -> anyhow::Result<()> {
		let user = FleeingUser::new(FleeingSettings { radius: 12.0 });
		let objective = user.objective(Vec3::X * 4.0);
		assert!(matches!(objective, MovementObjective::FleeFrom(_)));
		assert!((objective.location().radius - 12.0).abs() < 1e-4);
		assert_eq!(objective.location().point, Vec3::X * 4.0);
		Ok(())
	}

	#[test]
	fn idle_signal_does_not_drive() -> anyhow::Result<()> {
		let threat = Entity::from_bits(1);
		let mut evasion = EvasionIntelligenceUser::default();
		evasion.include(threat, AssailantSource::ENEMYSHIP);
		assert!(!evasion.signal.is_flee());
		let _ = AssailantContact {
			subject: threat,
			position: Vec3::ZERO,
			movement_vector: Vec3::ZERO,
			last_known_at: 0.0,
		};
		Ok(())
	}
}
