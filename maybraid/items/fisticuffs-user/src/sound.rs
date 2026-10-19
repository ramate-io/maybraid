//! Effort grunts on [`FisticuffsStrike`] (active window start).

use bevy::prelude::*;
use maybraid_audio::{
	Audio, AudioClip, AudioPlugin, AudioSystems, FisticuffsStrikeAudioState, FisticuffsStrikeSounds,
	Mixer,
};

use crate::{FisticuffsStrike, FisticuffsUser};

pub(crate) fn ensure_strike_audio_state(
	mut commands: Commands,
	users: Query<Entity, (With<FisticuffsUser>, Without<FisticuffsStrikeAudioState>)>,
) {
	for entity in &users {
		commands.entity(entity).insert(FisticuffsStrikeAudioState::seeded(entity.to_bits()));
	}
}

pub(crate) fn play_strike_grunts(
	mut commands: Commands,
	mut strikes: MessageReader<FisticuffsStrike>,
	mut attackers: Query<
		(&GlobalTransform, Option<&mut FisticuffsStrikeAudioState>),
		With<FisticuffsUser>,
	>,
	clips: Res<Assets<AudioClip>>,
	audio: Option<Res<Audio>>,
	sounds: Option<Res<FisticuffsStrikeSounds>>,
	mixer: Option<ResMut<Mixer>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
) {
	let (Some(audio), Some(sounds), Some(mut mixer), Some(listener)) =
		(audio.as_deref(), sounds.as_deref(), mixer, listeners.iter().next())
	else {
		return;
	};
	for strike in strikes.read() {
		let Ok((transform, audio_state)) = attackers.get_mut(strike.attacker) else {
			continue;
		};
		let point = transform.translation();
		let mut local = audio_state
			.as_deref()
			.copied()
			.unwrap_or_else(|| FisticuffsStrikeAudioState::seeded(strike.attacker.to_bits()));
		sounds.play_strike(
			&mut commands,
			strike.combo_step,
			&mut local,
			&clips,
			audio,
			&mut mixer,
			listener,
			point,
		);
		if let Some(mut slot) = audio_state {
			*slot = local;
		} else if let Ok(mut entity) = commands.get_entity(strike.attacker) {
			entity.try_insert(local);
		}
	}
}

pub(crate) fn configure_strike_audio(app: &mut App) {
	if !app.is_plugin_added::<AudioPlugin>() {
		app.add_plugins(AudioPlugin);
	}
	app.add_systems(
		PostUpdate,
		(ensure_strike_audio_state, play_strike_grunts)
			.chain()
			.before(AudioSystems::Sync),
	);
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_audio::{decode_wav_mono, EFFORT_VARIANT_COUNT};
	use std::path::Path;

	fn silent_clip() -> AudioClip {
		let bytes = std::fs::read(
			Path::new(env!("CARGO_MANIFEST_DIR"))
				.join("../../assets/sound-effects/character/fisticuffs/effort_man_1_001.wav"),
		)
		.expect("placeholder effort wav");
		AudioClip { frames: decode_wav_mono(&bytes).expect("mono wav") }
	}

	fn strike_grunt_count(world: &mut World) -> usize {
		world
			.query::<&Name>()
			.iter(world)
			.filter(|name| name.as_str() == "fisticuffs-strike-grunt")
			.count()
	}

	fn test_app_with_audio() -> Option<App> {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, AssetPlugin::default()));
		app.add_plugins(AudioPlugin);
		app.update();
		if app.world().get_resource::<Audio>().is_none() {
			return None;
		}
		app.world_mut().spawn((
			SpatialListener::new(0.4),
			GlobalTransform::from_translation(Vec3::ZERO),
		));
		let handle = app.world_mut().resource_mut::<Assets<AudioClip>>().add(silent_clip());
		app.world_mut().insert_resource(FisticuffsStrikeSounds::with_uniform_clip(handle));
		Some(app)
	}

	#[test]
	fn strike_event_plays_exactly_one_grunt() {
		let Some(mut app) = test_app_with_audio() else {
			return;
		};
		app.add_message::<FisticuffsStrike>();
		configure_strike_audio(&mut app);
		let attacker = app
			.world_mut()
			.spawn((FisticuffsUser::default(), GlobalTransform::IDENTITY))
			.id();
		app.world_mut().write_message(FisticuffsStrike {
			attacker,
			side: character_rigs::Side::Left,
			kind: crate::AttackKind::Jab,
			combo_step: 0,
			progress: 0.32,
			target: Vec3::Z,
		});
		app.update();
		assert_eq!(strike_grunt_count(app.world_mut()), 1);
	}

	#[test]
	fn strike_without_fisticuffs_user_is_silent() {
		let Some(mut app) = test_app_with_audio() else {
			return;
		};
		app.add_message::<FisticuffsStrike>();
		configure_strike_audio(&mut app);
		let attacker = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
		app.world_mut().write_message(FisticuffsStrike {
			attacker,
			side: character_rigs::Side::Right,
			kind: crate::AttackKind::Jab,
			combo_step: 1,
			progress: 0.32,
			target: Vec3::Z,
		});
		app.update();
		assert_eq!(strike_grunt_count(app.world_mut()), 0);
	}

	#[test]
	fn variant_selection_is_deterministic_for_a_seed() {
		let mut a = FisticuffsStrikeAudioState::seeded(77);
		let mut b = FisticuffsStrikeAudioState::seeded(77);
		for _ in 0..EFFORT_VARIANT_COUNT * 2 {
			assert_eq!(a.pick_variant(), b.pick_variant());
			assert!((a.pitch_jitter() - b.pitch_jitter()).abs() < 1e-6);
		}
	}
}
