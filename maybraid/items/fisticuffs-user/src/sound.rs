//! Flinch grunts on [`FisticuffsStrike`] (active window start).
//!
//! Jabs reuse [`FlinchSounds::play`] and the authored damage clips — the same
//! path characters already use when they take a hit.

use bevy::prelude::*;
use maybraid_audio::{
	Audio, AudioBus, AudioClip, AudioPlugin, AudioSystems, FlinchProfile, FlinchSounds,
	FlinchState, Mixer, SpatialOneShot,
};

use crate::{FisticuffsImpact, FisticuffsStrike, FisticuffsUser};

pub const FISTICUFFS_IMPACT_SOUND: &str = "sound-effects/weapons/fisticuffs/impact_001.wav";

#[derive(Resource)]
pub(crate) struct FisticuffsImpactSounds {
	impact: Handle<AudioClip>,
}

impl FromWorld for FisticuffsImpactSounds {
	fn from_world(world: &mut World) -> Self {
		Self { impact: world.resource::<AssetServer>().load(FISTICUFFS_IMPACT_SOUND) }
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct ImpactAudioRequest {
	point: Vec3,
	bus: AudioBus,
}

fn impact_audio_request(impact: &FisticuffsImpact) -> ImpactAudioRequest {
	ImpactAudioRequest { point: impact.point, bus: AudioBus::Impacts }
}

pub(crate) fn ensure_strike_audio_state(
	mut commands: Commands,
	users: Query<Entity, (With<FisticuffsUser>, Without<FlinchState>)>,
) {
	for entity in &users {
		let seed = entity.to_bits();
		if let Ok(mut entity) = commands.get_entity(entity) {
			entity.try_insert(FlinchState::seeded(seed));
		}
	}
}

pub(crate) fn play_strike_grunts(
	mut commands: Commands,
	mut strikes: MessageReader<FisticuffsStrike>,
	mut attackers: Query<
		(&GlobalTransform, Option<&FlinchProfile>, Option<&mut FlinchState>),
		With<FisticuffsUser>,
	>,
	clips: Res<Assets<AudioClip>>,
	audio: Option<Res<Audio>>,
	sounds: Option<Res<FlinchSounds>>,
	mixer: Option<ResMut<Mixer>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
) {
	let (Some(audio), Some(sounds), Some(mut mixer), Some(listener)) =
		(audio.as_deref(), sounds.as_deref(), mixer, listeners.iter().next())
	else {
		return;
	};
	for strike in strikes.read() {
		let Ok((transform, profile, audio_state)) = attackers.get_mut(strike.attacker) else {
			continue;
		};
		let point = transform.translation();
		let profile = profile.copied().unwrap_or_default();
		let mut local = audio_state
			.as_deref()
			.copied()
			.unwrap_or_else(|| FlinchState::seeded(strike.attacker.to_bits()));
		sounds.play(&mut commands, profile, &mut local, &clips, audio, &mut mixer, listener, point);
		if let Some(mut slot) = audio_state {
			*slot = local;
		} else if let Ok(mut entity) = commands.get_entity(strike.attacker) {
			entity.try_insert(local);
		}
	}
}

pub(crate) fn play_impact_sounds(
	mut commands: Commands,
	mut impacts: MessageReader<FisticuffsImpact>,
	sounds: Option<Res<FisticuffsImpactSounds>>,
	clips: Res<Assets<AudioClip>>,
	audio: Option<Res<Audio>>,
	mixer: Option<ResMut<Mixer>>,
	listeners: Query<&GlobalTransform, With<SpatialListener>>,
) {
	let (Some(sounds), Some(audio), Some(mut mixer), Some(listener)) =
		(sounds, audio, mixer, listeners.iter().next())
	else {
		return;
	};
	for impact in impacts.read() {
		let request = impact_audio_request(impact);
		audio.play_or_queue(
			&mut commands,
			&sounds.impact,
			&clips,
			SpatialOneShot::at(request.point).bus(request.bus),
			&mut mixer,
			listener,
			"fisticuffs-impact",
		);
	}
}

pub(crate) fn configure_strike_audio(app: &mut App) {
	if !app.is_plugin_added::<AudioPlugin>() {
		app.add_plugins(AudioPlugin);
	}
	app.init_resource::<FisticuffsImpactSounds>()
		.add_systems(
			PostUpdate,
			(ensure_strike_audio_state, play_strike_grunts)
				.chain()
				.before(AudioSystems::Sync),
		)
		.add_systems(
			PostUpdate,
			play_impact_sounds
				.after(crate::sensor::resolve_fist_hits)
				.before(AudioSystems::Sync),
		);
}

#[cfg(test)]
mod tests {
	use super::*;
	use maybraid_audio::decode_wav_mono;
	use std::path::Path;

	fn silent_clip() -> AudioClip {
		let bytes = std::fs::read(
			Path::new(env!("CARGO_MANIFEST_DIR"))
				.join("../../assets/sound-effects/character/damage/man_001.wav"),
		)
		.expect("authored damage grunt wav");
		AudioClip { frames: decode_wav_mono(&bytes).expect("mono wav") }
	}

	fn strike_grunt_count(world: &mut World) -> usize {
		world
			.query::<&Name>()
			.iter(world)
			.filter(|name| name.as_str() == "flinch-grunt")
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
		app.world_mut()
			.spawn((SpatialListener::new(0.4), GlobalTransform::from_translation(Vec3::ZERO)));
		let handle = app.world_mut().resource_mut::<Assets<AudioClip>>().add(silent_clip());
		app.world_mut().insert_resource(FlinchSounds::with_uniform_clip(handle));
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
	fn landing_audio_request_uses_impact_point_and_punch_clip() -> anyhow::Result<()> {
		let impact = FisticuffsImpact {
			attacker: Entity::from_bits(1),
			side: character_rigs::Side::Left,
			collider: Entity::from_bits(2),
			target: None,
			point: Vec3::new(1.0, 2.0, 3.0),
			normal: Vec3::Y,
		};
		let request = impact_audio_request(&impact);
		assert_eq!(request.point, impact.point);
		assert_eq!(request.bus, AudioBus::Impacts);
		assert_eq!(FISTICUFFS_IMPACT_SOUND, "sound-effects/weapons/fisticuffs/impact_001.wav");
		let wav_path = Path::new(env!("CARGO_MANIFEST_DIR"))
			.join("../../assets/")
			.join(FISTICUFFS_IMPACT_SOUND);
		let bytes = std::fs::read(wav_path)?;
		let frames = decode_wav_mono(&bytes)?;
		assert!(!frames.is_empty());
		Ok(())
	}
}
