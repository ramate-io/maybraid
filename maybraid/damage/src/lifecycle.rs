//! One-shot downing and component-backed deferred despawn.

use bevy::prelude::*;

use crate::Died;

/// A damageable entity whose health crossed zero.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Downed {
	pub source: Option<Entity>,
	pub point: Vec3,
	pub at: f32,
}

/// Queue an entity hierarchy for despawn after a game-time delay.
#[derive(Component, Clone, Debug)]
pub struct DespawnAfter(Timer);

impl DespawnAfter {
	pub fn seconds(seconds: f32) -> Self {
		Self(Timer::from_seconds(seconds.max(0.0), TimerMode::Once))
	}

	pub fn remaining_secs(&self) -> f32 {
		self.0.remaining_secs()
	}
}

/// Materialize [`Died`] messages as durable, queryable state.
pub fn mark_downed(
	time: Res<Time>,
	mut died: MessageReader<Died>,
	mut commands: Commands,
	already_downed: Query<(), With<Downed>>,
) {
	let now = time.elapsed_secs();
	for death in died.read() {
		if already_downed.contains(death.entity) {
			continue;
		}
		commands.entity(death.entity).try_insert(Downed {
			source: death.source,
			point: death.point,
			at: now,
		});
	}
}

/// Drain queued despawns in [`Last`] so the hierarchy remains readable for the frame.
pub fn tick_queued_despawns(
	time: Res<Time>,
	mut commands: Commands,
	mut queued: Query<(Entity, &mut DespawnAfter)>,
) {
	for (entity, mut despawn) in &mut queued {
		despawn.0.tick(time.delta());
		if despawn.0.is_finished() {
			commands.entity(entity).try_despawn();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn despawn_delay_clamps_at_zero() {
		let queued = DespawnAfter::seconds(-1.0);
		assert_eq!(queued.remaining_secs(), 0.0);
	}

	#[test]
	fn death_becomes_durable_downed_state() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<Died>()
			.add_systems(Update, mark_downed);
		let entity = app.world_mut().spawn_empty().id();
		let point = Vec3::new(1.0, 2.0, 3.0);
		app.world_mut().write_message(Died { entity, source: None, point });

		app.update();

		let downed = app.world().get::<Downed>(entity);
		assert!(downed.is_some_and(|downed| downed.point == point));
	}

	#[test]
	fn zero_delay_despawns_on_the_next_drain() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins).add_systems(Update, tick_queued_despawns);
		let entity = app.world_mut().spawn(DespawnAfter::seconds(0.0)).id();

		app.update();

		assert!(!app.world().entities().contains(entity));
	}
}
