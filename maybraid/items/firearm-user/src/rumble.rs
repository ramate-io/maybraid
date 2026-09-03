//! Pad rumble for the followed player's fire and landed hits.

use std::time::Duration;

use bevy::prelude::*;
use damage::DamageApplied;
use firearms::WeaponFired;
use maybraid_input::PadRumble;
use player::CameraFollow;

/// Catalog recoil 4 → `WeaponFired.recoil` ≈ 0.08 rad (`RECOIL_PITCH_PER_UNIT`).
const FIRE_RECOIL_FULL: f32 = 0.08;
const FIRE_MS: u64 = 120;
const FIRE_WEAK: f32 = 0.4;
const FIRE_STRONG: f32 = 0.35;
const FIRE_WEAK_KICK: f32 = 0.3;
const FIRE_STRONG_KICK: f32 = 0.4;
const HIT_MS: u64 = 200;
const HIT_WEAK: f32 = 0.65;
const HIT_STRONG: f32 = 0.9;

pub(crate) fn fire_rumble(recoil: f32) -> PadRumble {
	let kick = (recoil / FIRE_RECOIL_FULL).clamp(0.0, 1.0);
	PadRumble::motors(
		Duration::from_millis(FIRE_MS),
		FIRE_WEAK + FIRE_WEAK_KICK * kick,
		FIRE_STRONG + FIRE_STRONG_KICK * kick,
	)
}

pub(crate) fn hit_rumble() -> PadRumble {
	PadRumble::motors(Duration::from_millis(HIT_MS), HIT_WEAK, HIT_STRONG)
}

pub(crate) fn pulse_combat_rumble(
	mut fired: MessageReader<WeaponFired>,
	mut hits: MessageReader<DamageApplied>,
	followed: Query<Entity, With<CameraFollow>>,
	mut rumble: MessageWriter<PadRumble>,
) {
	let Ok(player) = followed.single() else {
		return;
	};
	let mut fire = false;
	let mut recoil: f32 = 0.0;
	for event in fired.read() {
		if event.shooter != player {
			continue;
		}
		fire = true;
		recoil = recoil.max(event.recoil);
	}
	let hit = hits.read().any(|applied| applied.source == Some(player));
	if fire {
		info!("pad_rumble: fire recoil={recoil:.3}");
		rumble.write(fire_rumble(recoil));
	}
	if hit {
		info!("pad_rumble: hit confirm");
		rumble.write(hit_rumble());
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn fire_pulse_grows_with_recoil() {
		let none = fire_rumble(0.0);
		let full = fire_rumble(FIRE_RECOIL_FULL);
		assert_eq!(none.duration, Duration::from_millis(FIRE_MS));
		assert!((none.intensity.weak_motor - FIRE_WEAK).abs() < 1e-5);
		assert!((full.intensity.weak_motor - (FIRE_WEAK + FIRE_WEAK_KICK)).abs() < 1e-5);
		assert!(full.intensity.weak_motor > none.intensity.weak_motor);
	}

	#[test]
	fn hit_pulse_is_heavier_than_a_recoilless_shot() {
		let fire = fire_rumble(0.0);
		let hit = hit_rumble();
		assert!(hit.duration > fire.duration);
		assert!(hit.intensity.strong_motor > fire.intensity.strong_motor);
	}

	fn collect_pad_rumble(app: &mut App) -> Vec<PadRumble> {
		let messages = app.world().resource::<Messages<PadRumble>>();
		let mut cursor = messages.get_cursor();
		cursor.read(messages).copied().collect()
	}

	fn combat_app() -> (App, Entity) {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_message::<WeaponFired>()
			.add_message::<DamageApplied>()
			.add_message::<PadRumble>()
			.add_systems(Update, pulse_combat_rumble);
		let player = app.world_mut().spawn(CameraFollow).id();
		(app, player)
	}

	#[test]
	fn followed_player_fire_and_hit_rumble() {
		let (mut app, player) = combat_app();
		let target = app.world_mut().spawn_empty().id();
		app.world_mut().write_message(WeaponFired { shooter: player, recoil: 0.04 });
		app.world_mut().write_message(DamageApplied {
			target,
			source: Some(player),
			amount: 25.0,
			remaining: 75.0,
			point: Vec3::ZERO,
		});
		app.update();
		let pulses = collect_pad_rumble(&mut app);
		assert_eq!(pulses.len(), 2);
		assert_eq!(pulses[0], fire_rumble(0.04));
		assert_eq!(pulses[1], hit_rumble());
	}

	#[test]
	fn npc_shots_do_not_rumble() {
		let (mut app, player) = combat_app();
		let npc = app.world_mut().spawn_empty().id();
		app.world_mut().write_message(WeaponFired { shooter: npc, recoil: 0.08 });
		app.world_mut().write_message(DamageApplied {
			target: player,
			source: Some(npc),
			amount: 25.0,
			remaining: 75.0,
			point: Vec3::ZERO,
		});
		app.update();
		assert!(collect_pad_rumble(&mut app).is_empty());
	}
}
