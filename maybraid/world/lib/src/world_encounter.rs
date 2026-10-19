//! World adapter: pause, session, and body-replace → encounter progress.
//!
//! Does not place the encounter in the world
//! ([#1117](https://github.com/ramate-io/maybraid/issues/1117)). Gameplay
//! awards do not require combat-hud. Does not own [`lubikans::LubikanSessionStore`].

use bevy::prelude::*;
use bikanwarda_encounter::BikanwardaEncounterPlugin;
use characters::{AnimClip, AnimRef, AnimRefRoot};
use damage::Died;
use encounters::{
	EncounterInstance, EncounterPhase, EncounterProgressStore, EncounterSignal,
	EncounterSignalKind, EncounterSystems, EncounterTimeEnabled,
};
use lubikans::{LubikanOwnerLifetime, LubikanOwnerLifetimeKind};
use world_player::Player;

use crate::control::{WorldGameplayEnabled, WorldPlaySessionEnded};

/// Wires Bikanwarda outcomes into the world plugin graph.
pub struct WorldEncounterPlugin;

impl Plugin for WorldEncounterPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<BikanwardaEncounterPlugin>() {
			app.add_plugins(BikanwardaEncounterPlugin);
		}
		configure(app);
	}
}

/// Headless-safe pause / session / body-replace forwarding.
pub fn configure(app: &mut App) {
	app.add_systems(
		PostUpdate,
		(
			sync_encounter_time_enabled.before(EncounterSystems::Ingest),
			forward_world_session_end.before(EncounterSystems::Resolve),
			forward_owner_lifetime.before(EncounterSystems::Resolve),
			forward_player_deaths.before(EncounterSystems::Resolve),
			drive_sleep_clips.after(EncounterSystems::Phase),
		),
	);
}

fn sync_encounter_time_enabled(
	gameplay: Res<WorldGameplayEnabled>,
	enabled: Option<ResMut<EncounterTimeEnabled>>,
) {
	let Some(mut enabled) = enabled else {
		return;
	};
	if enabled.0 != gameplay.0 {
		enabled.0 = gameplay.0;
	}
}

fn forward_world_session_end(
	mut ended: MessageReader<WorldPlaySessionEnded>,
	mut signals: MessageWriter<EncounterSignal>,
) {
	if ended.read().next().is_none() {
		return;
	}
	signals.write(EncounterSignal::session_reset());
}

fn forward_owner_lifetime(
	mut lives: MessageReader<LubikanOwnerLifetime>,
	mut signals: MessageWriter<EncounterSignal>,
) {
	for notice in lives.read() {
		let kind = match notice.kind {
			LubikanOwnerLifetimeKind::ReplaceBody => EncounterSignalKind::BodyReplaced,
			LubikanOwnerLifetimeKind::RestartSession => EncounterSignalKind::SessionReset,
			LubikanOwnerLifetimeKind::Equip | LubikanOwnerLifetimeKind::Unequip => continue,
		};
		signals.write(EncounterSignal::new(
			encounters::EncounterId::from_bytes([0; 32]),
			kind,
			notice.current,
		));
	}
}

fn forward_player_deaths(
	mut died: MessageReader<Died>,
	players: Query<(), With<Player>>,
	mut signals: MessageWriter<EncounterSignal>,
) {
	if !died.read().any(|death| players.contains(death.entity)) {
		return;
	}
	signals.write(EncounterSignal::new(
		encounters::EncounterId::from_bytes([0; 32]),
		EncounterSignalKind::PlayerDied,
		None,
	));
}

/// Map explicit sleep phase to [#1141](https://github.com/ramate-io/maybraid/pull/1141) clips.
/// Missing `AnimRefRoot` is fine — fixtures have no presentation.
pub(crate) fn sleep_clip(phase: EncounterPhase) -> Option<AnimClip> {
	match phase {
		EncounterPhase::Sleep(sleep) if sleep.elapsed_secs <= 0.0 => {
			Some(AnimClip::epiphant_sleep_enter())
		}
		EncounterPhase::Sleep(_) => Some(AnimClip::epiphant_sleep_hold()),
		EncounterPhase::Waking => Some(AnimClip::epiphant_wake()),
		_ => None,
	}
}

fn drive_sleep_clips(
	store: Res<EncounterProgressStore>,
	mut clips: Query<(Entity, &EncounterInstance, &mut AnimRefRoot)>,
) {
	for (entity, instance, mut root) in &mut clips {
		if store.get(instance.id).is_none_or(|record| record.instance != Some(entity)) {
			continue;
		}
		let Some(record) = store.get(instance.id) else {
			continue;
		};
		let Some(clip) = sleep_clip(record.phase) else {
			continue;
		};
		if root.0.clip.id() != clip.id() {
			root.0 = AnimRef::new(clip);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

	#[test]
	fn gameplay_off_freezes_encounter_time() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(WorldGameplayEnabled(false));
		world.insert_resource(EncounterTimeEnabled(true));
		world
			.run_system_once(sync_encounter_time_enabled)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.resource::<EncounterTimeEnabled>().0);
		Ok(())
	}

	#[test]
	fn session_end_emits_encounter_reset() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<WorldPlaySessionEnded>>();
		world.init_resource::<Messages<EncounterSignal>>();
		world.write_message(WorldPlaySessionEnded);
		world
			.run_system_once(forward_world_session_end)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let count = world
			.run_system_once(|mut signals: MessageReader<EncounterSignal>| signals.read().count())
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(count, 1);
		Ok(())
	}

	#[test]
	fn body_replace_is_not_a_session_reset() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<LubikanOwnerLifetime>>();
		world.init_resource::<Messages<EncounterSignal>>();
		world.write_message(LubikanOwnerLifetime::new(
			Some(Entity::from_bits(1)),
			Some(Entity::from_bits(2)),
			LubikanOwnerLifetimeKind::ReplaceBody,
		));
		world
			.run_system_once(forward_owner_lifetime)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let kind = world
			.run_system_once(|mut signals: MessageReader<EncounterSignal>| {
				signals.read().next().map(|signal| signal.kind)
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.ok_or_else(|| anyhow::anyhow!("signal"))?;
		assert_eq!(kind, EncounterSignalKind::BodyReplaced);
		Ok(())
	}

	#[test]
	fn sleep_phase_maps_to_epiphant_clips() {
		use encounters::EncounterSleep;
		assert_eq!(
			sleep_clip(EncounterPhase::Sleep(EncounterSleep::new(20.0))).map(|clip| clip.id()),
			Some(characters::AnimId::EpiphantSleepEnter)
		);
		assert_eq!(
			sleep_clip(EncounterPhase::Sleep(EncounterSleep {
				elapsed_secs: 1.0,
				health_at_enter: 20.0,
			}))
			.map(|clip| clip.id()),
			Some(characters::AnimId::EpiphantSleepHold)
		);
		assert_eq!(
			sleep_clip(EncounterPhase::Waking).map(|clip| clip.id()),
			Some(characters::AnimId::EpiphantWake)
		);
		assert_eq!(sleep_clip(EncounterPhase::Combat).map(|clip| clip.id()), None);
	}
}
